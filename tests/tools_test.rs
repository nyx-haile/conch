use async_trait::async_trait;
use conch::prep::tools::{Tool, ToolRegistry};
use serde_json::{json, Value};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }
    fn description(&self) -> &'static str {
        "Echo the provided text"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    async fn call(&self, input: Value) -> anyhow::Result<String> {
        let text = input["text"].as_str().unwrap_or("").to_string();
        Ok(text)
    }
}

#[tokio::test]
async fn registry_dispatches_to_registered_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoTool));

    let result = registry
        .call("echo", json!({ "text": "hi" }))
        .await
        .unwrap();
    assert_eq!(result, "hi");
}

#[tokio::test]
async fn registry_returns_error_for_unknown_tool() {
    let registry = ToolRegistry::new();
    let err = registry.call("missing", json!({})).await.unwrap_err();
    assert!(err.to_string().contains("missing"));
}

#[test]
fn registry_exposes_tool_definitions() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoTool));

    let defs = registry.definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].function.name, "echo");
    assert_eq!(defs[0].tool_type, "function");
}
