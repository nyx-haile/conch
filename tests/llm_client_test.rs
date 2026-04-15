use conch::llm::client::LlmClient;
use conch::llm::types::{ChatRequest, Message};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn chat_call_sends_bearer_auth_and_parses_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer sk-or-test"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_abc",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "hello back" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = LlmClient::new("sk-or-test", &server.uri());

    let request = ChatRequest {
        model: "some/model".into(),
        max_tokens: 1024,
        messages: vec![Message::user("hello")],
        tools: vec![],
    };

    let response = client.chat(&request).await.unwrap();
    assert_eq!(response.choices[0].finish_reason, "stop");
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some("hello back")
    );
}

#[tokio::test]
async fn chat_call_returns_error_on_non_2xx() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": { "type": "invalid_request_error", "message": "bad" }
        })))
        .mount(&server)
        .await;

    let client = LlmClient::new("sk-or-test", &server.uri());
    let request = ChatRequest {
        model: "some/model".into(),
        max_tokens: 1024,
        messages: vec![],
        tools: vec![],
    };

    let err = client.chat(&request).await.unwrap_err();
    assert!(err.to_string().contains("400"));
}
