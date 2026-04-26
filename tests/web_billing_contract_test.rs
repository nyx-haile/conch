use chrono::{Duration, TimeZone, Utc};
use conch::web::auth::{
    require_active_session, AuthError, AuthSession, SignupRequest, WorkspacePrincipal,
};
use conch::web::billing::{
    BillingError, CheckoutConfig, CheckoutMode, CheckoutSessionCompleted, CheckoutSessionDraft,
    CheckoutUrls, CreditLedger, LedgerOutcome, LegalUrls, PrepaidPack, SavedPaymentMethodUse,
    StripeWebhookEvent, UsageDebit, CHECKOUT_TRUST_COPY, TRIAL_MINUTES,
};
use conch::web::legal::{
    checkout_disclosure, landing_page, privacy_page, recording_consent_page,
    require_recording_consent, terms_page, CtaLinks, InterviewStartGate, PublicSiteConfig,
    RecordingConsentAcceptance, RECORDING_CONSENT_BUTTON, REQUIRED_BILLING_DISCLOSURE,
};

#[test]
fn trial_setup_checkout_uses_card_setup_mode_without_a_charge() {
    let draft =
        CheckoutSessionDraft::trial_setup(&checkout_config(false), "workspace-1", "a@b.com")
            .unwrap();

    assert_eq!(draft.mode, CheckoutMode::Setup);
    assert_eq!(draft.mode.as_str(), "setup");
    assert_eq!(
        draft.payment_method_use,
        Some(SavedPaymentMethodUse::CustomerPresentOnly)
    );
    assert_eq!(
        draft
            .payment_method_use
            .expect("setup session saves a customer-present payment method")
            .as_str(),
        "on_session"
    );
    assert_eq!(draft.pack, None);
    assert_eq!(draft.price_lookup_key, None);
    assert_eq!(draft.trust_copy, CHECKOUT_TRUST_COPY);
    assert!(draft.trust_copy.contains("No subscription"));
    assert!(draft.trust_copy.contains("No automatic charge"));
}

#[test]
fn prepaid_pack_checkout_uses_one_time_payment_mode_and_expected_minutes() {
    let draft = CheckoutSessionDraft::prepaid_pack(
        &checkout_config(false),
        "workspace-1",
        PrepaidPack::Team,
    )
    .unwrap();

    assert_eq!(draft.mode, CheckoutMode::Payment);
    assert_eq!(draft.mode.as_str(), "payment");
    assert_eq!(draft.pack, Some(PrepaidPack::Team));
    assert_eq!(
        draft.price_lookup_key.as_deref(),
        Some("conch_team_10000_stt_minutes")
    );
    assert_eq!(PrepaidPack::Starter.price_cents(), 2_900);
    assert_eq!(PrepaidPack::Starter.stt_minutes(), 1_000);
    assert_eq!(PrepaidPack::Team.price_cents(), 19_900);
    assert_eq!(PrepaidPack::Team.stt_minutes(), 10_000);
    assert_eq!(PrepaidPack::Pilot.price_cents(), 49_900);
    assert_eq!(PrepaidPack::Pilot.stt_minutes(), 30_000);
}

#[test]
fn live_checkout_rejects_placeholder_legal_or_payment_values() {
    let err = CheckoutSessionDraft::trial_setup(&checkout_config(true), "workspace-1", "a@b.com")
        .unwrap_err();

    assert!(matches!(
        err,
        BillingError::LivePaymentPlaceholder {
            field: "success_url",
            ..
        }
    ));
}

#[test]
fn setup_checkout_success_grants_exactly_one_trial_idempotently() {
    let mut ledger = CreditLedger::new();
    let event = setup_completed_event("evt-setup-1");

    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(event.clone())),
        Ok(LedgerOutcome::TrialGranted {
            seconds: i64::from(TRIAL_MINUTES) * 60,
        })
    );
    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(event)),
        Ok(LedgerOutcome::DuplicateIgnored)
    );

    let balance = ledger.balance("workspace-1").unwrap();
    assert_eq!(
        balance.trial_seconds_remaining,
        i64::from(TRIAL_MINUTES) * 60
    );
    assert_eq!(balance.paid_seconds_remaining, 0);
    assert_eq!(balance.payment_method_id.as_deref(), Some("pm_trial"));
    assert_eq!(
        balance.payment_method_detach_after,
        Some(instant() + Duration::days(44))
    );
}

#[test]
fn failed_or_expired_setup_session_grants_no_trial_minutes() {
    let mut ledger = CreditLedger::new();

    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionExpired {
            event_id: "evt-expired-1".to_string(),
            session_id: "cs_expired".to_string(),
        }),
        Ok(LedgerOutcome::NoCreditGrant)
    );
    assert!(ledger.balance("workspace-1").is_none());
}

#[test]
fn paid_pack_grants_once_and_refunds_never_make_usage_negative() {
    let mut ledger = CreditLedger::new();
    let event = CheckoutSessionCompleted {
        event_id: "evt-paid-1".to_string(),
        session_id: "cs_paid".to_string(),
        mode: CheckoutMode::Payment,
        workspace_id: "workspace-1".to_string(),
        stripe_customer_id: "cus_123".to_string(),
        payment_method_id: None,
        pack: Some(PrepaidPack::Starter),
        completed_at: instant(),
    };

    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(event.clone())),
        Ok(LedgerOutcome::PaidPackGranted {
            pack: PrepaidPack::Starter,
            seconds: 60_000,
        })
    );
    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(event)),
        Ok(LedgerOutcome::DuplicateIgnored)
    );
    assert_eq!(
        ledger
            .balance("workspace-1")
            .unwrap()
            .paid_seconds_remaining,
        60_000
    );

    assert_eq!(
        ledger.apply(StripeWebhookEvent::PaidPackRefunded {
            event_id: "evt-refund-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            seconds_to_revoke: 99_999,
        }),
        Ok(LedgerOutcome::PaidCreditsRevoked { seconds: 60_000 })
    );
    assert_eq!(
        ledger
            .balance("workspace-1")
            .unwrap()
            .paid_seconds_remaining,
        0
    );
}

#[test]
fn cloud_stt_decrements_credits_but_local_stt_does_not() {
    let mut ledger = CreditLedger::new();
    ledger
        .apply(StripeWebhookEvent::CheckoutSessionCompleted(
            setup_completed_event("evt-setup-2"),
        ))
        .unwrap();

    let before = ledger
        .balance("workspace-1")
        .unwrap()
        .total_seconds_remaining();
    assert_eq!(
        ledger.debit_usage("workspace-1", UsageDebit::LocalStt { seconds: 600 }),
        Ok(LedgerOutcome::NoCreditGrant)
    );
    assert_eq!(
        ledger
            .balance("workspace-1")
            .unwrap()
            .total_seconds_remaining(),
        before
    );

    assert_eq!(
        ledger.debit_usage("workspace-1", UsageDebit::DeepgramCloudStt { seconds: 75 },),
        Ok(LedgerOutcome::UsageDebited { seconds: 75 })
    );
    assert_eq!(
        ledger
            .balance("workspace-1")
            .unwrap()
            .total_seconds_remaining(),
        before - 75
    );
}

#[test]
fn auth_shell_requires_normalized_email_terms_and_active_session() {
    let signup = SignupRequest::new("  USER@Example.COM  ", "terms-2026-04-25").unwrap();
    assert_eq!(signup.email, "user@example.com");
    assert_eq!(
        SignupRequest::new("not-an-email", "terms").unwrap_err(),
        AuthError::InvalidEmail
    );

    let session = AuthSession {
        principal: WorkspacePrincipal {
            user_id: "user-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            email: signup.email,
        },
        expires_at: instant() + Duration::hours(1),
    };
    assert_eq!(
        require_active_session(Some(&session), instant())
            .unwrap()
            .workspace_id,
        "workspace-1"
    );
    assert_eq!(
        require_active_session(Some(&session), instant() + Duration::hours(2)).unwrap_err(),
        AuthError::ExpiredSession
    );
}

#[test]
fn legal_and_static_surfaces_include_required_payment_and_consent_copy() {
    let config = PublicSiteConfig::beta_placeholder();
    let landing = landing_page(&config, &CtaLinks::disabled());
    let terms = terms_page(&config);
    let privacy = privacy_page(&config);
    let recording = recording_consent_page(&config);

    for surface in [&landing, &terms] {
        assert!(surface.contains(REQUIRED_BILLING_DISCLOSURE));
        assert!(surface.contains("prepaid"));
    }
    assert!(landing.contains("aria-disabled=\"true\""));
    assert!(privacy.contains("raw card data does not touch Conch servers"));
    assert!(privacy.contains("Provider API keys are server-only"));
    assert!(recording.contains(RECORDING_CONSENT_BUTTON));
    assert_eq!(checkout_disclosure(), REQUIRED_BILLING_DISCLOSURE);
}

#[test]
fn recording_consent_gate_blocks_interview_start_until_accepted() {
    assert_eq!(
        require_recording_consent(None),
        InterviewStartGate::MissingRecordingConsent
    );
    let consent = RecordingConsentAcceptance::new("workspace-1", instant());
    assert_eq!(
        require_recording_consent(Some(&consent)),
        InterviewStartGate::Allowed
    );
}

#[test]
fn unsupported_checkout_modes_are_rejected_at_the_boundary() {
    assert_eq!(CheckoutMode::try_from("setup"), Ok(CheckoutMode::Setup));
    assert_eq!(CheckoutMode::try_from("payment"), Ok(CheckoutMode::Payment));
    assert_eq!(
        CheckoutMode::try_from("subscription").unwrap_err(),
        BillingError::UnsupportedCheckoutMode("subscription".to_string())
    );
}

fn checkout_config(live_payments_enabled: bool) -> CheckoutConfig {
    CheckoutConfig {
        urls: CheckoutUrls {
            success_url: "https://example.com/success".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
        },
        legal: LegalUrls {
            terms_url: "https://example.com/terms".to_string(),
            privacy_url: "https://example.com/privacy".to_string(),
            recording_consent_url: "https://example.com/recording-consent".to_string(),
            support_email: "support@example.com".to_string(),
            billing_email: "billing@example.com".to_string(),
        },
        live_payments_enabled,
    }
}

fn setup_completed_event(event_id: &str) -> CheckoutSessionCompleted {
    CheckoutSessionCompleted {
        event_id: event_id.to_string(),
        session_id: "cs_setup".to_string(),
        mode: CheckoutMode::Setup,
        workspace_id: "workspace-1".to_string(),
        stripe_customer_id: "cus_123".to_string(),
        payment_method_id: Some("pm_trial".to_string()),
        pack: None,
        completed_at: instant(),
    }
}

fn instant() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 25, 12, 0, 0)
        .single()
        .unwrap()
}
