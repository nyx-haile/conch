use conch::llm::types::{ChatResponse, FunctionCall, Message, Role, ToolCall, ToolDefinition};
use serde_json::json;

#[test]
fn system_message_serializes_with_role_and_content() {
    let msg = Message::system("you are helpful");
    let v = serde_json::to_value(&msg).unwrap();
    assert_eq!(v, json!({ "role": "system", "content": "you are helpful" }));
}

#[test]
fn user_message_serializes_with_role_and_content() {
    let msg = Message::user("hello");
    let v = serde_json::to_value(&msg).unwrap();
    assert_eq!(v, json!({ "role": "user", "content": "hello" }));
}

#[test]
fn tool_result_message_serializes_with_tool_call_id() {
    let msg = Message::tool_result("call_1", "output here");
    let v = serde_json::to_value(&msg).unwrap();
    assert_eq!(
        v,
        json!({
            "role": "tool",
            "content": "output here",
            "tool_call_id": "call_1"
        })
    );
}

#[test]
fn tool_definition_uses_function_wrapper() {
    let def = ToolDefinition::function(
        "calc",
        "evaluate",
        json!({ "type": "object", "properties": {} }),
    );
    let v = serde_json::to_value(&def).unwrap();
    assert_eq!(v["type"], "function");
    assert_eq!(v["function"]["name"], "calc");
    assert_eq!(v["function"]["description"], "evaluate");
    assert!(v["function"]["parameters"].is_object());
}

#[test]
fn chat_response_parses_tool_call_shape() {
    let raw = json!({
        "id": "chatcmpl-1",
        "model": "anthropic/claude-haiku-4.5",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_abc",
                    "type": "function",
                    "function": {
                        "name": "github",
                        "arguments": "{\"action\":\"get_repo\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });

    let resp: ChatResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(resp.choices.len(), 1);
    assert_eq!(resp.choices[0].finish_reason, "tool_calls");
    assert_eq!(resp.choices[0].message.role, Role::Assistant);
    let calls = resp.choices[0].message.tool_calls.as_ref().unwrap();
    assert_eq!(calls[0].id, "call_abc");
    assert_eq!(calls[0].function.name, "github");
}

#[test]
fn chat_response_parses_plain_text_finish() {
    let raw = json!({
        "id": "chatcmpl-2",
        "choices": [{
            "message": { "role": "assistant", "content": "final answer" },
            "finish_reason": "stop"
        }]
    });
    let resp: ChatResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(
        resp.choices[0].message.content.as_deref(),
        Some("final answer")
    );
    assert_eq!(resp.choices[0].finish_reason, "stop");
}

#[test]
fn tool_call_roundtrip() {
    let call = ToolCall {
        id: "c1".into(),
        call_type: "function".into(),
        function: FunctionCall {
            name: "calc".into(),
            arguments: "{\"expression\":\"1+1\"}".into(),
        },
    };
    let v = serde_json::to_value(&call).unwrap();
    assert_eq!(v["type"], "function");
    assert_eq!(v["function"]["name"], "calc");
    let back: ToolCall = serde_json::from_value(v).unwrap();
    assert_eq!(back, call);
}
