pub mod anthropic;
pub mod catalog;
pub mod client;
pub mod types;

use crate::config::Config;
use crate::llm::catalog::{resolve_openrouter_model, ModelCatalogEntry, ModelEntitlement};
use crate::llm::client::LlmClient;
use crate::model::Model;
use crate::provider::Provider;
use anyhow::{anyhow, Result};

/// Resolve the right LLM client + model slug for the given provider.
///
/// `Provider::Anthropic` uses the native Anthropic API with `ANTHROPIC_API_KEY`.
/// `Provider::OpenAI` uses the native OpenAI Chat Completions API with
/// `OPENAI_API_KEY` or `OPENAI_API_KEY_FILE`.
/// `Provider::OpenRouter` and routed model-family aliases use OpenRouter with
/// `OPENROUTER_API_KEY`.
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
        Provider::OpenAI => {
            let key = config
                .resolve_openai_api_key()?
                .ok_or_else(|| anyhow!("OPENAI_API_KEY or OPENAI_API_KEY_FILE is not set"))?;
            Ok((
                LlmClient::openai(&key),
                Provider::OpenAI.slug_for(model).to_string(),
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

/// Resolve the launch OpenRouter gateway with server-side catalog enforcement.
///
/// Browser/API callers should use this path when a user selects a model slug:
/// it defaults omitted selections to the Grok free-tier model, rejects unknown
/// slugs, and enforces the workspace entitlement before returning a client.
pub fn resolve_openrouter_gateway(
    config: &Config,
    requested_slug: Option<&str>,
    entitlement: ModelEntitlement,
) -> Result<(LlmClient, &'static ModelCatalogEntry)> {
    let key = config
        .openrouter_api_key()
        .ok_or_else(|| anyhow!("OPENROUTER_API_KEY is not set"))?;
    let entry = resolve_openrouter_model(requested_slug, entitlement)?;

    Ok((LlmClient::openrouter(key), entry))
}
