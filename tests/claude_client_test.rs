use conch::claude::client::ClaudeClient;
use conch::claude::types::{ContentBlock, Message, MessagesRequest, Role, StopReason};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn messages_call_sends_correct_headers_and_parses_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-test"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_abc",
            "model": "claude-opus-4-6",
            "content": [{ "type": "text", "text": "hello back" }],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = ClaudeClient::new("sk-test", &server.uri());

    let request = MessagesRequest {
        model: "claude-opus-4-6".into(),
        max_tokens: 1024,
        system: Some("you are a test".into()),
        messages: vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: "hello".into() }],
        }],
        tools: vec![],
    };

    let response = client.messages(&request).await.unwrap();

    assert_eq!(response.id, "msg_abc");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
    match &response.content[0] {
        ContentBlock::Text { text } => assert_eq!(text, "hello back"),
        other => panic!("expected text block, got {:?}", other),
    }
}

#[tokio::test]
async fn messages_call_returns_error_on_non_2xx() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": { "type": "invalid_request_error", "message": "bad" }
        })))
        .mount(&server)
        .await;

    let client = ClaudeClient::new("sk-test", &server.uri());
    let request = MessagesRequest {
        model: "claude-opus-4-6".into(),
        max_tokens: 1024,
        system: None,
        messages: vec![],
        tools: vec![],
    };

    let err = client.messages(&request).await.unwrap_err();
    assert!(err.to_string().contains("400"));
}
