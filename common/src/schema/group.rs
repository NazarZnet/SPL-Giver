#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Group {
    pub id: i64,
    pub spl_share_percent: f64,
    #[serde(default)]
    pub spl_total_lamports: u64,
    pub spl_price_lamports: u64,
    pub initial_unlock_percent: f64,
    pub unlock_interval_seconds: i64,
    pub unlock_percent_per_interval: f64,
    #[serde(default)]
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(default)]
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl Group {
    pub async fn from_yaml_file(path: &str, total_amount: u64) -> anyhow::Result<Vec<Group>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut groups: Vec<Group> = serde_yaml::from_str(&content)?;
        groups.iter_mut().for_each(|g| {
            g.spl_total_lamports = (g.spl_share_percent * total_amount as f64).round() as u64;
        });
        Ok(groups)
    }
}
