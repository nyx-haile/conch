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
        "/app",
        "/app/signup",
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
        config.contains("connect-src 'self' wss://api.deepgram.com wss://agent.deepgram.com"),
        "CSP must narrowly allow Deepgram WebSocket origins for browser voice"
    );
    let config_json = read_json("vercel.json");
    assert_eq!(
        header_value_for(&config_json, "/app", "Permissions-Policy"),
        Some("camera=(), geolocation=(), microphone=(self), payment=()"),
        "only /app should allow same-origin microphone use after consent"
    );
    for route in [
        "/",
        "/app/signup",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
    ] {
        assert_eq!(
            header_value_for(&config_json, route, "Permissions-Policy"),
            Some("camera=(), geolocation=(), microphone=(), payment=()"),
            "{route} should deny microphone access"
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
fn javascript_app_has_app_first_build_contract() {
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
        "motion dependency must be removed with the animation-heavy launch surface"
    );

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
        "src/styles.css",
        "api/_session.js",
        "api/_usage.js",
        "api/trial-signup.js",
        "api/trial-confirm.js",
        "api/deepgram-token.js",
        "public/favicon.svg",
        "public/robots.txt",
        "public/sitemap.xml",
    ] {
        assert!(
            repo_path(path).exists(),
            "missing app asset or serverless helper {path}"
        );
    }
    assert!(
        !repo_path("src/components/BackgroundBeams.jsx").exists(),
        "BackgroundBeams animation component must be removed"
    );
}

#[test]
fn app_first_surface_is_placeholder_free_and_accessible() {
    let mut failures = Vec::new();

    for path in [
        "index.html",
        "src/main.jsx",
        "src/App.jsx",
        "vite.config.js",
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
            "motion/react",
            "BackgroundBeams",
            "Launch brief / Checkout beta",
            "Finite app states",
            "Session checklist",
            "What happens next",
            "step-list",
            "<strong>{state}</strong>",
            "Session: {trialSession",
            "Request beta access",
            "View prepaid plans",
            "Starter",
            "Team",
            "Pilot",
            "$29",
            "$199",
            "$499",
        ] {
            if contents.contains(forbidden) {
                failures.push(format!("{path}: contains forbidden `{forbidden}`"));
            }
        }
    }

    let app = read("src/App.jsx");
    for required in [
        "conch@theos.sh",
        "/app",
        "/app/signup",
        "Start free",
        "Open Conch",
        "BYOK",
        "usage-based",
        "Deepgram",
        "voice_config_missing",
        "email_confirmation_required",
        "$10 of managed usage",
        "$10,000",
        "No subscription. No automatic charge.",
        "Ready to talk",
        "/terms.html",
        "/privacy.html",
        "/recording-consent.html",
        "/status.html",
    ] {
        assert!(
            app.contains(required),
            "JavaScript app missing `{required}`"
        );
    }

    for state in [
        "signup_required",
        "trial_pending",
        "voice_config_missing",
        "email_confirmation_required",
        "usage_config_missing",
        "consent_required",
        "ready_to_talk",
        "listening",
        "thinking",
        "speaking",
        "ended",
        "error",
    ] {
        assert!(
            app.contains(state),
            "app shell missing finite state `{state}`"
        );
    }

    assert!(
        failures.is_empty(),
        "App-first launch app must be root-routable, animation-free, and placeholder-free:\n{}",
        failures.join("\n")
    );
}

#[test]
fn deepgram_token_broker_is_server_only_and_no_store() {
    let broker = read("api/deepgram-token.js");
    let signup = read("api/trial-signup.js");
    let session = read("api/_session.js");
    let usage = read("api/_usage.js");
    let http = read("api/_http.js");

    for required in [
        "https://api.deepgram.com/v1/auth/grant",
        "process.env.DEEPGRAM_API_KEY",
        "verifyTrialSessionToken",
        "extractBearerToken",
        "ttl_seconds",
        "setJsonNoStoreHeaders",
        "voice_config_missing",
        "email_confirmation_required",
        "consent_required",
        "signup_required",
        "Authorization",
        "X-Conch-Session",
        "max_session_seconds",
    ] {
        assert!(
            broker.contains(required),
            "token broker missing `{required}`"
        );
    }

    assert!(
        !broker.contains("X-Conch-Trial") && !broker.contains(r#"startsWith("trial_")"#),
        "token broker must not trust self-attested trial headers or raw trial id shape"
    );
    assert!(
        signup.contains("createEmailConfirmation"),
        "signup API must require email confirmation before trial sessions"
    );

    for required in [
        "usage_config_missing",
        "usage_limit_reached",
        "free_trials_closed",
        "perUserTrialBudgetCents",
        "globalFreeTrialBudgetCents",
        "1000",
        "1000000",
    ] {
        assert!(
            usage.contains(required),
            "usage helper missing `{required}`"
        );
    }

    for required in [
        "createHmac",
        "timingSafeEqual",
        "CONCH_SESSION_SIGNING_SECRET",
        "DEEPGRAM_API_KEY",
    ] {
        assert!(
            session.contains(required),
            "session signer missing `{required}`"
        );
    }

    for required in ["Cache-Control", "no-store", "readJsonBody"] {
        assert!(http.contains(required), "HTTP helper missing `{required}`");
    }

    assert!(
        !read("src/App.jsx").contains("DEEPGRAM_API_KEY"),
        "browser app must not reference the server-only Deepgram key name"
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
