use anyhow::Context;

use crate::state::AppState;

pub struct AppConfig {
    pub pending_json: String,
    pub groups_yaml: String,
    pub buyers_csv: String,
    pub wallet: String,
    pub mint: String,
    pub database_url: String,
    pub client_url: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let pending_json =
            std::env::var("PENDING_JSON").unwrap_or_else(|_| "../pending_ops.json".to_string());

        let groups_yaml = std::env::var("GROUPS_YAML").context("GROUPS_YAML must be set")?;

        let buyers_csv = std::env::var("BUYERS_CSV").context("BUYERS_CSV must be set")?;

        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

        let client_url = std::env::var("CLIENT_URL").context("CLIENT_URL must be set")?;

        let wallet = std::env::var("MAIN_WALLET").context("MAIN_WALLET must be set")?;

        let mint = std::env::var("MINT_PUBKEY").context("MINT_PUBKEY must be set")?;

        Ok(Self {
            pending_json,
            groups_yaml,
            buyers_csv,
            wallet,
            mint,
            database_url,
            client_url,
        })
    }

    pub async fn create_app_state(&self) -> anyhow::Result<AppState> {
        AppState::new(
            &self.database_url,
            &self.client_url,
            &self.wallet,
            &self.mint,
            &self.pending_json,
        )
        .await
        .context("Failed to initialize AppState")
    }
}
