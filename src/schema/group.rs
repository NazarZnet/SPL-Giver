#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Group {
    pub id: i64,
    pub spl_share_percent: f64,
    #[serde(default)]
    pub spl_total: f64,
    pub spl_price: f64,
    pub initial_unlock_percent: f64,
    pub unlock_interval_seconds: i64,
    pub unlock_percent_per_interval: f64,
    //TODO: REmove this field. It doesnt make sense
    #[serde(default)]
    pub unlock_task_spawned: bool,
    #[serde(default)]
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(default)]
    pub updated_at: Option<chrono::NaiveDateTime>,
    // #[serde(skip)]
    // #[serde(default)]
    // pub unlock_task: Option<Arc<Mutex<JoinHandle<()>>>>,
}

impl Group {
    pub async fn from_yaml_file(path: &str, tokens_amount: f64) -> anyhow::Result<Vec<Group>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut groups: Vec<Group> = serde_yaml::from_str(&content)?;
        groups.iter_mut().for_each(|g| {
            g.spl_total = g.spl_share_percent * tokens_amount;
        });
        log::debug!("Loaded groups from YAML file: {:#?}", groups);
        Ok(groups)
    }
}
