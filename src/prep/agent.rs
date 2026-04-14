use crate::claude::client::ClaudeClient;
use crate::claude::types::{
    ContentBlock, Message, MessagesRequest, Role, StopReason,
};
use crate::prep::tools::ToolRegistry;
use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub max_turns: usize,
}

pub async fn run_agent(
    client: &ClaudeClient,
    registry: &ToolRegistry,
    config: &AgentConfig,
    user_prompt: &str,
) -> anyhow::Result<String> {
    let mut messages: Vec<Message> = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: user_prompt.to_string(),
        }],
    }];

    for turn in 0..config.max_turns {
        let request = MessagesRequest {
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            system: Some(config.system_prompt.clone()),
            messages: messages.clone(),
            tools: registry.definitions(),
        };

        let response = client.messages(&request).await?;

        // Append assistant's turn to history.
        messages.push(Message {
            role: Role::Assistant,
            content: response.content.clone(),
        });

        match response.stop_reason {
            StopReason::EndTurn | StopReason::StopSequence => {
                return Ok(extract_text(&response.content));
            }
            StopReason::MaxTokens => {
                return Err(anyhow!("agent hit max_tokens at turn {}", turn));
            }
            StopReason::ToolUse => {
                // Handled in the next task.
                return Err(anyhow!("tool use not yet supported"));
            }
        }
    }
    Err(anyhow!("agent did not terminate within {} turns", config.max_turns))
}

fn extract_text(content: &[ContentBlock]) -> String {
    let mut out = String::new();
    for block in content {
        if let ContentBlock::Text { text } = block {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }
    out
}
