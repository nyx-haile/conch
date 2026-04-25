use crate::llm::catalog::{openrouter_slug_for_depth, OPENROUTER_FREE_TIER_DEFAULT_SLUG};
use crate::model::Model;
use anyhow::{anyhow, Result};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    OpenAI,
    OpenRouter,
    DeepSeek,
    Meta,
    Google,
}

impl Provider {
    pub fn slug_for(self, model: Model) -> &'static str {
        match (self, model) {
            (Self::Anthropic, Model::Haiku) => "anthropic/claude-haiku-4.5",
            (Self::Anthropic, Model::Sonnet) => "anthropic/claude-sonnet-4.6",
            (Self::Anthropic, Model::Opus) => "anthropic/claude-opus-4.6",

            (Self::OpenAI, Model::Haiku) => "gpt-4.1-mini",
            (Self::OpenAI, Model::Sonnet) => "gpt-4.1",
            (Self::OpenAI, Model::Opus) => "gpt-5.1",

            (Self::OpenRouter, model) => openrouter_slug_for_depth(model),

            (Self::DeepSeek, Model::Haiku) => "deepseek/deepseek-chat-v3:free",
            (Self::DeepSeek, Model::Sonnet) => "deepseek/deepseek-chat-v3:free",
            (Self::DeepSeek, Model::Opus) => "deepseek/deepseek-r1:free",

            (Self::Meta, Model::Haiku) => "meta-llama/llama-3.3-70b-instruct:free",
            (Self::Meta, Model::Sonnet) => "meta-llama/llama-3.3-70b-instruct:free",
            (Self::Meta, Model::Opus) => "meta-llama/llama-3.3-70b-instruct:free",

            (Self::Google, Model::Haiku) => "google/gemini-2.0-flash-exp:free",
            (Self::Google, Model::Sonnet) => "google/gemini-2.0-flash-exp:free",
            (Self::Google, Model::Opus) => "google/gemini-2.5-pro-preview",
        }
    }
}

impl FromStr for Provider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic),
            "openai" => Ok(Self::OpenAI),
            "openrouter" | "grok" | "xai" | "x-ai" => Ok(Self::OpenRouter),
            "deepseek" => Ok(Self::DeepSeek),
            "meta" | "llama" => Ok(Self::Meta),
            "google" | "gemini" => Ok(Self::Google),
            other => Err(anyhow!(
                "unknown provider {:?}; valid values: anthropic, openai, openrouter, grok, deepseek, meta, google",
                other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_providers() {
        assert_eq!(
            "anthropic".parse::<Provider>().unwrap(),
            Provider::Anthropic
        );
        assert_eq!(
            "ANTHROPIC".parse::<Provider>().unwrap(),
            Provider::Anthropic
        );
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!(
            "openrouter".parse::<Provider>().unwrap(),
            Provider::OpenRouter
        );
        assert_eq!("grok".parse::<Provider>().unwrap(), Provider::OpenRouter);
        assert_eq!("deepseek".parse::<Provider>().unwrap(), Provider::DeepSeek);
        assert_eq!("meta".parse::<Provider>().unwrap(), Provider::Meta);
        assert_eq!("llama".parse::<Provider>().unwrap(), Provider::Meta);
        assert_eq!("google".parse::<Provider>().unwrap(), Provider::Google);
    }

    #[test]
    fn errors_on_unknown_provider() {
        let err = "mistral".parse::<Provider>().unwrap_err();
        assert!(err.to_string().contains("unknown provider"));
    }

    #[test]
    fn anthropic_slugs_point_at_claude_4x() {
        assert_eq!(
            Provider::Anthropic.slug_for(Model::Haiku),
            "anthropic/claude-haiku-4.5"
        );
        assert_eq!(
            Provider::Anthropic.slug_for(Model::Sonnet),
            "anthropic/claude-sonnet-4.6"
        );
        assert_eq!(
            Provider::Anthropic.slug_for(Model::Opus),
            "anthropic/claude-opus-4.6"
        );
    }

    #[test]
    fn openrouter_gateway_defaults_to_grok_for_fast_depth() {
        assert_eq!(
            Provider::OpenRouter.slug_for(Model::Haiku),
            OPENROUTER_FREE_TIER_DEFAULT_SLUG
        );
    }

    #[test]
    fn openai_slugs_point_at_native_models() {
        assert_eq!(Provider::OpenAI.slug_for(Model::Haiku), "gpt-4.1-mini");
        assert_eq!(Provider::OpenAI.slug_for(Model::Sonnet), "gpt-4.1");
        assert_eq!(Provider::OpenAI.slug_for(Model::Opus), "gpt-5.1");
    }
}
