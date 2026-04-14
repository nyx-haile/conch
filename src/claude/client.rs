use crate::claude::types::{MessagesRequest, MessagesResponse};
use anyhow::{anyhow, Context};
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ClaudeClient {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn anthropic(api_key: &str) -> Self {
        Self::new(api_key, "https://api.anthropic.com")
    }

    pub async fn messages(
        &self,
        request: &MessagesRequest,
    ) -> anyhow::Result<MessagesResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .context("sending messages request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("claude api returned {}: {}", status, body));
        }

        response
            .json::<MessagesResponse>()
            .await
            .context("parsing messages response")
    }
}
