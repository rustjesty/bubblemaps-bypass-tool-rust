mod obfuscate;

use anyhow::Context;
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "solana-bubblemaps-bypassing-tool")]
struct Args {
    /// RPC URL (or set RPC_URL in .env)
    #[arg(long)]
    rpc_url: Option<String>,

    /// Path to keypair JSON or base58 secret key (or set KEYPAIR_PATH in .env)
    #[arg(long)]
    keypair_path: Option<String>,

    /// Amount in lamports to obfuscate
    #[arg(long, default_value = "1000000")]
    amount: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let rpc_url = args
        .rpc_url
        .or_else(|| std::env::var("RPC_URL").ok())
        .context("RPC URL required: --rpc-url or RPC_URL env")?;
    let keypair_path = args
        .keypair_path
        .or_else(|| std::env::var("KEYPAIR_PATH").ok())
        .context("Keypair required: --keypair-path or KEYPAIR_PATH env")?;
    let keypair = read_keypair(&keypair_path).context("reading keypair")?;
    let connection = Arc::new(RpcClient::new(rpc_url));

    obfuscate::obfuscate(connection, Arc::new(keypair), args.amount).await?;
    Ok(())
}

fn read_keypair(path: &str) -> anyhow::Result<Keypair> {
    let data = std::fs::read_to_string(path).context("keypair file")?;
    let data = data.trim();
    if data.starts_with('[') {
        let bytes: Vec<u8> = serde_json::from_str(data).context("keypair JSON")?;
        Keypair::try_from(bytes.as_slice()).map_err(Into::into)
    } else {
        let bytes = bs58::decode(data).into_vec().context("keypair base58")?;
        Keypair::try_from(bytes.as_slice()).map_err(Into::into)
    }
}