use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;

use solana_sdk::{
    instruction::Instruction, message::VersionedMessage, pubkey::Pubkey, signature::Keypair, system_instruction, transaction::VersionedTransaction, signer::Signer,
};
use spl_token::{
    instruction::{close_account, sync_native},
    native_mint,
};
use std::sync::Arc;
use tracing::info;

use crate::{executor::legacy::{send_and_confirm_transaction_legacy, TransactionType}, utils::instruction::get_or_create_ata_instruction};

pub async fn obfuscate(
    connection: Arc<RpcClient>,
    payer: Arc<Keypair>,
    amount: u64,
) -> Result<VersionedTransaction> {
    info!("Calling obfuscate...");
    
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

    send_and_confirm_transaction_legacy(&connection, &TransactionType::Versioned(transaction.clone())).await?;

    Ok(transaction)
}
