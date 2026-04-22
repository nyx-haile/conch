use crate::llm::client::LlmClient;
use crate::llm::types::{ChatRequest, Message};
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
    client: &LlmClient,
    registry: &ToolRegistry,
    config: &AgentConfig,
    user_prompt: &str,
) -> anyhow::Result<String> {
    let mut messages: Vec<Message> = vec![
        Message::system(config.system_prompt.clone()),
        Message::user(user_prompt),
    ];

    for turn in 0..config.max_turns {
        let request = ChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            max_tokens: config.max_tokens,
            tools: registry.definitions(),
        };

        let response = client.chat(&request).await?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no choices in response at turn {}", turn))?;

        let assistant_msg = choice.message.clone();
        let tool_calls = assistant_msg.tool_calls.clone().unwrap_or_default();
        messages.push(assistant_msg);

        match choice.finish_reason.as_str() {
            "stop" | "end_turn" => {
                return Ok(choice.message.content.unwrap_or_default());
            }
            "length" | "max_tokens" => {
                return Err(anyhow!("agent hit max_tokens at turn {}", turn));
            }
            "tool_calls" | "tool_use" | "function_call" => {
                if tool_calls.is_empty() {
                    return Err(anyhow!(
                        "finish_reason indicated tool_calls but none were present"
                    ));
                }
                for call in &tool_calls {
                    let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                        .unwrap_or_else(|_| serde_json::json!({ "raw": call.function.arguments }));
                    let result = match registry.call(&call.function.name, args).await {
                        Ok(output) => output,
                        Err(e) => format!("tool error: {}", e),
                    };
                    messages.push(Message::tool_result(&call.id, result));
                }
            }
            other => {
                return Err(anyhow!(
                    "unexpected finish_reason at turn {}: {}",
                    turn,
                    other
                ));
            }
        }
    }
    Err(anyhow!(
        "agent did not terminate within {} turns",
        config.max_turns
    ))
}
