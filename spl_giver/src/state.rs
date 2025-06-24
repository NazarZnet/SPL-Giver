use anyhow::{Context, Result};
use common::{Buyer, Database, Group, SplToken};

pub struct AppState {
    pub spl_token: SplToken,
    pub db: Database,
}
impl AppState {
    pub async fn new(
        database_url: &str,
        client_url: &str,
        wallet: &str,
        mint: &str,
    ) -> Result<Self> {
        let spl_token_context = SplToken::new(client_url, wallet, mint).await?;

        let db = Database::new(database_url).await?;
        log::info!("Database initialized successfully!");

        Ok(AppState {
            spl_token: spl_token_context,
            db,
        })
    }
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let client_url = std::env::var("CLIENT_URL").context("CLIENT_URL must be set")?;
        let wallet = std::env::var("MAIN_WALLET").context("MAIN_WALLET must be set")?;
        let mint = std::env::var("MINT_PUBKEY").context("MINT_PUBKEY must be set")?;

        let state = AppState::new(&database_url, &client_url, &wallet, &mint)
            .await
            .context("Failed to initialize AppState")?;

        Ok(state)
    }

    pub async fn initialize_data_from_files(
        &self,
        groups_yaml: &str,
        buyers_csv: &str,
    ) -> Result<()> {
        let groups = Group::from_yaml_file(groups_yaml, self.spl_token.balance)
            .await
            .with_context(|| format!("Failed to load groups from `{}`", groups_yaml))?;

        let buyers = Buyer::load_from_csv(buyers_csv, &groups)
            .await
            .with_context(|| format!("Failed to load buyers from `{}`", buyers_csv))?;

        for group in &groups {
            self.db
                .save_group(group)
                .await
                .with_context(|| format!("Failed to save group id={} to database", group.id))?;
        }

        for buyer in &buyers {
            self.db.save_buyer(buyer).await.with_context(|| {
                format!("Failed to save buyer wallet={} to database", buyer.wallet)
            })?;
        }

        Ok(())
    }
}
