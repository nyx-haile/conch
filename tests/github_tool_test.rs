use conch::prep::github_tool::parse_repo_ref;

#[test]
fn parses_https_url() {
    let r = parse_repo_ref("https://github.com/user/proj").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn parses_url_with_trailing_slash_and_git_suffix() {
    let r = parse_repo_ref("https://github.com/user/proj.git/").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn parses_short_form() {
    let r = parse_repo_ref("user/proj").unwrap();
    assert_eq!(r, ("user".to_string(), "proj".to_string()));
}

#[test]
fn rejects_non_github_input() {
    assert!(parse_repo_ref("howtowin.lol").is_none());
    assert!(parse_repo_ref("just some freeform text").is_none());
}

use conch::prep::github_tool::GithubTool;
use conch::prep::tools::Tool;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn github_tool_fetches_repo_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/user/proj"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "proj",
            "full_name": "user/proj",
            "description": "a test project",
            "language": "Rust",
            "stargazers_count": 42,
            "html_url": "https://github.com/user/proj",
            "default_branch": "main"
        })))
        .mount(&server)
        .await;

    let tool = GithubTool::new(None, server.uri());
    let result = tool
        .call(json!({ "action": "get_repo", "owner": "user", "repo": "proj" }))
        .await
        .unwrap();

    assert!(result.contains("a test project"));
    assert!(result.contains("Rust"));
    assert!(result.contains("42"));
}

#[tokio::test]
async fn github_tool_fetches_readme() {
    let server = MockServer::start().await;
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode("# Hello\n\nWorld");
    Mock::given(method("GET"))
        .and(path("/repos/user/proj/readme"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "README.md",
            "content": encoded,
            "encoding": "base64"
        })))
        .mount(&server)
        .await;

    let tool = GithubTool::new(None, server.uri());
    let result = tool
        .call(json!({ "action": "get_readme", "owner": "user", "repo": "proj" }))
        .await
        .unwrap();

    assert!(result.contains("# Hello"));
    assert!(result.contains("World"));
}

#[tokio::test]
async fn github_tool_fetches_recent_commits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/user/proj/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "sha": "abc123",
                "commit": {
                    "message": "initial commit",
                    "author": { "name": "Nyx", "date": "2026-04-01T12:00:00Z" }
                }
            },
            {
                "sha": "def456",
                "commit": {
                    "message": "add feature X",
                    "author": { "name": "Nyx", "date": "2026-04-02T12:00:00Z" }
                }
            }
        ])))
        .mount(&server)
        .await;

    let tool = GithubTool::new(None, server.uri());
    let result = tool
        .call(json!({ "action": "list_commits", "owner": "user", "repo": "proj", "limit": 10 }))
        .await
        .unwrap();

    assert!(result.contains("initial commit"));
    assert!(result.contains("add feature X"));
}

#[tokio::test]
async fn github_tool_unknown_action_errors() {
    let server = MockServer::start().await;
    let tool = GithubTool::new(None, server.uri());
    let err = tool
        .call(json!({ "action": "bogus", "owner": "u", "repo": "r" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("action"));
}
