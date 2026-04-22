use conch::llm::client::LlmClient;
use conch::llm::types::{ChatRequest, Message};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn openai_chat_call_sends_bearer_auth_and_parses_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer sk-openai-test"))
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

    let client = LlmClient::openai_with_base_url("sk-openai-test", &server.uri());

    let request = ChatRequest {
        model: "gpt-4.1-mini".into(),
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
async fn openrouter_chat_call_sends_router_headers() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer sk-or-test"))
        .and(header("http-referer", "https://github.com/conch-cli/conch"))
        .and(header("x-title", "conch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_or",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "router ok" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = LlmClient::openrouter_with_base_url("sk-or-test", &server.uri());
    let request = ChatRequest {
        model: "deepseek/deepseek-chat-v3:free".into(),
        max_tokens: 1024,
        messages: vec![Message::user("hello")],
        tools: vec![],
    };

    let response = client.chat(&request).await.unwrap();
    assert_eq!(response.choices[0].finish_reason, "stop");
}

#[tokio::test]
async fn openai_chat_call_returns_error_on_non_2xx() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": { "type": "invalid_request_error", "message": "bad" }
        })))
        .mount(&server)
        .await;

    let client = LlmClient::openai_with_base_url("sk-openai-test", &server.uri());
    let request = ChatRequest {
        model: "gpt-4.1-mini".into(),
        max_tokens: 1024,
        messages: vec![],
        tools: vec![],
    };

    let err = client.chat(&request).await.unwrap_err();
    assert!(err.to_string().contains("400"));
}
