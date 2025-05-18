use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use rand::Rng;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tokio::fs::File;

fn pubkey_to_string<S>(pk: &Pubkey, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&pk.to_string())
}

fn pubkey_from_string<'de, D>(d: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Buyer {
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "pubkey_from_string"
    )]
    pub wallet: Pubkey,
    pub paid_sol: f64,
    pub group_id: u8,
    #[serde(default)]
    pub received_spl: f64,
    #[serde(default)]
    pub pending_spl: f64,
    #[serde(default)]
    pub error: Option<String>,
}

//Remove in production
pub async fn generate_test_buyers_csv_async(
    path: &str,
    buyers_count: usize,
    group_count: u8,
) -> Result<()> {
    let file = File::create(path).await?;
    let mut wtr = csv_async::AsyncSerializer::from_writer(file);

    let mut rng = rand::rng();

    for _ in 0..buyers_count {
        let keypair = Keypair::new();
        let wallet = keypair.pubkey();
        let paid_sol: f64 = rng.random_range(100.0..2000.0);
        let group_id: u8 = rng.random_range(1..=group_count);

        let buyer = Buyer {
            wallet,
            paid_sol,
            group_id,
            received_spl: 0.0,
            pending_spl: 0.0,
            error: None,
        };
        wtr.serialize(buyer).await?;
    }
    log::info!("Test buyers CSV generated at {}", path);
    wtr.flush().await?;
    Ok(())
}
