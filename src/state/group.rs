
use std::sync::Arc;
use tokio_stream::StreamExt;

use anyhow::Result;
use tokio::{fs, sync::Mutex};

use crate::schema::{Buyer, Group};

#[derive(Debug)]
pub struct GroupContext {
    pub groups: Vec<Arc<Mutex<Group>>>,
}

impl GroupContext {
    pub async fn from_yaml_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let groups: Vec<Group> = serde_yaml::from_str(&content)?;
        let groups = groups
            .into_iter()
            .map(|g| Arc::new(Mutex::new(g)))
            .collect();
        Ok(GroupContext { groups })
    }

    pub async fn update_groups_spl_amount(&mut self, amount: f64) {
        for group in &mut self.groups {
            let mut group = group.lock().await;
            group.spl_total = group.spl_share_percent * amount;
        }
    }

    pub async fn load_buyers_from_csv(&mut self, path: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(true)
            .create_deserializer(content.as_bytes());
        let mut records = rdr.deserialize::<Buyer>();
        while let Some(buyer) = records.next().await {
            let buyer = buyer?;
            // Find the group asynchronously

            for group_arc in &self.groups {
                let  group = group_arc.lock().await;
                if group.id == buyer.group_id {
                    // group.buyers.push(buyer);
                    //TODO: save buyers and group to database
                    break;
                }
            }
        }
        Ok(())
    }
}
