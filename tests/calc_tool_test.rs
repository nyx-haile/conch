use conch::prep::calc_tool::CalcTool;
use conch::prep::tools::Tool;
use serde_json::json;

#[tokio::test]
async fn calc_evaluates_simple_arithmetic() {
    let tool = CalcTool;
    let result = tool
        .call(json!({ "expression": "2 + 3 * 4" }))
        .await
        .unwrap();
    assert_eq!(result, "14");
}

#[tokio::test]
async fn calc_handles_float_result() {
    let tool = CalcTool;
    let result = tool.call(json!({ "expression": "10 / 4" })).await.unwrap();
    assert_eq!(result, "2.5");
}

#[tokio::test]
async fn calc_returns_error_for_invalid_expression() {
    let tool = CalcTool;
    let err = tool
        .call(json!({ "expression": "not-math" }))
        .await
        .unwrap_err();
    assert!(err.to_string().to_lowercase().contains("expression"));
}
