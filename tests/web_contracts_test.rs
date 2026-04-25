use chrono::Utc;
use conch::interview::tui::state::Status;
use conch::web::*;
use serde_json::json;

#[test]
fn client_event_schema_uses_approved_realtime_names() {
    let event = ClientEvent::SessionStart(SessionStart {
        topic: "this repository".to_string(),
        model_slug: Some(FREE_DEFAULT_MODEL_SLUG.to_string()),
    });

    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({
            "type": "session.start",
            "payload": {
                "topic": "this repository",
                "model_slug": "x-ai/grok-4-fast"
            }
        })
    );

    let frame = ClientEvent::AudioFrame(AudioFrame {
        codec: AudioCodec::Pcm16,
        sample_rate: 16_000,
        sequence: 42,
        chunk: "AAE=".to_string(),
    });
    assert_eq!(serde_json::to_value(frame).unwrap()["type"], "audio.frame");
}

#[test]
fn server_event_schema_carries_web_status_and_usage() {
    let event = ServerEvent::StatusChanged(StatusChanged {
        status: WebStatus::from(Status::Listening),
        banner: None,
    });

    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({
            "type": "status.changed",
            "payload": { "status": "listening" }
        })
    );

    let usage = ServerEvent::UsageUpdated(UsageUpdated {
        stt_seconds: 30,
        tts_chars: 120,
        llm_tokens: 800,
        credits_remaining: CreditsRemaining {
            stt_seconds: 7_170,
            tts_chars: 59_880,
            llm_budget_cents: 499,
        },
    });
    assert_eq!(
        serde_json::to_value(usage).unwrap()["type"],
        "usage.updated"
    );
}

#[test]
fn tui_parity_tokens_match_launch_contract() {
    let tokens = ui_token_contract();
    assert_eq!(tokens.conch_label, "✣ Conch");
    assert_eq!(tokens.user_label, "● You");
    assert_eq!(tokens.desktop_transcript_percent, 60);
    assert_eq!(tokens.desktop_sidebar_percent, 40);
    assert_eq!(
        tokens.waveform_bars,
        ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
    );
}

#[test]
fn launch_catalog_defaults_to_openrouter_grok_and_enforces_free_entitlement() {
    let catalog = ModelCatalog::launch_default();
    let grok = catalog.get(FREE_DEFAULT_MODEL_SLUG).unwrap();
    assert_eq!(grok.provider, ModelProvider::OpenRouter);
    assert_eq!(grok.tier, ModelTier::Free);
    assert!(grok.enabled);

    let policy = WorkspaceModelPolicyRecord::launch_default("ws_1");
    let entitlements = EntitlementsRecord::trial_default("ws_1");
    let allowed = catalog.allowed_for(&policy, &entitlements);
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0].slug, FREE_DEFAULT_MODEL_SLUG);

    let paid_model = catalog.get("anthropic/claude-haiku-4.5").unwrap();
    let err = policy
        .ensure_model_allowed(paid_model, &entitlements)
        .unwrap_err();
    assert!(matches!(
        err,
        ModelPolicyError::TierNotInWorkspacePolicy(ModelTier::Balanced)
    ));
}

#[test]
fn paid_policy_allows_balanced_models_without_exposing_disabled_routes() {
    let catalog = ModelCatalog::launch_default();
    let mut policy = WorkspaceModelPolicyRecord::launch_default("ws_1");
    policy.allowed_tiers.push(ModelTier::Balanced);
    let mut entitlements = EntitlementsRecord::trial_default("ws_1");
    entitlements.model_tier = ModelTier::Balanced;

    let paid_model = catalog.get("anthropic/claude-haiku-4.5").unwrap();
    assert!(policy
        .ensure_model_allowed(paid_model, &entitlements)
        .is_ok());

    policy
        .disabled_provider_routes
        .push(ModelProvider::OpenRouter);
    let err = policy
        .ensure_model_allowed(paid_model, &entitlements)
        .unwrap_err();
    assert!(matches!(err, ModelPolicyError::DisabledProvider(_)));
}

#[test]
fn billing_contract_rejects_subscriptions_and_trial_charges() {
    let now = Utc::now();
    let subscription = PaymentEventRecord {
        id: "pe_1".to_string(),
        workspace_id: "ws_1".to_string(),
        stripe_event_id: "evt_sub".to_string(),
        event_type: PaymentEventType::OneTimePaymentSucceeded,
        checkout_mode: CheckoutMode::Subscription,
        amount_cents: 1000,
        credit_stt_seconds: 3_600,
        processed_at: now,
    };
    assert_eq!(
        subscription.validate_launch_contract().unwrap_err(),
        BillingContractError::SubscriptionModeForbidden
    );

    let trial_charge = PaymentEventRecord {
        id: "pe_2".to_string(),
        workspace_id: "ws_1".to_string(),
        stripe_event_id: "evt_setup".to_string(),
        event_type: PaymentEventType::TrialSetupSucceeded,
        checkout_mode: CheckoutMode::Setup,
        amount_cents: 1,
        credit_stt_seconds: DEFAULT_TRIAL_STT_SECONDS as i64,
        processed_at: now,
    };
    assert_eq!(
        trial_charge.validate_launch_contract().unwrap_err(),
        BillingContractError::TrialMustNotCharge
    );

    let trial = PaymentEventRecord {
        amount_cents: 0,
        ..trial_charge
    };
    assert!(trial.validate_launch_contract().is_ok());
    assert!(trial.grants_trial());
}

#[test]
fn usage_ledger_deduplicates_provider_events_and_computes_remaining_credits() {
    let now = Utc::now();
    let mut ledger = UsageLedger::default();

    let first = UsageEventRecord {
        id: "usage_1".to_string(),
        workspace_id: "ws_1".to_string(),
        session_id: Some("sess_1".to_string()),
        kind: UsageKind::SttSeconds,
        units: 30,
        provider: "deepgram".to_string(),
        provider_request_id: Some("dg_req_1".to_string()),
        created_at: now,
    };
    let duplicate = UsageEventRecord {
        id: "usage_2".to_string(),
        ..first.clone()
    };
    assert!(ledger.record(first));
    assert!(!ledger.record(duplicate));

    assert!(ledger.record(UsageEventRecord {
        id: "usage_3".to_string(),
        workspace_id: "ws_1".to_string(),
        session_id: Some("sess_1".to_string()),
        kind: UsageKind::TtsCharacters,
        units: 120,
        provider: "deepgram".to_string(),
        provider_request_id: Some("dg_tts_1".to_string()),
        created_at: now,
    }));
    assert!(ledger.record(UsageEventRecord {
        id: "usage_4".to_string(),
        workspace_id: "ws_1".to_string(),
        session_id: Some("sess_1".to_string()),
        kind: UsageKind::LlmCostCents,
        units: 7,
        provider: "openrouter".to_string(),
        provider_request_id: Some("or_req_1".to_string()),
        created_at: now,
    }));

    let totals = ledger.totals_for_workspace("ws_1");
    assert_eq!(totals.stt_seconds, 30);
    assert_eq!(totals.tts_chars, 120);
    assert_eq!(totals.llm_cost_cents, 7);

    let entitlements = EntitlementsRecord {
        trial_stt_seconds: 60,
        paid_stt_seconds: 0,
        llm_budget_cents: 10,
        tts_char_budget: 200,
        ..EntitlementsRecord::trial_default("ws_1")
    };
    assert_eq!(
        entitlements.credits_remaining(&totals),
        CreditsRemaining {
            stt_seconds: 30,
            tts_chars: 80,
            llm_budget_cents: 3,
        }
    );
}
