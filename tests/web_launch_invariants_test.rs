use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn billing_code_cannot_create_subscriptions_or_off_session_charges() {
    let mut failures = Vec::new();

    for file in implementation_files() {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        let normalized = compact_lowercase(&contents);
        let relative = display_path(&file);

        for (label, needle) in [
            (
                "Stripe Checkout subscription mode",
                "\"mode\":\"subscription\"",
            ),
            ("Stripe Checkout subscription mode", "mode:\"subscription\""),
            ("Stripe Checkout subscription mode", "mode='subscription'"),
            ("Stripe Checkout subscription mode", "mode=subscription"),
            (
                "Stripe Checkout subscription enum",
                "checkoutsessionmode::subscription",
            ),
            ("Stripe subscription create call", "subscriptions.create"),
            ("Stripe subscription create call", "subscription::create"),
            ("Stripe subscription API path", "/v1/subscriptions"),
            ("Stripe subscription data parameter", "subscription_data"),
            ("Stripe recurring price configuration", "\"recurring\":"),
            ("Stripe recurring price configuration", "recurring:"),
            ("Stripe recurring price parameter", "price_data[recurring]"),
            ("Stripe subscription trial parameter", "trial_period_days"),
            ("Stripe subscription cycle anchor", "billing_cycle_anchor"),
            ("off-session card charge path", "off_session"),
        ] {
            if normalized.contains(needle) {
                failures.push(format!("{relative}: forbidden {label} (`{needle}`)"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "web launch billing must stay setup-mode + explicit prepaid packs only; \
         no subscriptions, recurring prices, or off-session/automatic charges:\n{}",
        failures.join("\n")
    );
}

#[test]
fn browser_facing_code_never_references_provider_secret_env_vars() {
    let mut failures = Vec::new();

    for file in client_surface_files() {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        let relative = display_path(&file);
        let upper = contents.to_ascii_uppercase();

        for secret_name in [
            "ANTHROPIC_API_KEY",
            "DEEPGRAM_API_KEY",
            "ELEVENLABS_API_KEY",
            "OPENAI_API_KEY",
            "OPENROUTER_API_KEY",
            "STRIPE_SECRET_KEY",
            "STRIPE_WEBHOOK_SECRET",
        ] {
            if upper.contains(secret_name) {
                failures.push(format!(
                    "{relative}: browser-facing file references server-only `{secret_name}`"
                ));
            }
        }

        for public_prefix in [
            "NEXT_PUBLIC_ANTHROPIC",
            "NEXT_PUBLIC_DEEPGRAM",
            "NEXT_PUBLIC_ELEVENLABS",
            "NEXT_PUBLIC_OPENAI",
            "NEXT_PUBLIC_OPENROUTER",
            "NEXT_PUBLIC_STRIPE_SECRET",
            "PUBLIC_ANTHROPIC",
            "PUBLIC_DEEPGRAM",
            "PUBLIC_ELEVENLABS",
            "PUBLIC_OPENAI",
            "PUBLIC_OPENROUTER",
            "PUBLIC_STRIPE_SECRET",
            "VITE_ANTHROPIC",
            "VITE_DEEPGRAM",
            "VITE_ELEVENLABS",
            "VITE_OPENAI",
            "VITE_OPENROUTER",
            "VITE_STRIPE_SECRET",
        ] {
            if upper.contains(public_prefix) {
                failures.push(format!(
                    "{relative}: public env prefix would expose provider/Stripe secrets `{public_prefix}`"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Deepgram/OpenRouter/LLM provider keys and Stripe secrets must remain server-only:\n{}",
        failures.join("\n")
    );
}

#[test]
fn browser_mic_capture_is_gated_by_recording_consent() {
    let mut failures = Vec::new();

    for file in client_surface_files() {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        let haystack = compact_lowercase(&contents);
        let relative = display_path(&file);

        let capture_pos = [
            "navigator.mediadevices.getusermedia",
            ".getusermedia(",
            "newmediarecorder(",
            "audio.frame",
        ]
        .into_iter()
        .filter_map(|needle| haystack.find(needle).map(|pos| (needle, pos)))
        .min_by_key(|(_, pos)| *pos);

        let Some((capture_marker, first_capture_pos)) = capture_pos else {
            continue;
        };

        let consent_pos = [
            "consent.accept",
            "consentaccepted",
            "hasconsent",
            "recordingconsent",
            "recording-consent",
            "recordingconsentnotice",
            "consent_version",
            "consentversion",
        ]
        .into_iter()
        .filter_map(|needle| haystack.find(needle))
        .min();

        match consent_pos {
            Some(first_consent_pos) if first_consent_pos <= first_capture_pos => {}
            Some(_) => failures.push(format!(
                "{relative}: `{capture_marker}` appears before the consent gate"
            )),
            None => failures.push(format!(
                "{relative}: `{capture_marker}` appears without an explicit recording-consent gate"
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "browser mic capture / audio frame sending must be impossible before recording consent:\n{}",
        failures.join("\n")
    );
}

#[test]
fn tui_widget_contract_remains_the_web_ui_source_of_truth() {
    let root = repo_root();
    let widgets = fs::read_to_string(root.join("src/interview/tui/widgets.rs"))
        .expect("read TUI widgets source");
    let state =
        fs::read_to_string(root.join("src/interview/tui/state.rs")).expect("read TUI state source");

    for required in [
        "Constraint::Percentage(60)",
        "Constraint::Percentage(40)",
        "\" Transcript \"",
        "\" Brief \"",
        "\"✣ Conch\"",
        "Color::Magenta",
        "\"● You\"",
        "Color::Green",
        "\"▁\", \"▂\", \"▃\", \"▄\", \"▅\", \"▆\", \"▇\", \"█\"",
        "\" [Hold Space] talk  [Tap Space] toggle mic  [Esc] interrupt",
    ] {
        assert!(
            widgets.contains(required),
            "TUI/Web parity contract missing `{required}` from widgets.rs"
        );
    }

    for status in [
        "Idle",
        "Listening",
        "Thinking",
        "Filling",
        "Speaking",
        "Filler",
        "Closing",
    ] {
        assert!(
            state.contains(status),
            "TUI/Web parity contract missing `{status}` status from state.rs"
        );
    }
}

#[test]
fn web_interview_ui_preserves_tui_labels_when_present() {
    let candidates: Vec<(PathBuf, String)> = client_surface_files()
        .into_iter()
        .filter_map(|file| {
            let contents = fs::read_to_string(&file).ok()?;
            let lower_path = display_path(&file).to_ascii_lowercase();
            let lower_contents = contents.to_ascii_lowercase();
            let looks_like_interview_ui = lower_path.contains("interview")
                || lower_path.contains("session")
                || lower_path.contains("workspace")
                || lower_contents.contains("✣ conch")
                || lower_contents.contains("hold space")
                || lower_contents.contains("tap space");
            looks_like_interview_ui.then_some((file, contents))
        })
        .collect();

    if candidates.is_empty() {
        // The web app may be introduced by another worker. The source-of-truth
        // TUI contract test above still locks the tokens that web UI must mirror
        // once interview UI files exist.
        return;
    }

    let aggregate = candidates
        .iter()
        .map(|(path, contents)| format!("\n// {}\n{contents}", display_path(path)))
        .collect::<String>();

    for required in [
        "Transcript",
        "Brief",
        "✣ Conch",
        "● You",
        "Hold Space",
        "Tap Space",
        "Esc",
        "Listening",
        "Thinking",
        "Speaking",
        "Closing",
    ] {
        assert!(
            aggregate.contains(required),
            "web interview UI must mirror the TUI contract; missing `{required}`"
        );
    }

    assert!(
        ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
            .into_iter()
            .any(|bar| aggregate.contains(bar)),
        "web interview UI must preserve cyan block-style waveform bars from the TUI"
    );
}

#[test]
fn app_first_surface_replaces_animation_fake_brief_and_fixed_pricing() {
    let app = fs::read_to_string(repo_root().join("src/App.jsx")).expect("read App.jsx");
    let package = fs::read_to_string(repo_root().join("package.json")).expect("read package.json");
    let vercel = fs::read_to_string(repo_root().join("vercel.json")).expect("read vercel.json");

    for required in [
        "/app",
        "/app/signup",
        "Start free",
        "Open Conch",
        "BYOK",
        "usage-based",
        "Deepgram",
        "voice_config_missing",
        "email_confirmation_required",
        "usage_config_missing",
        "$10 of managed usage",
        "$10,000",
        "No subscription. No automatic charge.",
        "Ready to talk",
    ] {
        assert!(
            app.contains(required),
            "app-first web surface missing `{required}`"
        );
    }

    for state in [
        "signup_required",
        "trial_pending",
        "voice_config_missing",
        "email_confirmation_required",
        "usage_config_missing",
        "usage_limit_reached",
        "free_trials_closed",
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
            "web app missing finite state `{state}`"
        );
    }

    for forbidden in [
        "BackgroundBeams",
        "motion/react",
        "Launch brief / Checkout beta",
        "Finite app states",
        "Session checklist",
        "What happens next",
        "step-list",
        "<strong>{state}</strong>",
        "Session: {trialSession",
        "Request beta access",
        "View prepaid plans",
        "$29",
        "$199",
        "$499",
    ] {
        assert!(
            !app.contains(forbidden),
            "App.jsx still contains forbidden `{forbidden}`"
        );
        assert!(
            !vercel.contains(forbidden),
            "vercel.json still contains forbidden `{forbidden}`"
        );
    }

    assert!(
        !package.contains(r#""motion""#),
        "motion package must be removed with the old animated launch surface"
    );
    assert!(
        !repo_root()
            .join("src/components/BackgroundBeams.jsx")
            .exists(),
        "BackgroundBeams component should not exist after removing the animation"
    );
}

#[test]
fn deepgram_token_endpoint_is_server_only_and_consent_gated() {
    let broker = fs::read_to_string(repo_root().join("api/deepgram-token.js"))
        .expect("read Deepgram token broker");
    let app = fs::read_to_string(repo_root().join("src/App.jsx")).expect("read App.jsx");

    let signup =
        fs::read_to_string(repo_root().join("api/trial-signup.js")).expect("read trial signup API");
    let session = fs::read_to_string(repo_root().join("api/_session.js"))
        .expect("read session signing helper");
    let http = fs::read_to_string(repo_root().join("api/_http.js")).expect("read HTTP helper");

    for required in [
        "process.env.DEEPGRAM_API_KEY",
        "https://api.deepgram.com/v1/auth/grant",
        "verifyTrialSessionToken",
        "extractBearerToken",
        "ttl_seconds",
        "setJsonNoStoreHeaders",
        "reserveTrialTokenGrant",
        "signup_required",
        "email_confirmation_required",
        "consent_required",
        "voice_config_missing",
        "max_session_seconds",
    ] {
        assert!(
            broker.contains(required),
            "token broker missing `{required}`"
        );
    }

    assert!(
        !broker.contains("X-Conch-Trial") && !broker.contains(r#"startsWith("trial_")"#),
        "token broker must validate a signed session, not client-attested trial state"
    );
    let confirm = fs::read_to_string(repo_root().join("api/trial-confirm.js"))
        .expect("read trial confirmation API");
    let usage = fs::read_to_string(repo_root().join("api/_usage.js")).expect("read usage helper");

    let supabase =
        fs::read_to_string(repo_root().join("api/_supabase.js")).expect("read Supabase helper");
    let migration = fs::read_to_string(
        repo_root().join("supabase/migrations/20260426190000_conch_trial_usage.sql"),
    )
    .expect("read Supabase trial migration");

    assert!(signup.contains("requestSupabaseEmailConfirmation"));
    assert!(confirm.contains("getSupabaseUserFromAccessToken"));
    assert!(confirm.contains("verifySupabaseOtp"));
    assert!(session.contains("createHmac") && session.contains("timingSafeEqual"));
    assert!(supabase.contains("/auth/v1/otp") && supabase.contains("/rest/v1/rpc/"));
    assert!(migration.contains("conch_reserve_trial_usage"));
    assert!(
        usage.contains("perUserTrialBudgetCents") && usage.contains("globalFreeTrialBudgetCents")
    );
    assert!(usage.contains("1000") && usage.contains("1000000"));
    assert!(http.contains("Cache-Control") && http.contains("no-store"));

    assert!(
        !app.contains("DEEPGRAM_API_KEY"),
        "browser app must not reference the server-only Deepgram env var"
    );
    assert!(
        app.find("recordingConsentAccepted").unwrap_or(usize::MAX)
            < app.find("navigator.mediaDevices.getUserMedia").unwrap_or(0),
        "recording consent state must appear before getUserMedia in browser code"
    );
}

fn implementation_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(&repo_root(), &mut files);
    files
        .into_iter()
        .filter(|path| {
            has_implementation_extension(path)
                && !has_component(path, "tests")
                && !has_component(path, "specs")
                && !has_component(path, "target")
                && !has_component(path, ".git")
                && !has_component(path, ".omx")
                && !has_component(path, ".vercel")
                && !path.ends_with("Cargo.lock")
        })
        .collect()
}

fn client_surface_files() -> Vec<PathBuf> {
    implementation_files()
        .into_iter()
        .filter(|path| {
            let Some(ext) = path.extension().and_then(OsStr::to_str) else {
                return false;
            };
            matches!(
                ext,
                "cjs"
                    | "css"
                    | "html"
                    | "js"
                    | "jsx"
                    | "mjs"
                    | "scss"
                    | "svelte"
                    | "ts"
                    | "tsx"
                    | "vue"
            ) && !has_component(path, "api")
                && !has_component(path, "backend")
                && !has_component(path, "routes")
                && !has_component(path, "server")
        })
        .collect()
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            if matches!(
                name.as_ref(),
                ".git" | ".omx" | ".vercel" | "target" | "node_modules" | "dist" | "build"
            ) {
                continue;
            }
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn has_implementation_extension(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    matches!(
        ext,
        "cjs"
            | "css"
            | "html"
            | "js"
            | "jsx"
            | "json"
            | "mjs"
            | "rs"
            | "scss"
            | "svelte"
            | "toml"
            | "ts"
            | "tsx"
            | "vue"
    )
}

fn compact_lowercase(contents: &str) -> String {
    contents
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn has_component(path: &Path, component: &str) -> bool {
    path.components().any(|part| part.as_os_str() == component)
}

fn display_path(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
