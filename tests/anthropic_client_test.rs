use conch::llm::client::LlmClient;
use conch::llm::types::{ChatRequest, Message};
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn anthropic_client_sends_x_api_key_and_translates_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-test"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(body_partial_json(json!({
            "model": "claude-haiku-4-5-20251001",
            "system": "you are a research agent",
            "messages": [
                { "role": "user", "content": [{ "type": "text", "text": "hello" }] }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_abc",
            "type": "message",
            "role": "assistant",
            "model": "claude-haiku-4-5-20251001",
            "content": [{ "type": "text", "text": "hello back" }],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = LlmClient::anthropic_with_base_url("sk-ant-test", &server.uri());

    let request = ChatRequest {
        model: "claude-haiku-4-5-20251001".into(),
        max_tokens: 1024,
        messages: vec![
            Message::system("you are a research agent"),
            Message::user("hello"),
        ],
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
async fn anthropic_client_translates_tool_use_to_tool_calls() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_abc",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-6",
            "content": [
                { "type": "tool_use", "id": "toolu_1", "name": "calculator",
                  "input": { "expression": "2+2" } }
            ],
            "stop_reason": "tool_use"
        })))
        .mount(&server)
        .await;

    let client = LlmClient::anthropic_with_base_url("sk-ant-test", &server.uri());
    let request = ChatRequest {
        model: "claude-sonnet-4-6".into(),
        max_tokens: 1024,
        messages: vec![Message::user("what is 2+2")],
        tools: vec![],
    };

    let response = client.chat(&request).await.unwrap();
    assert_eq!(response.choices[0].finish_reason, "tool_calls");
    let calls = response.choices[0]
        .message
        .tool_calls
        .as_ref()
        .expect("expected tool_calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "toolu_1");
    assert_eq!(calls[0].function.name, "calculator");
    let parsed: serde_json::Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
    assert_eq!(parsed["expression"], "2+2");
}

#[tokio::test]
async fn anthropic_client_returns_error_on_non_2xx() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": { "type": "authentication_error", "message": "bad key" }
        })))
        .mount(&server)
        .await;

    let client = LlmClient::anthropic_with_base_url("sk-ant-test", &server.uri());
    let request = ChatRequest {
        model: "claude-opus-4-6".into(),
        max_tokens: 1024,
        messages: vec![Message::user("hi")],
        tools: vec![],
    };

    let err = client.chat(&request).await.unwrap_err();
    assert!(err.to_string().contains("401"));
}
