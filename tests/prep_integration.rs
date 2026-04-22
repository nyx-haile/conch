use conch::llm::client::LlmClient;
use conch::prep::agent::{run_agent, AgentConfig};
use conch::prep::github_tool::GithubTool;
use conch::prep::tools::ToolRegistry;
use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn prep_produces_brief_using_github_tool() {
    let llm_server = MockServer::start().await;
    let github_server = MockServer::start().await;

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

    // Turn 1: model requests a github tool call.
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
                        "function": {
                            "name": "github",
                            "arguments": "{\"action\":\"get_repo\",\"owner\":\"user\",\"repo\":\"howtowin\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })))
        .up_to_n_times(1)
        .mount(&llm_server)
        .await;

    // Turn 2: model writes the final brief.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "# Interview Brief: howtowin\n\n## Summary\nA tool for winning, written in Rust.\n"
                },
                "finish_reason": "stop"
            }]
        })))
        .mount(&llm_server)
        .await;

    let client = LlmClient::new("sk-or-test", &llm_server.uri());
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(GithubTool::new(None, github_server.uri())));

    let config = AgentConfig {
        model: "some/model".into(),
        max_tokens: 4096,
        system_prompt: "research agent".into(),
        max_turns: 10,
    };

    let brief = run_agent(
        &client,
        &registry,
        &config,
        "https://github.com/user/howtowin",
    )
    .await
    .unwrap();

    assert!(brief.contains("# Interview Brief: howtowin"));
    assert!(brief.contains("Rust"));
}
