use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn vercel_builds_the_javascript_app_at_site_root() {
    let config = read("vercel.json");

    assert!(
        config.contains(r#""buildCommand": "npm run build""#),
        "Vercel must build the JavaScript app before publishing"
    );
    assert!(
        config.contains(r#""outputDirectory": "dist""#),
        "Vercel must publish Vite's dist directory at the deployment root"
    );
    assert!(
        config.contains(r#""source": "/terms.html""#)
            && config.contains(r#""source": "/privacy.html""#)
            && config.contains(r#""source": "/recording-consent.html""#)
            && config.contains(r#""source": "/status.html""#),
        "legacy public legal/status URLs must rewrite into the SPA"
    );
    assert!(
        config.contains("script-src 'self'") && !config.contains("script-src 'none'"),
        "CSP must allow the built JavaScript bundle while keeping scripts same-origin"
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
fn javascript_launch_app_has_required_build_contract() {
    let package = read_json("package.json");
    let scripts = package
        .get("scripts")
        .and_then(Value::as_object)
        .expect("package.json scripts object");
    assert_eq!(
        scripts.get("build").and_then(Value::as_str),
        Some("vite build"),
        "npm run build must create the Vite dist deployment"
    );
    assert!(
        package
            .pointer("/dependencies/react")
            .and_then(Value::as_str)
            .is_some(),
        "React must be present for the JavaScript app"
    );
    assert!(
        package
            .pointer("/dependencies/motion")
            .and_then(Value::as_str)
            .is_some(),
        "motion must be present for the Aceternity-inspired animated UI"
    );

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
        "src/components/BackgroundBeams.jsx",
        "src/styles.css",
        "public/favicon.svg",
        "public/robots.txt",
        "public/sitemap.xml",
    ] {
        assert!(
            repo_path(path).exists(),
            "missing JavaScript app asset {path}"
        );
    }
}

#[test]
fn javascript_launch_app_is_placeholder_free_and_root_routable() {
    let mut failures = Vec::new();

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
        "src/components/BackgroundBeams.jsx",
        "src/styles.css",
        "public/robots.txt",
        "public/sitemap.xml",
    ] {
        let contents = read(path);
        for forbidden in [
            "example.com",
            "status.example.com",
            "#payments-disabled",
            "nyx@users.noreply.github.com",
            "/web/static/",
            "navigator.mediaDevices.getUserMedia",
            ".getUserMedia(",
            "new MediaRecorder(",
        ] {
            if contents.contains(forbidden) {
                failures.push(format!("{path}: contains forbidden `{forbidden}`"));
            }
        }
    }

    let app = read("src/App.jsx");
    for required in [
        "conch@theos.sh",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
        "No subscription. No automatic charge.",
    ] {
        assert!(
            app.contains(required),
            "JavaScript app missing `{required}`"
        );
    }

    assert!(
        failures.is_empty(),
        "JavaScript launch app must be root-routable and placeholder-free:\n{}",
        failures.join("\n")
    );
}

fn read_json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|err| panic!("parse {path}: {err}"))
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
