use anyhow::Result;
use reqwest::header::HeaderValue;
use crate::{HttpClient, cmc::model::CmcCryptoData};

const COINMARKETCAP_BASE_URL: &str = "https://pro-api.coinmarketcap.com/";


pub async fn get_crypto(
    http_client: &HttpClient,
    symbols: Vec<String>,
    api_key: &str,
) -> Result<CmcCryptoData> {
    let url = format!(
        "{}v2/cryptocurrency/quotes/latest?symbol={}",
        COINMARKETCAP_BASE_URL, symbols.join(",")
    );
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("X-CMC_PRO_API_KEY", HeaderValue::from_str(api_key)?);
    headers.insert("Accept", HeaderValue::from_str("application/json")?);
    let cryptodata = http_client
        .get_request::<CmcCryptoData>(url, Some(headers))
        .await?;

    Ok(cryptodata)
}
