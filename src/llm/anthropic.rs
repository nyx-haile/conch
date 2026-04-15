use crate::llm::types::{
    ChatRequest, ChatResponse, Choice, FunctionCall, Message, Role, ToolCall,
};
use crate::model::Model;
use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn native_model_id(model: Model) -> &'static str {
    match model {
        Model::Haiku => "claude-haiku-4-5-20251001",
        Model::Sonnet => "claude-sonnet-4-6",
        Model::Opus => "claude-opus-4-6",
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let native = to_native_request(request)?;
        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&native)
            .send()
            .await
            .context("sending anthropic messages request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("anthropic api returned {}: {}", status, body));
        }

        let native_response: NativeResponse = response
            .json()
            .await
            .context("parsing anthropic messages response")?;
        Ok(from_native_response(native_response))
    }
}

fn to_native_request(req: &ChatRequest) -> anyhow::Result<NativeRequest> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut native_messages: Vec<NativeMessage> = Vec::new();

    for msg in &req.messages {
        match msg.role {
            Role::System => {
                if let Some(text) = &msg.content {
                    system_parts.push(text.clone());
                }
            }
            Role::User => {
                let blocks = vec![NativeBlock::Text {
                    text: msg.content.clone().unwrap_or_default(),
                }];
                native_messages.push(NativeMessage {
                    role: "user".to_string(),
                    content: blocks,
                });
            }
            Role::Assistant => {
                let mut blocks: Vec<NativeBlock> = Vec::new();
                if let Some(text) = &msg.content {
                    if !text.is_empty() {
                        blocks.push(NativeBlock::Text { text: text.clone() });
                    }
                }
                if let Some(calls) = &msg.tool_calls {
                    for call in calls {
                        let input: Value = serde_json::from_str(&call.function.arguments)
                            .unwrap_or_else(|_| {
                                serde_json::json!({ "raw": call.function.arguments })
                            });
                        blocks.push(NativeBlock::ToolUse {
                            id: call.id.clone(),
                            name: call.function.name.clone(),
                            input,
                        });
                    }
                }
                native_messages.push(NativeMessage {
                    role: "assistant".to_string(),
                    content: blocks,
                });
            }
            Role::Tool => {
                let tool_use_id = msg
                    .tool_call_id
                    .clone()
                    .ok_or_else(|| anyhow!("tool message missing tool_call_id"))?;
                let block = NativeBlock::ToolResult {
                    tool_use_id,
                    content: msg.content.clone().unwrap_or_default(),
                };
                // Fold consecutive tool results into a single user message.
                if let Some(last) = native_messages.last_mut() {
                    if last.role == "user"
                        && last
                            .content
                            .iter()
                            .all(|b| matches!(b, NativeBlock::ToolResult { .. }))
                    {
                        last.content.push(block);
                        continue;
                    }
                }
                native_messages.push(NativeMessage {
                    role: "user".to_string(),
                    content: vec![block],
                });
            }
        }
    }

    let tools = req
        .tools
        .iter()
        .map(|t| NativeTool {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            input_schema: t.function.parameters.clone(),
        })
        .collect();

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };

    Ok(NativeRequest {
        model: req.model.clone(),
        max_tokens: req.max_tokens,
        system,
        messages: native_messages,
        tools,
    })
}

fn from_native_response(resp: NativeResponse) -> ChatResponse {
    let mut text_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for block in resp.content {
        match block {
            NativeBlock::Text { text } => text_parts.push(text),
            NativeBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id,
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name,
                        arguments: serde_json::to_string(&input).unwrap_or_else(|_| "{}".into()),
                    },
                });
            }
            NativeBlock::ToolResult { .. } => {}
        }
    }

    let content = if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join("\n"))
    };
    let tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    let finish_reason = match resp.stop_reason.as_deref() {
        Some("end_turn") => "stop".to_string(),
        Some("tool_use") => "tool_calls".to_string(),
        Some("max_tokens") => "length".to_string(),
        Some(other) => other.to_string(),
        None => "stop".to_string(),
    };

    ChatResponse {
        id: Some(resp.id),
        model: Some(resp.model),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                tool_calls,
                tool_call_id: None,
            },
            finish_reason,
        }],
    }
}

#[derive(Debug, Serialize)]
struct NativeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<NativeMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<NativeTool>,
}

#[derive(Debug, Serialize)]
struct NativeMessage {
    role: String,
    content: Vec<NativeBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum NativeBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct NativeTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct NativeResponse {
    id: String,
    model: String,
    content: Vec<NativeBlock>,
    stop_reason: Option<String>,
}
