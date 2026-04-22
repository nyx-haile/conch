pub mod anthropic;
pub mod client;
pub mod types;

use crate::config::Config;
use crate::llm::client::LlmClient;
use crate::model::Model;
use crate::provider::Provider;
use anyhow::{anyhow, Result};

/// Resolve the right LLM client + model slug for the given provider.
///
/// `Provider::Anthropic` uses the native Anthropic API with `ANTHROPIC_API_KEY`.
/// All other providers route through OpenRouter with `OPENROUTER_API_KEY`.
pub fn resolve(config: &Config, provider: Provider, model: Model) -> Result<(LlmClient, String)> {
    match provider {
        Provider::Anthropic => {
            let key = config
                .anthropic_api_key()
                .ok_or_else(|| anyhow!("ANTHROPIC_API_KEY is not set"))?;
            Ok((
                LlmClient::anthropic(key),
                anthropic::native_model_id(model).to_string(),
            ))
        }
        other => {
            let key = config
                .openrouter_api_key()
                .ok_or_else(|| anyhow!("OPENROUTER_API_KEY is not set"))?;
            Ok((
                LlmClient::openrouter(key),
                other.slug_for(model).to_string(),
            ))
        }
    }
}
