use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn vercel_serves_static_launch_directory_at_site_root() {
    let config = read("vercel.json");

    assert!(
        config.contains(r#""outputDirectory": "web/static""#),
        "Vercel must publish web/static as the deployment root so / and /terms.html resolve"
    );
    for header in [
        "Content-Security-Policy",
        "X-Content-Type-Options",
        "X-Frame-Options",
        "Referrer-Policy",
        "Permissions-Policy",
    ] {
        assert!(config.contains(header), "missing security header {header}");
    }
}

#[test]
fn launch_static_pages_do_not_ship_placeholder_or_broken_root_links() {
    let static_root = repo_path("web/static");
    let mut failures = Vec::new();

    for entry in fs::read_dir(&static_root).expect("read web/static") {
        let entry = entry.expect("static entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("html") {
            continue;
        }
        let html = fs::read_to_string(&path).expect("read html page");
        let relative = path.strip_prefix(repo_root()).unwrap().display();

        for forbidden in ["example.com", "status.example.com", "#payments-disabled"] {
            if html.contains(forbidden) {
                failures.push(format!("{relative}: contains placeholder `{forbidden}`"));
            }
        }

        for href in hrefs(&html) {
            if let Some(root_path) = href.strip_prefix('/') {
                let root_path = root_path.split(['?', '#']).next().unwrap_or(root_path);
                let target = if root_path.is_empty() {
                    static_root.join("index.html")
                } else {
                    static_root.join(root_path)
                };
                if !target.exists() {
                    failures.push(format!(
                        "{relative}: root link `{href}` has no static target"
                    ));
                }
            }
            if href.starts_with("/web/static/") {
                failures.push(format!(
                    "{relative}: link `{href}` leaks the Vercel output directory"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "static launch pages must be root-routable and placeholder-free:\n{}",
        failures.join("\n")
    );
}

fn hrefs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(pos) = rest.find("href=\"") {
        rest = &rest[pos + 6..];
        if let Some(end) = rest.find('"') {
            out.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    out
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_path(path)).unwrap_or_else(|err| panic!("read {path}: {err}"))
}

fn repo_path(path: &str) -> PathBuf {
    repo_root().join(path)
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
