use std::str::FromStr;

use anyhow::Context;
use solana_sdk::pubkey::Pubkey;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::schema::{Buyer, Group, Schedule, Transaction};

pub struct DbContext {
    pool: SqlitePool,
}
impl DbContext {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)
            .context("Failed to create SQLite connect options")?
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        //Apply migrations
        sqlx::migrate!()
            .run(&pool)
            .await
            .context("Database migration error")?;
        Ok(Self { pool })
    }
    pub async fn save_group(&self, group: &Group) -> anyhow::Result<Option<Group>> {
        let spl_total = group.spl_total as i64;
        let unlock_interval_seconds = group.unlock_interval_seconds;
        let saved_group = sqlx::query_as!(
            Group,
            r#"
        INSERT OR IGNORE INTO groups (
            id, spl_share_percent, spl_total, spl_price,
            initial_unlock_percent, unlock_interval_seconds,
            unlock_percent_per_interval
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *;
        "#,
            group.id,
            group.spl_share_percent,
            spl_total,
            group.spl_price,
            group.initial_unlock_percent,
            unlock_interval_seconds,
            group.unlock_percent_per_interval
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to save group to database")?;

        if let Some(ref g) = saved_group {
            log::debug!("Saved group to database: {:#?}", g);
        } else {
            log::debug!("Group with id {} already exists, not inserted.", group.id);
        }
        Ok(saved_group)
    }

    pub async fn get_groups(&self) -> anyhow::Result<Vec<Group>> {
        let groups = sqlx::query_as!(
            Group,
            r#"
            SELECT * FROM groups;
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all groups from database")?;
        log::debug!("Retrieved groups from database: {:#?}", groups);
        Ok(groups)
    }
    pub async fn get_group(&self, group_id: i64) -> anyhow::Result<Group> {
        let row = sqlx::query_as!(
            Group,
            r#"
            SELECT * FROM groups WHERE id = ?
            "#,
            group_id
        )
        .fetch_one(&self.pool)
        .await
        .context(format!("Failed to get group with id {}", group_id))?;
        Ok(row)
    }

    pub async fn save_buyer(&self, buyer: &Buyer) -> anyhow::Result<Option<Buyer>> {
        let wallet_str = buyer.wallet.to_string();
        let group_id = buyer.group_id;

        let row = sqlx::query!(
            r#"
        INSERT OR IGNORE INTO buyers (
            wallet, paid_sol, group_id, received_spl, received_percent, pending_spl, error
        ) VALUES (?, ?, ?, ?, ?, ?,?)
        RETURNING *;
        "#,
            wallet_str,
            buyer.paid_sol,
            group_id,
            buyer.received_spl,
            buyer.received_percent,
            buyer.pending_spl,
            buyer.error
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to save buyer to database")?;

        let saved_buyer = row.map(|row| Buyer {
            wallet: Pubkey::from_str(&row.wallet)
                .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))
                .unwrap(), // handle error as needed
            paid_sol: row.paid_sol,
            group_id: row.group_id,
            received_spl: row.received_spl,
            received_percent: row.received_percent,
            pending_spl: row.pending_spl,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        });

        if let Some(ref b) = saved_buyer {
            log::debug!("Saved buyer to database: {:#?}", b);
        } else {
            log::debug!(
                "Buyer with wallet {} already exists, not inserted.",
                wallet_str
            );
        }
        Ok(saved_buyer)
    }

    pub async fn get_buyers_by_group(&self, group_id: i64) -> anyhow::Result<Vec<Buyer>> {
        let rows = sqlx::query!(
            r#"
            SELECT * FROM buyers WHERE group_id = ?;
            "#,
            group_id
        )
        .fetch_all(&self.pool)
        .await
        .context(format!(
            "Failed to get buyers for group with ID: {:?}",
            group_id
        ))?;

        let buyers = rows
            .into_iter()
            .map(|row| Buyer {
                wallet: Pubkey::from_str(&row.wallet)
                    .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))
                    .unwrap(), // handle error as needed
                paid_sol: row.paid_sol,
                group_id: row.group_id,
                received_spl: row.received_spl,
                received_percent: row.received_percent,
                pending_spl: row.pending_spl,
                error: row.error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();
        log::debug!("Retrieved buyers for group {}: {:#?}", group_id, buyers);
        Ok(buyers)
    }
    pub async fn update_buyer(
        &self,
        wallet: &str,
        received_spl: f64,
        received_percent: f64,
        pending_spl: f64,
    ) -> anyhow::Result<Buyer> {
        let row = sqlx::query!(
            r#"
            UPDATE buyers
            SET received_spl = ?, received_percent = ?, pending_spl = ?
            WHERE wallet = ? RETURNING *;
            "#,
            received_spl,
            received_percent,
            pending_spl,
            wallet
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to update buyer in database")?;

        let updated_buyer = Buyer {
            wallet: Pubkey::from_str(&row.wallet)
                .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))?,
            paid_sol: row.paid_sol,
            group_id: row.group_id,
            received_spl: row.received_spl,
            received_percent: row.received_percent,
            pending_spl: row.pending_spl,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        log::debug!("Updated buyer in database: {:#?}", updated_buyer);
        Ok(updated_buyer)
    }

    pub async fn get_buyer_by_wallet(&self, wallet: &str) -> anyhow::Result<Buyer> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM buyers WHERE wallet = ?
            "#,
            wallet
        )
        .fetch_one(&self.pool)
        .await
        .context(format!("Failed to get buyer with wallet {}", wallet))?;

        let buyer = Buyer {
            wallet: Pubkey::from_str(&row.wallet)
                .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))?,
            paid_sol: row.paid_sol,
            group_id: row.group_id,
            received_spl: row.received_spl,
            received_percent: row.received_percent,
            pending_spl: row.pending_spl,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        Ok(buyer)
    }

    // // --- TRANSACTIONS ---

    pub async fn save_transaction(&self, transaction: Transaction) -> anyhow::Result<Transaction> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            INSERT INTO transactions (
                buyer_wallet, group_id, amount, percent, status, error_message, sent_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *;
            "#,
            transaction.buyer_wallet,
            transaction.group_id,
            transaction.amount,
            transaction.percent,
            transaction.status,
            transaction.error_message,
            transaction.sent_at
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to save transaction")?;
        log::debug!("Saved transaction to database: {:#?}", transaction);
        Ok(transaction)
    }

    pub async fn get_failed_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                *
            FROM transactions
            WHERE status = 'failed'
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get failed transactions")?;
        log::debug!("Retrieved failed transactions: {:#?}", transactions);
        Ok(transactions)
    }

    pub async fn get_all_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                *
            FROM transactions
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all transactions")?;
        log::debug!("Retrieved all transactions: {:#?}", transactions);
        Ok(transactions)
    }

    pub async fn add_schedule(&self, schedule: &Schedule) -> anyhow::Result<Schedule> {
        let row = sqlx::query_as!(
            Schedule,
            r#"
            INSERT INTO schedule (
                group_id, buyer_wallet, scheduled_at, amount, percent, status
            ) VALUES (?, ?, ?, ?, ?, ?)
            RETURNING
                *
            "#,
            schedule.group_id,
            schedule.buyer_wallet,
            schedule.scheduled_at,
            schedule.amount,
            schedule.percent,
            schedule.status
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to add schedule")?;
        log::debug!("Added schedule to database: {:#?}", row);
        Ok(row)
    }

    pub async fn get_schedules_by_status(&self, status: &str) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
            SELECT
                *
            FROM schedule
            WHERE status = ?
            "#,
            status
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get schedules by status")?;
        log::debug!("Retrieved schedules with status {}: {:#?}", status, rows);
        Ok(rows)
    }

    pub async fn get_all_schedules(&self) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
            SELECT
                *
            FROM schedule
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all schedules")?;
        log::debug!("Retrieved all schedules: {:#?}", rows);
        Ok(rows)
    }
    pub async fn get_schedules_due(
        &self,
        now: chrono::NaiveDateTime,
    ) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
            SELECT
                *
            FROM schedule
            WHERE  scheduled_at <= ? AND status = 'pending'
            "#,
            now //maybe in future also check status
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get schedules due")?;
        Ok(rows)
    }
    pub async fn get_schedules_by_buyer_and_group(
        &self,
        buyer_wallet: &str,
        group_id: i64,
    ) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
                SELECT * FROM schedule WHERE buyer_wallet = ? AND group_id = ?
            "#,
            buyer_wallet,
            group_id
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get schedules by buyer and group")?;
        log::debug!(
            "Retrieved schedules for buyer {} in group {}: {:#?}",
            buyer_wallet,
            group_id,
            rows
        );
        Ok(rows)
    }
    pub async fn update_schedule_status(
        &self,
        schedule_id: i64,
        status: &str,
        error_message: Option<String>,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE schedule
            SET status = ?, 
                updated_at = CURRENT_TIMESTAMP,
                error_message = ?
            WHERE id = ?
            "#,
            status,
            error_message,
            schedule_id
        )
        .execute(&self.pool)
        .await
        .context(format!(
            "Failed to update schedule status for id {}",
            schedule_id
        ))?;
        log::debug!("Updated schedule status for id {}: {}", schedule_id, status);
        Ok(())
    }
    pub async fn delete_schedule(&self, schedule_id: i64) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM schedule WHERE id = ?
            "#,
            schedule_id
        )
        .execute(&self.pool)
        .await
        .context(format!("Failed to delete schedule with id {}", schedule_id))?;
        log::debug!("Deleted schedule with id {}", schedule_id);
        Ok(())
    }
}
