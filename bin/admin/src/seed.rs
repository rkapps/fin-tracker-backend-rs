use anyhow::{Context, Result};
use bin_shared::gcs::download_gcs_to_file;
use calamine::{Reader, Xlsx, open_workbook};
use fin_domain::tickers::TickerSeed;
use std::path::PathBuf;

pub fn load_ticker_seeds_from_file(file: PathBuf) -> Result<Vec<TickerSeed>> {
    let mut workbook: Xlsx<_> =
        open_workbook(&file).with_context(|| format!("Failed to open file: {:?}", file))?;

    let sheet = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow::anyhow!("No sheet found"))??;

    let mut tickers = Vec::new();

    for row in sheet.rows() {
        // skip header row
        let ticker = TickerSeed {
            asset_type: row[0]
                .to_string()
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid asset type: {}", e))?,
            exchange: row[1].to_string(),
            symbol: row[2].to_string(),
            name: row[3].to_string(),
            sector: row[4].to_string(),
            industry: row[5].to_string(),
            overview: row[6].to_string(),
        };
        tickers.push(ticker);
    }

    Ok(tickers)
}

pub async fn load_ticker_seeds_from_gcs(gcs_path: &str) -> anyhow::Result<PathBuf> {
    download_gcs_to_file(gcs_path).await

    // // parse gs://bucket-name/path/to/file.xlsx
    // let path = gcs_path
    //     .strip_prefix("gs://")
    //     .ok_or_else(|| anyhow::anyhow!("Invalid GCS path"))?;
    // let (bucket, object) = path
    //     .split_once('/')
    //     .ok_or_else(|| anyhow::anyhow!("Invalid GCS path"))?;

    // // create client using ADC (works automatically on Cloud Run)
    // let config = ClientConfig::default().with_auth().await?;
    // let client = Client::new(config);

    // // download object bytes
    // let data = client
    //     .download_object(
    //         &GetObjectRequest {
    //             bucket: bucket.to_string(),
    //             object: object.to_string(),
    //             ..Default::default()
    //         },
    //         &Range::default(),
    //     )
    //     .await?;

    // // write to temp file
    // let mut tmp = NamedTempFile::new()?;
    // tmp.write_all(&data)?;
    // let path = tmp.into_temp_path().keep()?;

    // Ok(path)
}
