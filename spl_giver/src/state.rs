use anyhow::Result;
use common::{Database, SplToken};

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
}
