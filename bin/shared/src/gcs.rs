// bin/shared/src/gcs.rs

use std::io::Write;
use std::path::PathBuf;

use google_cloud_storage::{
    client::{Client, ClientConfig},
    http::objects::{download::Range, get::GetObjectRequest},
};
use tempfile::NamedTempFile;

pub async fn download_gcs_bytes(gcs_path: &str) -> anyhow::Result<Vec<u8>> {
    let path = gcs_path
        .strip_prefix("gs://")
        .ok_or_else(|| anyhow::anyhow!("Invalid GCS path: {}", gcs_path))?;
    let (bucket, object) = path
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid GCS path: {}", gcs_path))?;

    let config = ClientConfig::default().with_auth().await?;
    let client = Client::new(config);

    let data = client
        .download_object(
            &GetObjectRequest {
                bucket: bucket.to_string(),
                object: object.to_string(),
                ..Default::default()
            },
            &Range::default(),
        )
        .await?;

    Ok(data)
}

// for files that need a temp path (xlsx, models)
pub async fn download_gcs_to_file(gcs_path: &str) -> anyhow::Result<PathBuf> {
    let data = download_gcs_bytes(gcs_path).await?;
    let mut tmp = NamedTempFile::new()?;
    tmp.write_all(&data)?;
    Ok(tmp.into_temp_path().keep()?)
}

// for text content (config, json)
pub async fn download_gcs_string(gcs_path: &str) -> anyhow::Result<String> {
    let data = download_gcs_bytes(gcs_path).await?;
    Ok(String::from_utf8(data)?)
}
