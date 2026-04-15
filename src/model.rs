#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Haiku,
    Sonnet,
    Opus,
}

impl Model {
    pub fn id(self) -> &'static str {
        match self {
            Self::Haiku => "claude-haiku-4-5-20251001",
            Self::Sonnet => "claude-sonnet-4-6",
            Self::Opus => "claude-opus-4-6",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_match_expected_values() {
        assert_eq!(Model::Haiku.id(), "claude-haiku-4-5-20251001");
        assert_eq!(Model::Sonnet.id(), "claude-sonnet-4-6");
        assert_eq!(Model::Opus.id(), "claude-opus-4-6");
    }
}
