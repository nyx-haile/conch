use crate::prep::tools::Tool;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct CalcTool;

#[async_trait]
impl Tool for CalcTool {
    fn name(&self) -> &'static str {
        "calculator"
    }

    fn description(&self) -> &'static str {
        "Evaluate a numeric expression. Supports +, -, *, /, parentheses, and basic functions (sqrt, pow, sin, cos)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "A numeric expression like '2 + 3 * 4' or 'sqrt(16)'"
                }
            },
            "required": ["expression"]
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<String> {
        let expr = input
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'expression' string"))?;
        let value = meval::eval_str(expr).context("failed to evaluate expression")?;
        if value.fract() == 0.0 {
            Ok(format!("{}", value as i64))
        } else {
            Ok(format!("{}", value))
        }
    }
}
