use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(date: &str, topic: &str) -> Self {
        let slug = slugify(topic);
        let slug = slug.chars().take(60).collect::<String>();
        let slug = slug.trim_end_matches('-').to_string();
        Self(format!("{}-{}", date, slug))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn directory(&self, sessions_root: &Path) -> PathBuf {
        sessions_root.join(&self.0)
    }
}

fn slugify(input: &str) -> String {
    // Strip URL scheme (e.g. "https://", "http://") before slugifying
    let input = if let Some(rest) = input.strip_prefix("https://") {
        rest
    } else if let Some(rest) = input.strip_prefix("http://") {
        rest
    } else {
        input
    };

    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for c in input.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
