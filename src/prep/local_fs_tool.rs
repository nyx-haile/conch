use crate::prep::tools::Tool;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MAX_READ_BYTES: usize = 100_000;
const DEFAULT_TREE_DEPTH: usize = 3;
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    "__pycache__",
    ".next",
    ".cache",
];

pub struct LocalFsTool {
    root: PathBuf,
}

impl LocalFsTool {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, rel: &str) -> anyhow::Result<PathBuf> {
        let candidate = self.root.join(rel);
        let canonical_root = self
            .root
            .canonicalize()
            .with_context(|| format!("canonicalize root {:?}", self.root))?;

        // For read operations the path must exist; for listings the path must
        // exist too. We canonicalize the candidate when possible; if it fails,
        // we fall back to lexical containment check on the joined path.
        let resolved = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => candidate.clone(),
        };

        if !resolved.starts_with(&canonical_root) {
            return Err(anyhow!(
                "path escapes root: {:?} not under {:?}",
                resolved,
                canonical_root
            ));
        }
        Ok(resolved)
    }

    fn list_dir(&self, rel: &str) -> anyhow::Result<String> {
        let dir = self.resolve(rel)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&dir).with_context(|| format!("read_dir {:?}", dir))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let ft = entry.file_type()?;
            let kind = if ft.is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                "dir"
            } else if ft.is_file() {
                "file"
            } else {
                continue;
            };
            let mut item = json!({ "name": name, "kind": kind });
            if kind == "file" {
                if let Ok(meta) = entry.metadata() {
                    item["size"] = json!(meta.len());
                }
            }
            entries.push(item);
        }
        entries.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        });
        Ok(serde_json::to_string(&entries)?)
    }

    fn read_file(&self, rel: &str) -> anyhow::Result<String> {
        let path = self.resolve(rel)?;
        let bytes = std::fs::read(&path).with_context(|| format!("read {:?}", path))?;
        if bytes.len() > MAX_READ_BYTES {
            let head = String::from_utf8_lossy(&bytes[..MAX_READ_BYTES]).to_string();
            Ok(format!(
                "{}\n\n... [truncated: file is {} bytes, showing first {}]",
                head,
                bytes.len(),
                MAX_READ_BYTES
            ))
        } else {
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
    }

    fn tree(&self, rel: &str, max_depth: usize) -> anyhow::Result<String> {
        let start = self.resolve(rel)?;
        let mut out = String::new();
        let display_root = if rel.is_empty() || rel == "." {
            ".".to_string()
        } else {
            rel.to_string()
        };
        out.push_str(&display_root);
        out.push('\n');
        walk_tree(&start, "", max_depth, &mut out)?;
        Ok(out)
    }
}

fn walk_tree(
    dir: &Path,
    prefix: &str,
    depth_remaining: usize,
    out: &mut String,
) -> anyhow::Result<()> {
    if depth_remaining == 0 {
        return Ok(());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("read_dir {:?}", dir))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|f| f.is_dir()).unwrap_or(false);
            !(is_dir && IGNORED_DIRS.contains(&name.as_str()))
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let last_idx = entries.len().saturating_sub(1);
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == last_idx;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        out.push_str(prefix);
        out.push_str(connector);
        out.push_str(&name);
        out.push('\n');

        if entry.file_type()?.is_dir() {
            let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            walk_tree(&entry.path(), &next_prefix, depth_remaining - 1, out)?;
        }
    }
    Ok(())
}

#[async_trait]
impl Tool for LocalFsTool {
    fn name(&self) -> &'static str {
        "local_fs"
    }

    fn description(&self) -> &'static str {
        "Inspect the local filesystem rooted at the current working directory. Use this when the topic refers to a local repo or project on disk. Actions: list_dir (entries in a directory), read_file (text contents, capped at 100KB), tree (nested layout up to max_depth, default 3). Paths are relative to the root and may not escape it. Common build/cache dirs are skipped automatically."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list_dir", "read_file", "tree"]
                },
                "path": {
                    "type": "string",
                    "description": "Path relative to the root (e.g. '.', 'src', 'README.md'). Default '.' for list_dir/tree."
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Max depth for tree (default 3)."
                }
            },
            "required": ["action"]
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<String> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'action'"))?;
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        match action {
            "list_dir" => self.list_dir(path),
            "read_file" => self.read_file(path),
            "tree" => {
                let depth = input
                    .get("max_depth")
                    .and_then(|v| v.as_u64())
                    .map(|d| d as usize)
                    .unwrap_or(DEFAULT_TREE_DEPTH);
                self.tree(path, depth)
            }
            other => Err(anyhow!("unknown action: {}", other)),
        }
    }
}
