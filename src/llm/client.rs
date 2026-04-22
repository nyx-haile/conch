use crate::llm::anthropic::AnthropicClient;
use crate::llm::types::{ChatRequest, ChatResponse, Choice, Message, Role};
use anyhow::{anyhow, Context};
use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// Counter for scripted LLM responses. Only meaningful when
/// `CONCH_TEST_SCRIPTED_LLM` is set. Note: this is process-global and
/// never resets — safe when the scripted mode is used from a subprocess
/// (e.g., the smoke test), but would need per-test reset logic if used
/// from in-process `#[tokio::test]` tests.
static SCRIPTED_LLM_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Cached check for the scripted LLM env var (checked once per process).
static SCRIPTED_LLM_SCRIPT: OnceLock<Option<String>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ChatCompletionsClient {
    http: Client,
    base_url: String,
    api_key: String,
    flavor: ChatCompletionsFlavor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatCompletionsFlavor {
    Generic,
    OpenRouter,
}

impl ChatCompletionsClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::with_flavor(api_key, base_url, ChatCompletionsFlavor::Generic)
    }

    pub fn openrouter(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::with_flavor(api_key, base_url, ChatCompletionsFlavor::OpenRouter)
    }

    fn with_flavor(
        api_key: impl Into<String>,
        base_url: &str,
        flavor: ChatCompletionsFlavor,
    ) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            flavor,
        }
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut req = self
            .http
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json");
        if self.flavor == ChatCompletionsFlavor::OpenRouter {
            req = req
                .header("http-referer", "https://github.com/conch-cli/conch")
                .header("x-title", "conch");
        }
        let response = req
            .json(request)
            .send()
            .await
            .context("sending chat completions request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "chat completions api returned {}: {}",
                status,
                body
            ));
        }

        response
            .json::<ChatResponse>()
            .await
            .context("parsing chat completions response")
    }
}

#[derive(Debug, Clone)]
pub enum LlmClient {
    ChatCompletions(ChatCompletionsClient),
    Anthropic(AnthropicClient),
}

impl LlmClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::ChatCompletions(ChatCompletionsClient::new(api_key, base_url))
    }

    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::openrouter_with_base_url(api_key, "https://openrouter.ai/api/v1")
    }

    pub fn openrouter_with_base_url(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::ChatCompletions(ChatCompletionsClient::openrouter(api_key, base_url))
    }

    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::openai_with_base_url(api_key, "https://api.openai.com/v1")
    }

    pub fn openai_with_base_url(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::ChatCompletions(ChatCompletionsClient::new(api_key, base_url))
    }

    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self::Anthropic(AnthropicClient::new(api_key, "https://api.anthropic.com"))
    }

    pub fn anthropic_with_base_url(api_key: impl Into<String>, base_url: &str) -> Self {
        Self::Anthropic(AnthropicClient::new(api_key, base_url))
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let cached =
            SCRIPTED_LLM_SCRIPT.get_or_init(|| std::env::var("CONCH_TEST_SCRIPTED_LLM").ok());
        if let Some(script) = cached {
            let parts: Vec<&str> = script.split('|').collect();
            let idx = SCRIPTED_LLM_INDEX.fetch_add(1, Ordering::SeqCst);
            let text = parts
                .get(idx)
                .unwrap_or(&"(scripted: no more replies)")
                .to_string();
            return Ok(ChatResponse {
                id: Some("scripted".to_string()),
                choices: vec![Choice {
                    index: 0,
                    message: Message {
                        role: Role::Assistant,
                        content: Some(text),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    finish_reason: "stop".to_string(),
                }],
                model: Some("scripted".to_string()),
            });
        }

        match self {
            Self::ChatCompletions(c) => c.chat(request).await,
            Self::Anthropic(c) => c.chat(request).await,
        }
    }
}
