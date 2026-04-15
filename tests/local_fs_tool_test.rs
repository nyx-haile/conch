use conch::prep::local_fs_tool::LocalFsTool;
use conch::prep::tools::Tool;
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

fn write_file(root: &std::path::Path, rel: &str, body: &str) {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full, body).unwrap();
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

#[tokio::test]
async fn list_dir_returns_entries_with_kinds() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "README.md", "hello");
    write_file(tmp.path(), "src/main.rs", "fn main(){}");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "list_dir", "path": "." }))
        .await
        .unwrap();
    let v = parse(&out);
    let names: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(names.contains(&"README.md"));
    assert!(names.contains(&"src"));

    let readme = v
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e.get("name").unwrap() == "README.md")
        .unwrap();
    assert_eq!(readme.get("kind").unwrap(), "file");

    let src = v
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e.get("name").unwrap() == "src")
        .unwrap();
    assert_eq!(src.get("kind").unwrap(), "dir");
}

#[tokio::test]
async fn list_dir_skips_ignored_directories() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/main.rs", "");
    write_file(tmp.path(), "target/debug/foo", "");
    write_file(tmp.path(), "node_modules/pkg/index.js", "");
    write_file(tmp.path(), ".git/config", "");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "list_dir", "path": "." }))
        .await
        .unwrap();
    let v = parse(&out);
    let names: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(names.contains(&"src"));
    assert!(!names.contains(&"target"));
    assert!(!names.contains(&"node_modules"));
    assert!(!names.contains(&".git"));
}

#[tokio::test]
async fn read_file_returns_contents() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "README.md", "# Hello\n\nWorld");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "read_file", "path": "README.md" }))
        .await
        .unwrap();
    assert!(out.contains("# Hello"));
    assert!(out.contains("World"));
}

#[tokio::test]
async fn read_file_truncates_large_files_with_marker() {
    let tmp = TempDir::new().unwrap();
    let big = "x".repeat(200_000);
    write_file(tmp.path(), "big.txt", &big);

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "read_file", "path": "big.txt" }))
        .await
        .unwrap();
    assert!(out.len() < 200_000);
    assert!(out.contains("truncated"));
}

#[tokio::test]
async fn tree_returns_nested_structure_up_to_depth() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "a/b/c/deep.txt", "");
    write_file(tmp.path(), "top.md", "");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "tree", "max_depth": 2 }))
        .await
        .unwrap();

    assert!(out.contains("top.md"));
    assert!(out.contains("a"));
    assert!(out.contains("b"));
    // depth 2 shouldn't reach "c"
    assert!(!out.contains("deep.txt"));
}

#[tokio::test]
async fn tree_skips_ignored_directories() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/main.rs", "");
    write_file(tmp.path(), "target/junk.o", "");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let out = tool
        .call(json!({ "action": "tree", "max_depth": 3 }))
        .await
        .unwrap();

    assert!(out.contains("src"));
    assert!(!out.contains("target"));
    assert!(!out.contains("junk.o"));
}

#[tokio::test]
async fn rejects_path_escape_above_root() {
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "inside.txt", "ok");

    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let result = tool
        .call(json!({ "action": "read_file", "path": "../../../etc/passwd" }))
        .await;
    assert!(result.is_err(), "expected error, got {:?}", result);
}

#[tokio::test]
async fn rejects_absolute_path_outside_root() {
    let tmp = TempDir::new().unwrap();
    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let result = tool
        .call(json!({ "action": "read_file", "path": "/etc/passwd" }))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_file_missing_path_errors() {
    let tmp = TempDir::new().unwrap();
    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let result = tool
        .call(json!({ "action": "read_file", "path": "nope.txt" }))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn unknown_action_errors() {
    let tmp = TempDir::new().unwrap();
    let tool = LocalFsTool::new(tmp.path().to_path_buf());
    let result = tool
        .call(json!({ "action": "destroy_universe", "path": "." }))
        .await;
    assert!(result.is_err());
}
