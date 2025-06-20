#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: i64,
    pub buyer_wallet: String,
    pub group_id: i64,
    pub amount_lamports: u64,
    pub percent: f64,
    pub status: String, // "pending", "success", "failed"
    pub error_message: Option<String>,
    pub sent_at: Option<chrono::NaiveDateTime>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
impl Transaction {
    pub fn new(
        buyer_wallet: String,
        group_id: i64,
        amount_lamports: u64,
        percent: f64,
        status: String,
    ) -> Self {
        Transaction {
            id: 0, // Default value, will be set by the database
            buyer_wallet,
            group_id,
            amount_lamports,
            percent,
            status,
            error_message: None,
            sent_at: Some(chrono::Utc::now().naive_utc()), // Default to current time
            created_at: None,
            updated_at: None,
        }
    }
}
