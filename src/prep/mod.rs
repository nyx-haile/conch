pub mod agent;
pub mod brief_prompt;
pub mod calc_tool;
pub mod github_tool;
pub mod local_fs_tool;
pub mod tools;

use anyhow::Context;
use crate::claude::client::ClaudeClient;
use crate::config::Config;
use crate::model::Model;
use crate::prep::agent::{run_agent, AgentConfig};
use crate::prep::calc_tool::CalcTool;
use crate::prep::github_tool::GithubTool;
use crate::prep::local_fs_tool::LocalFsTool;
use crate::prep::tools::ToolRegistry;

pub async fn run_prep(config: &Config, model: Model, topic: &str) -> anyhow::Result<String> {
    let api_key = config
        .anthropic_api_key()
        .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY is not set"))?;

    let client = ClaudeClient::anthropic(api_key);

    let cwd = std::env::current_dir().context("reading current working directory")?;

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(CalcTool));
    registry.register(Box::new(GithubTool::github(
        config.github_token().map(|t| t.to_string()),
    )));
    registry.register(Box::new(LocalFsTool::new(cwd)));

    let agent_config = AgentConfig {
        model: model.id().to_string(),
        max_tokens: 4096,
        system_prompt: brief_prompt::SYSTEM_PROMPT.to_string(),
        max_turns: 12,
    };

    let user_prompt = format!("Topic: {}\n\nProduce the interview brief now.", topic);
    run_agent(&client, &registry, &agent_config, &user_prompt).await
}
