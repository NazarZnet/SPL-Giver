use anyhow::Result;

use rand::Rng;
use solana_sdk::signature::{Keypair, Signer};
use tokio::fs::File;

use crate::schema::Buyer;

//Remove in production
pub async fn generate_test_buyers_csv_async(
    path: &str,
    buyers_count: usize,
    group_count: i64,
) -> Result<()> {
    let file = File::create(path).await?;
    let mut wtr = csv_async::AsyncSerializer::from_writer(file);

    let mut rng = rand::rng();

    for _ in 0..buyers_count {
        let keypair = Keypair::new();
        let wallet = keypair.pubkey();
        let paid_sol: f64 = rng.random_range(100.0..2000.0);
        let group_id: i64 = rng.random_range(1..=group_count);

        let buyer = Buyer {
            wallet,
            paid_sol,
            group_id,
            received_spl: 0.0,
            pending_spl: 0.0,
            error: None,
            created_at: None,
            updated_at: None,
        };
        wtr.serialize(buyer).await?;
    }
    log::info!("Test buyers CSV generated at {}", path);
    wtr.flush().await?;
    Ok(())
}
