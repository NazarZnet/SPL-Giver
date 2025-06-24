use anyhow::{Context, Result};
use common::{Buyer, Database, Group, SplToken, Transaction};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub struct AppState {
    pub spl_token: SplToken,
    pub db: Database,
    pub retry_queue: RetryQueue,
}
impl AppState {
    pub async fn new<P: AsRef<Path>>(
        database_url: &str,
        client_url: &str,
        wallet: &str,
        mint: &str,
        retry_queue_path: P,
    ) -> Result<Self> {
        let spl_token_context = SplToken::new(client_url, wallet, mint).await?;

        let db = Database::new(database_url).await?;
        log::info!("Database initialized successfully!");
        let retry_queue = RetryQueue::load(retry_queue_path).await?;

        Ok(AppState {
            spl_token: spl_token_context,
            db,
            retry_queue,
        })
    }
    pub async fn from_env<P: AsRef<Path>>(retry_queue_path: P) -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let client_url = std::env::var("CLIENT_URL").context("CLIENT_URL must be set")?;
        let wallet = std::env::var("MAIN_WALLET").context("MAIN_WALLET must be set")?;
        let mint = std::env::var("MINT_PUBKEY").context("MINT_PUBKEY must be set")?;

        let state = AppState::new(&database_url, &client_url, &wallet, &mint, retry_queue_path)
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

/// An operation that must be applied to the database.
#[derive(Serialize, Deserialize, Clone)]
pub enum PendingOp {
    SaveTransaction(Transaction),
    UpdateBuyer {
        wallet: String,
        received_spl: u64,
        received_percent: f64,
        pending_spl: u64,
    },
    UpdateSchedule {
        schedule_id: i64,
        status: String,
        error_message: Option<String>,
    },
}

/// A persistent queue of operations to retry on DB failure
pub struct RetryQueue {
    path: PathBuf,
    pub ops: Mutex<Vec<PendingOp>>,
}

impl RetryQueue {
    /// Asynchronously load the retry queue from disk, creating the file if it doesn't exist.
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        let mut file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&path_buf)
            .await
            .context(format!(
                "Failed to open `{}` for reading",
                path_buf.display()
            ))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await.context(format!(
            "Failed to read contents of `{}`",
            path_buf.display()
        ))?;

        let ops = if contents.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&contents)
                .context(format!("Invalid JSON in `{}`", path_buf.display()))?
        };

        Ok(RetryQueue {
            path: path_buf,
            ops: Mutex::new(ops),
        })
    }

    /// Asynchronously save the retry queue to disk atomically.
    pub async fn save(&self) -> Result<()> {
        let guard = self.ops.lock().await;
        let data = serde_json::to_vec_pretty(&*guard).context("Failed to serialize retry queue")?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await
            .context(format!("Could not open file `{}`", self.path.display()))?;

        file.write_all(&data)
            .await
            .context("Failed to write to temp file")?;
        file.flush().await.context("Failed to flush temp file")?;

        Ok(())
    }

    /// Append a new operation and persist immediately.
    pub async fn push_and_persist(&self, op: PendingOp) -> Result<()> {
        let mut guard = self.ops.lock().await;
        guard.push(op);
        self.save().await.context("Failed to save retry queue")
    }

    /// Attempt all pending operations, removing those that succeed.
    pub async fn flush(&self, db: &Database) -> Result<()> {
        let pending_ops = {
            let mut guard = self.ops.lock().await;
            std::mem::take(&mut *guard)
        };
        let mut remaining = Vec::new();
        for op in pending_ops {
            let outcome = match &op {
                PendingOp::SaveTransaction(tx) => db.save_transaction(tx.clone()).await.map(|_| ()),
                PendingOp::UpdateBuyer {
                    wallet,
                    received_spl,
                    received_percent,
                    pending_spl,
                } => db
                    .update_buyer(wallet, *received_spl, *received_percent, *pending_spl)
                    .await
                    .map(|_| ()),
                PendingOp::UpdateSchedule {
                    schedule_id,
                    status,
                    error_message,
                } => db
                    .update_schedule_status(*schedule_id, status, error_message.clone())
                    .await
                    .map(|_| ()),
            };

            if let Err(e) = outcome {
                log::error!("Operation failed, will retry later: {:?}", e);
                remaining.push(op);
            }
        }

        // Save only the remaining failures back
        {
            let mut guard = self.ops.lock().await;
            *guard = remaining;
        }
        self.save()
            .await
            .context("Failed to save updated retry queue")
    }
}
