use crate::llm::anthropic::AnthropicClient;
use crate::llm::types::{ChatRequest, ChatResponse};
use anyhow::{anyhow, Context};
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct OpenRouterClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl OpenRouterClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .http
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .header("http-referer", "https://github.com/conch-cli/conch")
            .header("x-title", "conch")
            .json(request)
            .send()
            .await
            .context("sending chat completions request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("openrouter api returned {}: {}", status, body));
        }

        response
            .json::<ChatResponse>()
            .await
            .context("parsing chat completions response")
    }
}

#[derive(Debug, Clone)]
pub enum LlmClient {
    OpenRouter(OpenRouterClient),
    Anthropic(AnthropicClient),
}

impl LlmClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::OpenRouter(OpenRouterClient::new(api_key, base_url))
    }

    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "https://openrouter.ai/api/v1")
    }

    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self::Anthropic(AnthropicClient::new(api_key, "https://api.anthropic.com"))
    }

    pub fn anthropic_with_base_url(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::Anthropic(AnthropicClient::new(api_key, base_url))
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        match self {
            Self::OpenRouter(c) => c.chat(request).await,
            Self::Anthropic(c) => c.chat(request).await,
        }
    }
}
