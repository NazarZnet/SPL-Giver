use std::str::FromStr;
use tokio_stream::StreamExt;

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

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
    pub group_id: i64,
    #[serde(default)]
    pub received_spl: f64,
    #[serde(default)]
    pub pending_spl: f64,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(default)]
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl Buyer {
    pub async fn load_from_csv(path: &str) -> anyhow::Result<Vec<Buyer>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(true)
            .create_deserializer(content.as_bytes());
        let mut records = rdr.deserialize::<Buyer>();
        let mut buyers = Vec::new();
        while let Some(buyer) = records.next().await {
            match buyer {
                Ok(buyer) => {
                    buyers.push(buyer);
                }
                Err(e) => {
                    log::error!("Error deserializing buyer: {}", e);
                    continue; // Skip this record
                }
            }
        }
        log::debug!("Loaded buyers from CSV file: {:#?}", buyers);
        if buyers.is_empty() {
            return Err(anyhow::anyhow!("No buyers found in the CSV file"));
        }
        Ok(buyers)
    }
}
