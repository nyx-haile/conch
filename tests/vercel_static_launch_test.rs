use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn vercel_builds_the_app_and_deep_links_into_spa() {
    let config = read("vercel.json");

    assert!(
        config.contains(r#""buildCommand": "npm run build""#),
        "Vercel must build the JavaScript app before publishing"
    );
    assert!(
        config.contains(r#""outputDirectory": "dist""#),
        "Vercel must publish Vite's dist directory at the deployment root"
    );
    for route in [
        "/download",
        "/cli",
        "/account",
        "/account/confirm",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
    ] {
        assert!(
            config.contains(&format!(r#""source": "{route}""#)),
            "Vercel must rewrite {route} into the SPA"
        );
    }
    assert!(
        config.contains("script-src 'self'") && !config.contains("script-src 'none'"),
        "CSP must allow the built JavaScript bundle while keeping scripts same-origin"
    );
    assert!(
        !config.contains("wss://api.deepgram.com") && !config.contains("wss://agent.deepgram.com"),
        "CSP must not expose Deepgram WebSocket origins now that browser voice is gone"
    );

    let config_json = read_json("vercel.json");
    for route in [
        "/",
        "/download",
        "/cli",
        "/account",
        "/account/confirm",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
    ] {
        assert_eq!(
            header_value_for(&config_json, route, "Permissions-Policy"),
            Some("camera=(), geolocation=(), microphone=(), payment=()"),
            "{route} must deny microphone access"
        );
    }

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
fn javascript_app_has_download_portal_build_contract() {
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
        package.pointer("/dependencies/motion").is_none(),
        "motion dependency must stay out of the bundle"
    );

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
        "src/styles.css",
        "public/favicon.svg",
        "public/robots.txt",
        "LICENSE",
    ] {
        assert!(
            repo_path(path).exists(),
            "missing required asset {path}"
        );
    }

    assert!(
        !repo_path("src/components/BackgroundBeams.jsx").exists(),
        "BackgroundBeams animation component must stay removed"
    );
    assert!(
        !repo_path("public/sitemap.xml").exists(),
        "public sitemap must stay out of the build until the canonical domain is live"
    );
}

#[test]
fn public_surface_is_placeholder_and_pii_free() {
    let mut failures = Vec::new();

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
        "src/styles.css",
        "public/robots.txt",
        "README.md",
    ] {
        let contents = read(path);
        for forbidden in [
            "example.com",
            "status.example.com",
            "nyx@users.noreply.github.com",
            "[redacted-preview-host]",
            "[redacted-supabase-ref]",
            "/web/static/",
            "motion/react",
            "BackgroundBeams",
            "Launch brief / Checkout beta",
            "Finite app states",
            "Session checklist",
            "Request beta access",
            "View prepaid plans",
            "Starter",
            "Team",
            "Pilot",
            "$29",
            "$199",
            "$499",
            "$10 of managed usage",
        ] {
            if contents.contains(forbidden) {
                failures.push(format!("{path}: contains forbidden `{forbidden}`"));
            }
        }
    }

    let app = read("src/App.jsx");
    for required in [
        "conch@theos.sh",
        "/download",
        "/account",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
        "cargo install",
    ] {
        assert!(
            app.contains(required),
            "JavaScript app missing `{required}`"
        );
    }

    assert!(
        !app.contains("DEEPGRAM_API_KEY"),
        "browser app must not reference any server-only key name"
    );
    assert!(
        !app.contains("MediaRecorder")
            && !app.contains("getUserMedia")
            && !app.contains("WebSocket"),
        "browser app must not contain microphone or live audio code"
    );

    let readme = read("README.md");
    assert!(
        !readme.contains("TBD") || !readme.contains("LICENSE"),
        "README must not advertise a missing LICENSE"
    );
    assert!(
        readme.contains("MIT"),
        "README must declare the project license"
    );

    assert!(
        failures.is_empty(),
        "Public surface must be placeholder- and PII-free:\n{}",
        failures.join("\n")
    );
}

fn header_value_for<'a>(config: &'a Value, source: &str, key: &str) -> Option<&'a str> {
    config
        .get("headers")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("source").and_then(Value::as_str) == Some(source))?
        .get("headers")?
        .as_array()?
        .iter()
        .find(|header| header.get("key").and_then(Value::as_str) == Some(key))?
        .get("value")?
        .as_str()
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
