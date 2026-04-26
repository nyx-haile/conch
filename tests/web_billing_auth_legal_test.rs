use chrono::{Duration, TimeZone, Utc};
use conch::web::auth::{
    require_active_session, AuthError, AuthSession, SignupRequest, WorkspacePrincipal,
};
use conch::web::billing::{
    BillingError, CheckoutConfig, CheckoutMode, CheckoutSessionCompleted, CheckoutSessionDraft,
    CheckoutUrls, CreditLedger, LedgerOutcome, LegalUrls, PrepaidPack, SavedPaymentMethodUse,
    StripeWebhookEvent, UsageDebit, CHECKOUT_TRUST_COPY, TRIAL_DAYS, TRIAL_MINUTES,
    UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY,
};
use conch::web::legal::{
    checkout_disclosure, landing_page, privacy_page, recording_consent_page,
    require_recording_consent, terms_page, CtaLinks, InterviewStartGate, PublicSiteConfig,
    RecordingConsentAcceptance, RECORDING_CONSENT_BUTTON, REQUIRED_BILLING_DISCLOSURE,
};

#[test]
fn setup_mode_checkout_is_card_gate_not_charge_or_subscription() {
    let checkout = CheckoutSessionDraft::trial_setup(
        &checkout_config(false),
        "workspace_1",
        "founder@example.com",
    )
    .unwrap();

    assert_eq!(checkout.mode, CheckoutMode::Setup);
    assert_eq!(checkout.mode.as_str(), "setup");
    assert_eq!(checkout.pack, None);
    assert_eq!(checkout.price_lookup_key, None);
    assert_eq!(
        checkout.payment_method_use,
        Some(SavedPaymentMethodUse::CustomerPresentOnly)
    );
    assert_eq!(checkout.payment_method_use.unwrap().as_str(), "on_session");
    assert_eq!(checkout.trust_copy, CHECKOUT_TRUST_COPY);
    assert!(checkout.trust_copy.contains("No subscription"));
    assert!(checkout.trust_copy.contains("No automatic charge"));
}

#[test]
fn prepaid_packs_are_one_time_payment_checkouts() {
    let cases = [
        (
            PrepaidPack::Starter,
            2_900,
            1_000,
            "conch_starter_1000_stt_minutes",
        ),
        (
            PrepaidPack::Team,
            19_900,
            10_000,
            "conch_team_10000_stt_minutes",
        ),
        (
            PrepaidPack::Pilot,
            49_900,
            30_000,
            "conch_pilot_30000_stt_minutes",
        ),
    ];

    for (pack, cents, minutes, lookup_key) in cases {
        assert_eq!(pack.price_cents(), cents);
        assert_eq!(pack.stt_minutes(), minutes);
        assert_eq!(pack.credit_seconds(), i64::from(minutes) * 60);

        let checkout =
            CheckoutSessionDraft::prepaid_pack(&checkout_config(false), "workspace_1", pack)
                .unwrap();
        assert_eq!(checkout.mode, CheckoutMode::Payment);
        assert_eq!(checkout.mode.as_str(), "payment");
        assert_eq!(checkout.payment_method_use, None);
        assert_eq!(checkout.pack, Some(pack));
        assert_eq!(checkout.price_lookup_key.as_deref(), Some(lookup_key));
        assert!(checkout.trust_copy.contains("prepaid credits"));
    }
}

#[test]
fn unsafe_or_live_placeholder_checkout_shapes_are_rejected() {
    assert_eq!(
        CheckoutMode::try_from("subscription").unwrap_err(),
        BillingError::UnsupportedCheckoutMode("subscription".to_string())
    );

    let err = CheckoutSessionDraft::trial_setup(
        &checkout_config(true),
        "workspace_1",
        "founder@example.com",
    )
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
fn setup_webhook_grants_trial_once_and_sets_card_detach_date() {
    let now = instant();
    let mut ledger = CreditLedger::new();
    let event = setup_completed_event("evt_setup_1", now);

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

    let balance = ledger.balance("workspace_1").unwrap();
    assert_eq!(
        balance.trial_seconds_remaining,
        i64::from(TRIAL_MINUTES) * 60
    );
    assert_eq!(
        balance.trial_expires_at,
        Some(now + Duration::days(TRIAL_DAYS))
    );
    assert_eq!(balance.payment_method_id.as_deref(), Some("pm_trial"));
    assert_eq!(
        balance.payment_method_detach_after,
        Some(now + Duration::days(TRIAL_DAYS + UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY))
    );
}

#[test]
fn paid_pack_webhooks_are_idempotent_and_refunds_cannot_go_negative() {
    let mut ledger = CreditLedger::new();
    let paid = CheckoutSessionCompleted {
        event_id: "evt_paid_1".to_string(),
        session_id: "cs_paid".to_string(),
        mode: CheckoutMode::Payment,
        workspace_id: "workspace_1".to_string(),
        stripe_customer_id: "cus_paid".to_string(),
        payment_method_id: None,
        pack: Some(PrepaidPack::Team),
        completed_at: instant(),
    };

    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(paid.clone())),
        Ok(LedgerOutcome::PaidPackGranted {
            pack: PrepaidPack::Team,
            seconds: 600_000,
        })
    );
    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionCompleted(paid)),
        Ok(LedgerOutcome::DuplicateIgnored)
    );

    assert_eq!(
        ledger.apply(StripeWebhookEvent::PaidPackRefunded {
            event_id: "evt_refund_1".to_string(),
            workspace_id: "workspace_1".to_string(),
            seconds_to_revoke: 999_999,
        }),
        Ok(LedgerOutcome::PaidCreditsRevoked { seconds: 600_000 })
    );
    assert_eq!(
        ledger
            .balance("workspace_1")
            .unwrap()
            .paid_seconds_remaining,
        0
    );
}

#[test]
fn expired_checkouts_and_local_stt_do_not_consume_or_provision_credits() {
    let mut ledger = CreditLedger::new();

    assert_eq!(
        ledger.apply(StripeWebhookEvent::CheckoutSessionExpired {
            event_id: "evt_expired".to_string(),
            session_id: "cs_expired".to_string(),
        }),
        Ok(LedgerOutcome::NoCreditGrant)
    );
    assert!(ledger.balance("workspace_1").is_none());

    ledger
        .apply(StripeWebhookEvent::CheckoutSessionCompleted(
            setup_completed_event("evt_setup_2", instant()),
        ))
        .unwrap();
    let before = ledger
        .balance("workspace_1")
        .unwrap()
        .total_seconds_remaining();
    assert_eq!(
        ledger.debit_usage("workspace_1", UsageDebit::LocalStt { seconds: 600 }),
        Ok(LedgerOutcome::NoCreditGrant)
    );
    assert_eq!(
        ledger
            .balance("workspace_1")
            .unwrap()
            .total_seconds_remaining(),
        before
    );
}

#[test]
fn signup_requires_terms_and_active_session_before_use() {
    assert_eq!(
        SignupRequest::new("founder@example.com", "").unwrap_err(),
        AuthError::MissingTermsAcceptance
    );
    assert_eq!(
        SignupRequest::new("not-an-email", "terms-2026-04-25").unwrap_err(),
        AuthError::InvalidEmail
    );

    let signup = SignupRequest::new(" Founder@Example.com ", "terms-2026-04-25").unwrap();
    assert_eq!(signup.email, "founder@example.com");

    let session = AuthSession {
        principal: WorkspacePrincipal {
            user_id: "user_1".to_string(),
            workspace_id: "workspace_1".to_string(),
            email: signup.email,
        },
        expires_at: instant() + Duration::hours(1),
    };
    assert_eq!(
        require_active_session(Some(&session), instant())
            .unwrap()
            .workspace_id,
        "workspace_1"
    );
    assert_eq!(
        require_active_session(Some(&session), instant() + Duration::hours(2)).unwrap_err(),
        AuthError::ExpiredSession
    );
}

#[test]
fn legal_pages_cover_billing_privacy_and_recording_consent_invariants() {
    let config = PublicSiteConfig::beta_placeholder();
    let landing = landing_page(&config, &CtaLinks::disabled());
    let terms = terms_page(&config);
    let privacy = privacy_page(&config);
    let recording = recording_consent_page(&config);
    let combined = [
        landing.as_str(),
        terms.as_str(),
        privacy.as_str(),
        recording.as_str(),
    ]
    .join("\n");

    assert_eq!(checkout_disclosure(), REQUIRED_BILLING_DISCLOSURE);
    assert!(combined.contains("No subscription"));
    assert!(combined.contains("No automatic charge"));
    assert!(combined.contains("raw card data does not touch Conch servers"));
    assert!(combined.contains("Provider API keys are server-only"));
    assert!(combined.contains(RECORDING_CONSENT_BUTTON));
    assert!(landing.contains("/app/signup"));
    assert!(landing.contains("Open Conch"));

    assert_eq!(
        require_recording_consent(None),
        InterviewStartGate::MissingRecordingConsent
    );
    let consent = RecordingConsentAcceptance::new("workspace_1", instant());
    assert_eq!(
        require_recording_consent(Some(&consent)),
        InterviewStartGate::Allowed
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

fn setup_completed_event(
    event_id: &str,
    completed_at: chrono::DateTime<Utc>,
) -> CheckoutSessionCompleted {
    CheckoutSessionCompleted {
        event_id: event_id.to_string(),
        session_id: "cs_setup".to_string(),
        mode: CheckoutMode::Setup,
        workspace_id: "workspace_1".to_string(),
        stripe_customer_id: "cus_trial".to_string(),
        payment_method_id: Some("pm_trial".to_string()),
        pack: None,
        completed_at,
    }
}

fn instant() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 25, 12, 0, 0)
        .single()
        .unwrap()
}
