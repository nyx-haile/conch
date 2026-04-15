use async_trait::async_trait;
use conch::llm::client::LlmClient;
use conch::prep::agent::{run_agent, AgentConfig};
use conch::prep::tools::{Tool, ToolRegistry};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct FixedTool {
    output: String,
}

#[async_trait]
impl Tool for FixedTool {
    fn name(&self) -> &'static str {
        "fixed"
    }
    fn description(&self) -> &'static str {
        "returns a fixed output"
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn call(&self, _input: serde_json::Value) -> anyhow::Result<String> {
        Ok(self.output.clone())
    }
}

fn base_config() -> AgentConfig {
    AgentConfig {
        model: "some/model".into(),
        max_tokens: 4096,
        system_prompt: "you are a research agent".into(),
        max_turns: 10,
    }
}

#[tokio::test]
async fn agent_returns_text_immediately_when_finish_reason_is_stop() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "# Brief\n\nAll done." },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = LlmClient::new("sk-or-test", &server.uri());
    let registry = ToolRegistry::new();

    let brief = run_agent(&client, &registry, &base_config(), "research howtowin")
        .await
        .unwrap();

    assert!(brief.contains("# Brief"));
    assert!(brief.contains("All done."));
}

#[tokio::test]
async fn agent_runs_tool_and_feeds_result_back() {
    let server = MockServer::start().await;

    // Turn 1: model requests tool call.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "fixed", "arguments": "{}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Turn 2: model produces final text.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "final brief here" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = LlmClient::new("sk-or-test", &server.uri());
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FixedTool {
        output: "tool said hi".into(),
    }));

    let brief = run_agent(&client, &registry, &base_config(), "do the thing")
        .await
        .unwrap();

    assert_eq!(brief, "final brief here");
}

#[tokio::test]
async fn agent_reports_tool_error_back_to_model() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "missing_tool", "arguments": "{}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "recovered" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = LlmClient::new("sk-or-test", &server.uri());
    let registry = ToolRegistry::new();

    let brief = run_agent(&client, &registry, &base_config(), "do the thing")
        .await
        .unwrap();

    assert_eq!(brief, "recovered");
}
