use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Schedule {
    pub id: i64,
    pub group_id: i64,
    pub buyer_wallet: String,
    pub scheduled_at: NaiveDateTime,
    pub amount_lamports: u64,
    pub percent: f64,
    pub status: String, // "pending",  "success", "failed"
    pub error_message: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Schedule {
    pub fn new(
        group_id: i64,
        buyer_wallet: String,
        scheduled_at: NaiveDateTime,
        amount_lamports: u64,
        percent: f64,
    ) -> Self {
        Schedule {
            id: 0,
            group_id,
            buyer_wallet,
            scheduled_at,
            amount_lamports,
            percent,
            status: "pending".to_string(), // Default status
            error_message: None,
            created_at: None,
            updated_at: None,
        }
    }
}
