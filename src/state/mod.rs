mod db;

mod spl_token_context;

use std::str::FromStr;

pub use db::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
pub use spl_token_context::*;

use anyhow::{Context, Result};

use tokio::io::AsyncWriteExt;

pub async fn save_to_env(key: &str, value: &str) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".env")
        .await
        .context("Failed to open .env file")?;
    let line = format!("{}={}\n", key, value);
    file.write_all(line.as_bytes())
        .await
        .context("Failed to write to .env file")?;
    Ok(())
}

pub struct AppState {
    pub spl_token_context: SplTokenContext,
    pub db: DbContext,
}
impl AppState {
    pub async fn new(database_url: &str, client_url: impl ToString) -> Result<Self> {
        let client =
            RpcClient::new_with_commitment(client_url.to_string(), CommitmentConfig::confirmed());

        let main_wallet = if let Ok(wallet) = std::env::var("MAIN_WALLET") {
            Keypair::from_base58_string(&wallet)
        } else {
            let wallet = SplTokenContext::generate_wallet(&client).await?;
            log::info!("Generated wallet: {}", wallet.pubkey());
            let wallet_str = wallet.to_base58_string();
            let _ = save_to_env("MAIN_WALLET", &wallet_str).await.map_err(|_| {
                log::error!("Failed to save wallet to .env");
            });
            wallet
        };

        log::info!("Main wallet: {}", main_wallet.pubkey());

        let mint = if let Ok(mint) = std::env::var("MINT_PUBKEY") {
            Pubkey::from_str(&mint).map_err(|_| {
                log::error!("Failed to parse mint pubkey");
                anyhow::anyhow!("Failed to parse mint pubkey")
            })?
        } else {
            let mint = SplTokenContext::create_mint(&client, &main_wallet).await?;
            let mint_str = mint.to_string();
            let _ = save_to_env("MINT_PUBKEY", &mint_str).await.map_err(|_| {
                log::error!("Failed to save mint pubkey to .env");
            });
            mint
        };
        log::info!("Mint pubkey: {}", mint);

        let token_account = SplTokenContext::get_or_create_associated_token_account(
            &client,
            &main_wallet.pubkey(),
            &main_wallet,
            &mint,
        )
        .await?;
        log::info!("Token account: {}", token_account);
        // let amount = 10_000_000 * LAMPORTS_PER_SOL;
        let spl_token_context =
            SplTokenContext::new(client, main_wallet, mint, token_account).await?;

        let db = DbContext::new(database_url).await?;
        log::info!("Database initialized successfully!");

        Ok(AppState {
            spl_token_context,
            db,
        })
    }
}
