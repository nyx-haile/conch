use anyhow::{Context, Result};
use conch::llm::types::{ChatRequest, Message};
use conch::model::Model;
use conch::provider::Provider;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    anyhow::ensure!(
        std::env::var_os("CONCH_TEST_SCRIPTED_LLM").is_none(),
        "CONCH_TEST_SCRIPTED_LLM must be unset for real OpenAI smoke"
    );
    let config = conch::config::Config::load()?;
    let provider = config.provider()?;
    anyhow::ensure!(
        provider == Provider::OpenAI,
        "CONCH_PROVIDER must be openai for this smoke"
    );
    let (client, model) = conch::llm::resolve(&config, provider, Model::Haiku)?;
    let response = client
        .chat(&ChatRequest {
            model: model.clone(),
            messages: vec![Message::user("Reply with exactly: conch-openai-smoke-ok")],
            max_tokens: 16,
            tools: vec![],
        })
        .await
        .context("calling OpenAI chat completions")?;
    let content = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .unwrap_or_default();
    anyhow::ensure!(
        content
            .to_ascii_lowercase()
            .contains("conch-openai-smoke-ok"),
        "unexpected OpenAI response: {content:?}"
    );
    println!("model: {model}");
    println!("response: {content}");
    Ok(())
}
