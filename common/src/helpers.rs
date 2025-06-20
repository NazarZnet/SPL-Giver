use anyhow::Context;
use tokio::io::AsyncWriteExt;

pub async fn save_to_env(key: &str, value: &str, file_path: &str) -> anyhow::Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .context("Failed to open .env file")?;
    let line = format!("{}={}\n", key, value);
    file.write_all(line.as_bytes())
        .await
        .context("Failed to write to .env file")?;
    Ok(())
}
