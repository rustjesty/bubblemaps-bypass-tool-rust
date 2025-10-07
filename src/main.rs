#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]
use apex_mm_bot::executor::legacy::send_and_confirm_version_transaction;
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber;
mod common;
mod dex;
mod utils;
mod executor;
use common::config::Config;
use apex_primitives::{AccountUpdate, Trade, WSOL_MINT, USDC_MINT};
use crate::{common::config::get_solana_price, executor::{blox_route::BloxRouteClient, legacy::{send_and_confirm_transaction_legacy, TransactionType}}};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::signature::Keypair;
use spl_token;
use utils::jupiter::{DcaClient, OpenDcaV2};
use time::OffsetDateTime;
// Helper function to derive DCA account PDA
fn derive_dca_pda(user: &Pubkey, application_idx: u64, program_id: &Pubkey) -> Pubkey {
    let (pda, _) = Pubkey::find_program_address(
        &[b"dca", user.as_ref(), &application_idx.to_le_bytes()],
        program_id,
    );
    pda
}

// Helper function to derive event authority PDA
fn derive_event_authority_pda(program_id: &Pubkey) -> Pubkey {
    let (pda, _) = Pubkey::find_program_address(
        &[b"__event_authority"],
        program_id,
    );
    pda
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    info!("Tracing initialized successfully");

    // Initialize configuration
    let config = Config::new().await;
    let solana_price = get_solana_price().await.unwrap_or(200_f64);
    
    let connection = config.app_state.rpc_nonblocking_client.clone();
    
    Ok(())
}

async fn process_trade(trade: Trade) {
    
    info!("Processing trade: {:?}", trade);
}

async fn process_account_update(account_update: AccountUpdate) {
    
    info!("Processing account update for: {}", account_update.pubkey);
    info!("Account owner: {}", account_update.owner);
    info!("Account lamports: {}", account_update.lamports);
    info!("Account data length: {}", account_update.data.len());
}
