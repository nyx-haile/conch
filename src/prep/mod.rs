pub mod agent;
pub mod brief_prompt;
pub mod calc_tool;
pub mod github_tool;
pub mod local_fs_tool;
pub mod tools;

use crate::config::Config;
use crate::llm::client::LlmClient;
use crate::prep::agent::{run_agent, AgentConfig};
use crate::prep::calc_tool::CalcTool;
use crate::prep::github_tool::GithubTool;
use crate::prep::local_fs_tool::LocalFsTool;
use crate::prep::tools::ToolRegistry;
use anyhow::Context;

pub async fn run_prep(
    config: &Config,
    client: &LlmClient,
    model_slug: &str,
    topic: &str,
) -> anyhow::Result<String> {
    let cwd = std::env::current_dir().context("reading current working directory")?;

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(CalcTool));
    registry.register(Box::new(GithubTool::github(
        config.github_token().map(|t| t.to_string()),
    )));
    registry.register(Box::new(LocalFsTool::new(cwd.clone())));

    let agent_config = AgentConfig {
        model: model_slug.to_string(),
        max_tokens: 4096,
        system_prompt: brief_prompt::SYSTEM_PROMPT.to_string(),
        max_turns: 12,
    };

    let user_prompt = build_user_prompt(topic, &cwd);
    run_agent(client, &registry, &agent_config, &user_prompt).await
}

fn build_user_prompt(topic: &str, cwd: &std::path::Path) -> String {
    let mut prompt = format!("Topic: {}", topic);
    if let Some(repo_name) = cwd.file_name().and_then(|name| name.to_str()) {
        if repo_name.eq_ignore_ascii_case(topic.trim()) {
            prompt.push_str(&format!(
                "\n\nThe topic exactly matches the current working directory name ({repo_name}). \
Treat it as the local checked-out project first. Start with local_fs (tree, README, and key source files) before assuming it refers to an external GitHub repository."
            ));
        }
    }
    prompt.push_str("\n\nProduce the interview brief now.");
    prompt
}

#[cfg(test)]
mod tests {
    use super::build_user_prompt;
    use std::path::Path;

    #[test]
    fn prep_prompt_biases_to_local_repo_when_topic_matches_checkout_name() {
        let prompt = build_user_prompt("conch", Path::new("/tmp/conch"));
        assert!(prompt.contains("Topic: conch"));
        assert!(prompt.contains("Treat it as the local checked-out project first."));
        assert!(prompt.contains("Start with local_fs"));
    }

    #[test]
    fn prep_prompt_skips_local_repo_hint_for_nonmatching_topic() {
        let prompt = build_user_prompt("some other topic", Path::new("/tmp/conch"));
        assert!(prompt.contains("Topic: some other topic"));
        assert!(!prompt.contains("local checked-out project first"));
    }
}
