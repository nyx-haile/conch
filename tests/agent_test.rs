use conch::claude::client::ClaudeClient;
use conch::prep::agent::{run_agent, AgentConfig};
use conch::prep::tools::ToolRegistry;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
