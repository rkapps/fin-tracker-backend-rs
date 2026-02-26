use anyhow::Result;
use reqwest::{Client, header::HeaderMap};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {

    //new creates a new httpclient with the url
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
        })
    }

    pub async fn get_raw(&self, url: String, headers: Option<HeaderMap>) -> Result<String> {
        let mut request = self.client.get(&url);
        if let Some(h) = headers {
            request = request.headers(h);
        }
        let response = request.send().await?.text().await?;
        Ok(response)
    }
    
    //send an https post
    pub async fn get_request<T: serde::de::DeserializeOwned + Send>(
        &self,
        url: String,
        headers: Option<reqwest::header::HeaderMap>,
    ) -> Result<T> {
        debug!("Url: {}", url);
        let mut request = self.client.get(url);

        if let Some(h) = headers {
            request = request.headers(h);
        }

        let response = request.send().await?;
        let text = response.text().await?;
        debug!("Raw response: {:#?}", text);

        let result: T = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("Failed to deserialize response: {}", e)
        })?;

        Ok(result)
    }



    //send an https post
    pub async fn post_request<T: serde::de::DeserializeOwned + Send>(
        &self,
        url: String,
        headers: Option<reqwest::header::HeaderMap>,
        body: serde_json::Value,
    ) -> Result<T> {
        debug!("Url: {}", url);
        let mut request = self.client.post(url);

        if let Some(h) = headers {
            request = request.headers(h);
        }

        let response = request.json(&body).send().await?;

        let text = response.text().await?;
        debug!("Raw response: {:#?}", text);

        let result: T = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("Failed to deserialize response: {}. Body: {}", e, text)
        })?;

        Ok(result)
    }

    pub async fn post_stream_request(
        &self,
        url: String,
        headers: Option<reqwest::header::HeaderMap>,
        body: serde_json::Value,
    ) -> reqwest::Result<reqwest::Response> {

        debug!("Url: {}", url);
        let mut request = self.client.post(url);

        if let Some(h) = headers {
            request = request.headers(h);
        }

        // debug!("Body: {:#?}", &body);
        let res = request.json(&body).send().await?;
        Ok(res)
    }
}

