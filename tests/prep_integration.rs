use conch::claude::client::ClaudeClient;
use conch::prep::agent::{run_agent, AgentConfig};
use conch::prep::github_tool::GithubTool;
use conch::prep::tools::ToolRegistry;
use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn prep_produces_brief_using_github_tool() {
    let claude_server = MockServer::start().await;
    let github_server = MockServer::start().await;

    // GitHub responses.
    Mock::given(method("GET"))
        .and(path("/repos/user/howtowin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "howtowin",
            "description": "A tool for winning",
            "language": "Rust",
            "stargazers_count": 3
        })))
        .mount(&github_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/user/howtowin/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&github_server)
        .await;

    // Claude: first request wants get_repo, second writes the brief.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "model": "claude-opus-4-6",
            "content": [
                {
                    "type": "tool_use",
                    "id": "t1",
                    "name": "github",
                    "input": { "action": "get_repo", "owner": "user", "repo": "howtowin" }
                }
            ],
            "stop_reason": "tool_use"
        })))
        .up_to_n_times(1)
        .mount(&claude_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_2",
            "model": "claude-opus-4-6",
            "content": [{
                "type": "text",
                "text": "# Interview Brief: howtowin\n\n## Summary\nA tool for winning, written in Rust.\n"
            }],
            "stop_reason": "end_turn"
        })))
        .mount(&claude_server)
        .await;

    let client = ClaudeClient::new("sk-test", &claude_server.uri());
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(GithubTool::new(None, github_server.uri())));

    let config = AgentConfig {
        model: "claude-opus-4-6".into(),
        max_tokens: 4096,
        system_prompt: "research agent".into(),
        max_turns: 10,
    };

    let brief = run_agent(&client, &registry, &config, "https://github.com/user/howtowin")
        .await
        .unwrap();

    assert!(brief.contains("# Interview Brief: howtowin"));
    assert!(brief.contains("Rust"));
}
