#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: Option<i64>,
    pub buyer_wallet: String,
    pub group_id: i64,
    pub amount: f64,
    pub status: String, // "pending", "success", "failed"
    pub error_message: Option<String>,
    pub scheduled_at: chrono::NaiveDateTime,
    pub sent_at: Option<chrono::NaiveDateTime>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
