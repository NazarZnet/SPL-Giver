use std::str::FromStr;

use anyhow::Context;
use solana_sdk::pubkey::Pubkey;
use sqlx::{MySqlPool, mysql::MySqlConnectOptions};

use crate::schema::{Buyer, Group, Schedule, Transaction};

pub struct Database {
    pool: MySqlPool,
}
impl Database {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let options = MySqlConnectOptions::from_str(database_url)
            .context("Failed to create SQLite connect options")?;
        let pool = MySqlPool::connect_with(options).await?;
        Ok(Self { pool })
    }
    pub async fn save_group(&self, group: &Group) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
                INSERT IGNORE INTO `groups` (
                    id, spl_share_percent, spl_total_lamports, spl_price_lamports,
                    initial_unlock_percent, unlock_interval_seconds,
                    unlock_percent_per_interval
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            group.id,
            group.spl_share_percent,
            group.spl_total_lamports,
            group.spl_price_lamports,
            group.initial_unlock_percent,
            group.unlock_interval_seconds,
            group.unlock_percent_per_interval
        )
        .execute(&self.pool)
        .await
        .context("Failed to save group to database")?;

        Ok(())
    }

    pub async fn get_groups(&self) -> anyhow::Result<Vec<Group>> {
        let groups = sqlx::query_as!(
            Group,
            r#"
            SELECT * FROM `groups`;
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all groups from database")?;
        Ok(groups)
    }
    pub async fn get_group(&self, group_id: i64) -> anyhow::Result<Group> {
        let row = sqlx::query_as!(
            Group,
            r#"
            SELECT * FROM `groups` WHERE id = ?
            "#,
            group_id
        )
        .fetch_one(&self.pool)
        .await
        .context(format!("Failed to get group with id {}", group_id))?;
        Ok(row)
    }

    pub async fn save_buyer(&self, buyer: &Buyer) -> anyhow::Result<()> {
        let wallet_str = buyer.wallet.to_string();
        sqlx::query!(
            r#"
            INSERT IGNORE INTO `buyers` (
                wallet, paid_lamports, group_id, received_spl_lamports, received_percent, pending_spl_lamports, error
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            wallet_str,
            buyer.paid_lamports,
            buyer.group_id,
            buyer.received_spl_lamports,
            buyer.received_percent,
            buyer.pending_spl_lamports,
            buyer.error
        )
        .execute(&self.pool)
        .await
        .context("Failed to save buyer to database")?;

        Ok(())
    }

    pub async fn get_buyers_by_group(&self, group_id: i64) -> anyhow::Result<Vec<Buyer>> {
        let rows = sqlx::query!(
            r#"
            SELECT * FROM `buyers` WHERE group_id = ?;
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
                paid_lamports: row.paid_lamports,
                group_id: row.group_id,
                received_spl_lamports: row.received_spl_lamports,
                received_percent: row.received_percent,
                pending_spl_lamports: row.pending_spl_lamports,
                error: row.error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();
        Ok(buyers)
    }
    pub async fn update_buyer(
        &self,
        wallet: &str,
        received_spl_lamports: u64,
        received_percent: f64,
        pending_spl_lamports: u64,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE `buyers`
            SET received_spl_lamports = ?, received_percent = ?, pending_spl_lamports = ?
            WHERE wallet = ?
            "#,
            received_spl_lamports,
            received_percent,
            pending_spl_lamports,
            wallet
        )
        .execute(&self.pool)
        .await
        .context("Failed to update buyer in database")?;

        Ok(())
    }

    pub async fn get_buyer_by_wallet(&self, wallet: &str) -> anyhow::Result<Buyer> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM `buyers` WHERE wallet = ?
            "#,
            wallet
        )
        .fetch_one(&self.pool)
        .await
        .context(format!("Failed to get buyer with wallet {}", wallet))?;

        let buyer = Buyer {
            wallet: Pubkey::from_str(&row.wallet)
                .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))?,
            paid_lamports: row.paid_lamports,
            group_id: row.group_id,
            received_spl_lamports: row.received_spl_lamports,
            received_percent: row.received_percent,
            pending_spl_lamports: row.pending_spl_lamports,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        Ok(buyer)
    }

    pub async fn save_transaction(&self, transaction: Transaction) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO `transactions` (
                buyer_wallet, group_id, amount_lamports, percent, status, error_message, sent_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            transaction.buyer_wallet,
            transaction.group_id,
            transaction.amount_lamports,
            transaction.percent,
            transaction.status,
            transaction.error_message,
            transaction.sent_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to save transaction")?;

        Ok(())
    }

    pub async fn get_failed_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                *
            FROM `transactions`
            WHERE status = 'failed'
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get failed transactions")?;

        Ok(transactions)
    }

    pub async fn get_all_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                *
            FROM `transactions`
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all transactions")?;

        Ok(transactions)
    }

    pub async fn add_schedule(&self, schedule: &Schedule) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO `schedule` (
                group_id, buyer_wallet, scheduled_at, amount_lamports, percent, status
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            schedule.group_id,
            schedule.buyer_wallet,
            schedule.scheduled_at,
            schedule.amount_lamports,
            schedule.percent,
            schedule.status
        )
        .execute(&self.pool)
        .await
        .context("Failed to add schedule")?;

        Ok(())
    }

    pub async fn get_schedules_by_status(&self, status: &str) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
            SELECT
                *
            FROM `schedule`
            WHERE status = ?
            "#,
            status
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get schedules by status")?;

        Ok(rows)
    }

    pub async fn get_all_schedules(&self) -> anyhow::Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            r#"
            SELECT
                *
            FROM `schedule`
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all schedules")?;

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
            FROM `schedule`
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
                SELECT * FROM `schedule` WHERE buyer_wallet = ? AND group_id = ?
            "#,
            buyer_wallet,
            group_id
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get schedules by buyer and group")?;

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
            UPDATE `schedule`
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

        Ok(())
    }
    pub async fn delete_schedule(&self, schedule_id: i64) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM `schedule` WHERE id = ?
            "#,
            schedule_id
        )
        .execute(&self.pool)
        .await
        .context(format!("Failed to delete schedule with id {}", schedule_id))?;

        Ok(())
    }
}
