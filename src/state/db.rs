use std::str::FromStr;

use anyhow::Context;
use solana_sdk::pubkey::Pubkey;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::schema::{Buyer, Group, Transaction};

pub struct DbContext {
    pool: SqlitePool,
}
impl DbContext {
    pub async fn new() -> anyhow::Result<Self> {
        //TODO:Connect db
        let options = SqliteConnectOptions::from_str("sqlite://spl_giver_history.sqlite")
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
    pub async fn save_group(&self, group: &Group) -> anyhow::Result<Group> {
        let spl_total = group.spl_total as i64;
        let unlock_interval_seconds = group.unlock_interval_seconds as i64;
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
        .fetch_one(&self.pool)
        .await
        .context("Failed to save group to database")?;
        log::debug!("Saved group to database: {:#?}", saved_group);
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

    pub async fn save_buyer(&self, buyer: &Buyer) -> anyhow::Result<Buyer> {
        //TODO: Find better way to decode data
        let wallet_str = buyer.wallet.to_string();
        let group_id = buyer.group_id as i64;

        let row = sqlx::query!(
            r#"
            INSERT OR IGNORE INTO buyers (
                wallet, paid_sol, group_id, received_spl, pending_spl, error
            ) VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *;
            "#,
            wallet_str,
            buyer.paid_sol,
            group_id,
            buyer.received_spl,
            buyer.pending_spl,
            buyer.error
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to save buyer to database")?;

        // Manually construct Buyer using your FromRow logic
        let saved_buyer = Buyer {
            wallet: Pubkey::from_str(&row.wallet)
                .map_err(|_| sqlx::Error::Decode("Invalid Pubkey".into()))?,
            paid_sol: row.paid_sol,
            group_id: row.group_id,
            received_spl: row.received_spl,
            pending_spl: row.pending_spl,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };

        log::debug!("Saved buyer to database: {:#?}", saved_buyer);
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
        pending_spl: f64,
    ) -> anyhow::Result<Buyer> {
        let row = sqlx::query!(
            r#"
            UPDATE buyers
            SET received_spl = ?, pending_spl = ?
            WHERE wallet = ? RETURNING *;
            "#,
            received_spl,
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
            pending_spl: row.pending_spl,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        log::debug!("Updated buyer in database: {:#?}", updated_buyer);
        Ok(updated_buyer)
    }

    // // --- TRANSACTIONS ---

    pub async fn save_transaction(&self, transaction: Transaction) -> anyhow::Result<Transaction> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            INSERT INTO transactions (
                buyer_wallet, group_id, amount, status, error_message, scheduled_at, sent_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *;
            "#,
            transaction.buyer_wallet,
            transaction.group_id,
            transaction.amount,
            transaction.status,
            transaction.error_message,
            transaction.scheduled_at,
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
}
