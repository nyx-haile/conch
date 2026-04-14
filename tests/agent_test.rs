use async_trait::async_trait;
use conch::claude::client::ClaudeClient;
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

#[tokio::test]
async fn agent_returns_text_immediately_when_stop_reason_is_end_turn() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "model": "claude-opus-4-6",
            "content": [{ "type": "text", "text": "# Brief\n\nAll done." }],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = ClaudeClient::new("sk-test", &server.uri());
    let registry = ToolRegistry::new();

    let config = AgentConfig {
        model: "claude-opus-4-6".into(),
        max_tokens: 4096,
        system_prompt: "you are a research agent".into(),
        max_turns: 10,
    };

    let brief = run_agent(&client, &registry, &config, "research howtowin")
        .await
        .unwrap();

    assert!(brief.contains("# Brief"));
    assert!(brief.contains("All done."));
}

#[tokio::test]
async fn agent_runs_tool_and_feeds_result_back() {
    let server = MockServer::start().await;

    // First call: model requests tool use.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "model": "claude-opus-4-6",
            "content": [
                { "type": "tool_use", "id": "toolu_1", "name": "fixed", "input": {} }
            ],
            "stop_reason": "tool_use"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: model produces final text after seeing tool output.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_2",
            "model": "claude-opus-4-6",
            "content": [{ "type": "text", "text": "final brief here" }],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = ClaudeClient::new("sk-test", &server.uri());
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FixedTool {
        output: "tool said hi".into(),
    }));

    let config = AgentConfig {
        model: "claude-opus-4-6".into(),
        max_tokens: 4096,
        system_prompt: "system".into(),
        max_turns: 10,
    };

    let brief = run_agent(&client, &registry, &config, "do the thing")
        .await
        .unwrap();

    assert_eq!(brief, "final brief here");
}

#[tokio::test]
async fn agent_reports_tool_error_back_to_model() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "model": "claude-opus-4-6",
            "content": [
                { "type": "tool_use", "id": "toolu_1", "name": "missing_tool", "input": {} }
            ],
            "stop_reason": "tool_use"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_2",
            "model": "claude-opus-4-6",
            "content": [{ "type": "text", "text": "recovered" }],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = ClaudeClient::new("sk-test", &server.uri());
    let registry = ToolRegistry::new();
    let config = AgentConfig {
        model: "claude-opus-4-6".into(),
        max_tokens: 4096,
        system_prompt: "system".into(),
        max_turns: 10,
    };

    let brief = run_agent(&client, &registry, &config, "do the thing")
        .await
        .unwrap();

    assert_eq!(brief, "recovered");
}
