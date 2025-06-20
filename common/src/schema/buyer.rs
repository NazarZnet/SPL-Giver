use rand::Rng;
use std::str::FromStr;
use tokio_stream::StreamExt;

use serde::{Deserialize, Serialize};
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signer::Signer};

use crate::schema::Group;

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
    pub paid_lamports: u64,
    pub group_id: i64,
    #[serde(default)]
    pub received_spl_lamports: u64,
    #[serde(default)]
    pub received_percent: f64,
    #[serde(default)]
    pub pending_spl_lamports: u64,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(default)]
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl Buyer {
    pub async fn load_from_csv(path: &str, groups: &[Group]) -> anyhow::Result<Vec<Buyer>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(true)
            .create_deserializer(content.as_bytes());
        let mut records = rdr.deserialize::<Buyer>();
        let mut buyers = Vec::new();
        while let Some(buyer_result) = records.next().await {
            match buyer_result {
                Ok(mut buyer) => {
                    if buyer.pending_spl_lamports == 0 {
                        if let Some(group) = groups.iter().find(|g| g.id == buyer.group_id) {
                            buyer.pending_spl_lamports =
                                buyer.paid_lamports / group.spl_price_lamports;
                        } else {
                            log::warn!(
                                "Group not found for buyer: {} group_id={}",
                                buyer.wallet,
                                buyer.group_id
                            );
                        }
                    }

                    buyers.push(buyer);
                }
                Err(e) => {
                    log::error!("Error deserializing buyer: {}", e);
                    continue;
                }
            }
        }
        log::debug!("Loaded buyers from CSV file: {:#?}", buyers);
        if buyers.is_empty() {
            return Err(anyhow::anyhow!("No buyers found in the CSV file"));
        }
        Ok(buyers)
    }

    //Remove in production
    pub async fn generate_test_buyers_csv_async(
        path: &str,
        buyers_count: i64,
        group_count: i64,
    ) -> anyhow::Result<()> {
        let file = tokio::fs::File::create(path).await?;
        let mut wtr = csv_async::AsyncSerializer::from_writer(file);

        let mut rng = rand::rng();

        for _ in 0..buyers_count {
            let keypair = solana_sdk::signature::Keypair::new();
            let wallet = keypair.pubkey();
            let paid_lamports: u64 =
                rng.random_range(100 * LAMPORTS_PER_SOL..2000 * LAMPORTS_PER_SOL);
            let group_id: i64 = rng.random_range(1..=group_count);

            let buyer = Buyer {
                wallet,
                paid_lamports,
                group_id,
                received_spl_lamports: 0,
                received_percent: 0.0,
                pending_spl_lamports: 0,
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
}
