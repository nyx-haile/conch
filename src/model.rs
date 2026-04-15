#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Haiku,
    Sonnet,
    Opus,
}

impl Model {
    /// OpenRouter model slug. These currently point at free-tier models
    /// while we iterate; swap back to `anthropic/claude-*` once we care
    /// about quality and rate limits.
    pub fn id(self) -> &'static str {
        match self {
            Self::Haiku => "meta-llama/llama-3.3-70b-instruct:free",
            Self::Sonnet => "deepseek/deepseek-chat-v3:free",
            Self::Opus => "deepseek/deepseek-r1:free",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_are_openrouter_free_slugs() {
        assert!(Model::Haiku.id().ends_with(":free"));
        assert!(Model::Sonnet.id().ends_with(":free"));
        assert!(Model::Opus.id().ends_with(":free"));
    }
}
