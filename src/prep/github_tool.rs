use crate::prep::tools::Tool;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

pub fn parse_repo_ref(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    let stripped = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let parts: Vec<&str> = stripped.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0];
    let repo = parts[1];
    if owner.is_empty()
        || repo.is_empty()
        || owner.contains(' ')
        || repo.contains(' ')
        || !owner.chars().any(|c| c.is_ascii_alphanumeric())
        || !repo.chars().any(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

pub struct GithubTool {
    http: Client,
    base_url: String,
    token: Option<String>,
}

impl GithubTool {
    pub fn new(token: Option<String>, base_url: String) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    pub fn github(token: Option<String>) -> Self {
        Self::new(token, "https://api.github.com".to_string())
    }

    async fn get_json(&self, path: &str) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .http
            .get(&url)
            .header("User-Agent", "conch-cli")
            .header("Accept", "application/vnd.github+json");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context("github request")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("github api returned {}: {}", status, body));
        }
        resp.json::<Value>().await.context("github json")
    }
}

#[async_trait]
impl Tool for GithubTool {
    fn name(&self) -> &'static str {
        "github"
    }

    fn description(&self) -> &'static str {
        "Look up information from GitHub. Actions: \
         get_repo (metadata), get_readme (README content), \
         list_commits (recent commits), list_user_repos (other repos under an owner)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get_repo", "get_readme", "list_commits", "list_user_repos"]
                },
                "owner": { "type": "string" },
                "repo": { "type": "string" },
                "limit": { "type": "integer", "default": 10 }
            },
            "required": ["action", "owner"]
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<String> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'action'"))?;
        let owner = input
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'owner'"))?;
        let repo = input.get("repo").and_then(|v| v.as_str());
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);

        match action {
            "get_repo" => {
                let repo = repo.ok_or_else(|| anyhow!("missing 'repo' for get_repo"))?;
                let data = self.get_json(&format!("/repos/{}/{}", owner, repo)).await?;
                Ok(serde_json::to_string_pretty(&data)?)
            }
            "get_readme" => {
                let repo = repo.ok_or_else(|| anyhow!("missing 'repo' for get_readme"))?;
                let data = self
                    .get_json(&format!("/repos/{}/{}/readme", owner, repo))
                    .await?;
                let content_b64 = data
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("no content in readme response"))?;
                let cleaned: String =
                    content_b64.chars().filter(|c| !c.is_whitespace()).collect();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(cleaned.as_bytes())
                    .context("decoding readme base64")?;
                Ok(String::from_utf8_lossy(&bytes).to_string())
            }
            "list_commits" => {
                let repo = repo.ok_or_else(|| anyhow!("missing 'repo' for list_commits"))?;
                let data = self
                    .get_json(&format!(
                        "/repos/{}/{}/commits?per_page={}",
                        owner, repo, limit
                    ))
                    .await?;
                let mut out = String::new();
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        let sha = item
                            .get("sha")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(7)
                            .collect::<String>();
                        let msg = item
                            .pointer("/commit/message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .lines()
                            .next()
                            .unwrap_or("");
                        let date = item
                            .pointer("/commit/author/date")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        out.push_str(&format!("{} {} — {}\n", date, sha, msg));
                    }
                }
                Ok(out)
            }
            "list_user_repos" => {
                let data = self
                    .get_json(&format!("/users/{}/repos?per_page={}", owner, limit))
                    .await?;
                let mut out = String::new();
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let lang = item.get("language").and_then(|v| v.as_str()).unwrap_or("");
                        out.push_str(&format!("{} [{}] — {}\n", name, lang, desc));
                    }
                }
                Ok(out)
            }
            other => Err(anyhow!("unknown action: {}", other)),
        }
    }
}
