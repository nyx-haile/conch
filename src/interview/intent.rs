use crate::llm::types::ToolDefinition;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;

static END_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(that'?s a wrap|end interview|we'?re done|wrap it up|end the session)\b",
    )
    .unwrap()
});

pub fn detect_end_command(final_text: &str) -> bool {
    !final_text.is_empty() && END_RE.is_match(final_text)
}

pub fn end_session_tool() -> ToolDefinition {
    ToolDefinition::function(
        "end_session",
        "Call when the interview's key angles are covered or the user signals they're done. \
         Provide a one-line reason.",
        json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Brief reason for ending now."
                }
            },
            "required": ["reason"]
        }),
    )
}
