use conch::claude::types::{ContentBlock, Message, Role, ToolDefinition, ToolUseBlock};
use serde_json::json;

#[test]
fn user_text_message_serializes_to_expected_json() {
    let msg = Message {
        role: Role::User,
        content: vec![ContentBlock::Text { text: "hello".into() }],
    };

    let actual = serde_json::to_value(&msg).unwrap();
    assert_eq!(
        actual,
        json!({
            "role": "user",
            "content": [{ "type": "text", "text": "hello" }]
        })
    );
}

#[test]
fn assistant_tool_use_message_serializes_to_expected_json() {
    let msg = Message {
        role: Role::Assistant,
        content: vec![ToolUseBlock {
            id: "toolu_1".into(),
            name: "github".into(),
            input: json!({ "action": "get_readme", "repo": "user/proj" }),
        }.into()],
    };

    let actual = serde_json::to_value(&msg).unwrap();
    assert_eq!(
        actual,
        json!({
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": "toolu_1",
                "name": "github",
                "input": { "action": "get_readme", "repo": "user/proj" }
            }]
        })
    );
}

#[test]
fn tool_result_message_serializes_to_expected_json() {
    let msg = Message {
        role: Role::User,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: "toolu_1".into(),
            content: "# Hello\n\nWorld".into(),
            is_error: false,
        }],
    };

    let actual = serde_json::to_value(&msg).unwrap();
    assert_eq!(
        actual,
        json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": "# Hello\n\nWorld",
                "is_error": false
            }]
        })
    );
}

#[test]
fn tool_definition_serializes_to_expected_json() {
    let tool = ToolDefinition {
        name: "calc".into(),
        description: "Evaluate arithmetic".into(),
        input_schema: json!({
            "type": "object",
            "properties": { "expr": { "type": "string" } },
            "required": ["expr"]
        }),
    };

    let actual = serde_json::to_value(&tool).unwrap();
    assert_eq!(actual["name"], "calc");
    assert_eq!(actual["description"], "Evaluate arithmetic");
    assert!(actual["input_schema"].is_object());
}
