mod web {
    pub mod billing {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/web/billing.rs"));
    }

    pub mod auth {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/web/auth.rs"));
    }

    pub mod legal {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/web/legal.rs"));
    }
}

use web::{auth, legal};

use chrono::{Duration, TimeZone, Utc};
use web::auth::{
    start_signup, AccountStatus, AuthError, SignupRequest, WorkspaceTier, CURRENT_PRIVACY_VERSION,
    CURRENT_RECORDING_CONSENT_VERSION, CURRENT_TERMS_VERSION,
};
use web::billing::{
    prepaid_pack_checkout, trial_setup_checkout, BillingError, BillingLedger, PrepaidPack,
    SetupIntentUsage, StripeCheckoutMode, StripeWebhookEvent, WebhookOutcome,
    STRIPE_CARD_STORAGE_DISCLOSURE, TRIAL_DAYS, TRIAL_STT_MINUTES,
    UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY,
};

fn valid_signup(email: &str) -> SignupRequest {
    SignupRequest {
        email: email.to_owned(),
        accepted_terms_version: Some(CURRENT_TERMS_VERSION.to_owned()),
        accepted_privacy_version: Some(CURRENT_PRIVACY_VERSION.to_owned()),
        accepted_recording_consent_version: Some(CURRENT_RECORDING_CONSENT_VERSION.to_owned()),
    }
}

#[test]
fn setup_mode_checkout_is_card_gate_not_charge_or_subscription() {
    let checkout = trial_setup_checkout(
        "cus_trial",
        "workspace_1",
        "https://conch.test/app?setup=success",
        "https://conch.test/pricing?setup=cancelled",
    );

    assert_eq!(checkout.mode, StripeCheckoutMode::Setup);
    assert_eq!(
        checkout.setup_intent_usage,
        Some(SetupIntentUsage::OnSession)
    );
    assert!(checkout.line_items.is_empty());
    assert!(!checkout.allows_subscription);
    assert!(!checkout.allows_automatic_charge);
    assert!(!checkout.stores_raw_card_data);
    assert!(checkout.consent_copy.contains("No subscription"));
    assert!(checkout.consent_copy.contains("No automatic charge"));
    checkout.assert_launch_safe().unwrap();
}

#[test]
fn prepaid_packs_are_one_time_payment_checkouts() {
    let cases = [
        (
            PrepaidPack::Starter,
            2_900,
            1_000,
            "CONCH_STRIPE_STARTER_PRICE_ID",
        ),
        (
            PrepaidPack::Team,
            19_900,
            10_000,
            "CONCH_STRIPE_TEAM_PRICE_ID",
        ),
        (
            PrepaidPack::Pilot,
            49_900,
            30_000,
            "CONCH_STRIPE_PILOT_PRICE_ID",
        ),
    ];

    for (pack, cents, minutes, price_env) in cases {
        assert_eq!(pack.amount_cents(), cents);
        assert_eq!(pack.stt_minutes(), minutes);
        let checkout = prepaid_pack_checkout(
            pack,
            "cus_paid",
            "workspace_paid",
            "https://conch.test/app?payment=success",
            "https://conch.test/pricing?payment=cancelled",
        );
        assert_eq!(checkout.mode, StripeCheckoutMode::Payment);
        assert_eq!(checkout.setup_intent_usage, None);
        assert_eq!(checkout.line_items.len(), 1);
        assert_eq!(checkout.line_items[0].price_env, price_env);
        assert!(!checkout.allows_subscription);
        assert!(!checkout.allows_automatic_charge);
        assert!(checkout.consent_copy.contains("One-time prepaid"));
        checkout.assert_launch_safe().unwrap();
    }
}

#[test]
fn unsafe_checkout_shapes_are_rejected() {
    let mut setup = trial_setup_checkout("cus", "workspace", "success", "cancel");
    setup.mode = StripeCheckoutMode::Subscription;
    assert_eq!(
        setup.assert_launch_safe(),
        Err(BillingError::SubscriptionNotAllowed)
    );

    let mut off_session = trial_setup_checkout("cus", "workspace", "success", "cancel");
    off_session.setup_intent_usage = Some(SetupIntentUsage::OffSession);
    assert_eq!(
        off_session.assert_launch_safe(),
        Err(BillingError::AutomaticChargeNotAllowed)
    );
}

#[test]
fn setup_webhook_grants_trial_once_and_sets_card_detach_date() {
    let now = Utc.with_ymd_and_hms(2026, 4, 25, 12, 0, 0).unwrap();
    let mut ledger = BillingLedger::new("workspace_1", "cus_trial");
    let event = StripeWebhookEvent::CheckoutSessionCompletedSetup {
        event_id: "evt_setup_1".to_owned(),
        checkout_session_id: "cs_setup_1".to_owned(),
        stripe_customer_id: "cus_trial".to_owned(),
        setup_intent_id: "seti_1".to_owned(),
        payment_method_id: "pm_1".to_owned(),
    };

    let outcome = ledger.process_webhook(event.clone(), now).unwrap();
    assert_eq!(outcome, WebhookOutcome::GrantedTrial(TRIAL_STT_MINUTES));
    assert!(ledger.trial_card_verified);
    assert_eq!(ledger.trial_minutes_remaining, TRIAL_STT_MINUTES);
    assert_eq!(ledger.grants.len(), 1);
    assert_eq!(
        ledger.grants[0].expires_at,
        now + Duration::days(TRIAL_DAYS)
    );
    assert_eq!(
        ledger.grants[0].payment_method_detach_after,
        Some(now + Duration::days(TRIAL_DAYS + UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY))
    );

    let duplicate = ledger.process_webhook(event, now).unwrap();
    assert_eq!(duplicate, WebhookOutcome::DuplicateIgnored);
    assert_eq!(ledger.trial_minutes_remaining, TRIAL_STT_MINUTES);
    assert_eq!(ledger.grants.len(), 1);
}

#[test]
fn paid_pack_webhooks_are_idempotent_across_checkout_and_payment_intent_events() {
    let now = Utc.with_ymd_and_hms(2026, 4, 25, 13, 0, 0).unwrap();
    let mut ledger = BillingLedger::new("workspace_1", "cus_paid");

    let checkout_event = StripeWebhookEvent::CheckoutSessionCompletedPayment {
        event_id: "evt_checkout_paid".to_owned(),
        checkout_session_id: Some("cs_paid".to_owned()),
        stripe_customer_id: "cus_paid".to_owned(),
        payment_intent_id: "pi_paid".to_owned(),
        pack: PrepaidPack::Team,
    };
    assert_eq!(
        ledger.process_webhook(checkout_event, now).unwrap(),
        WebhookOutcome::GrantedPaidPack {
            pack: PrepaidPack::Team,
            minutes: 10_000,
        }
    );

    let payment_intent_event = StripeWebhookEvent::PaymentIntentSucceeded {
        event_id: "evt_pi_paid".to_owned(),
        payment_intent_id: "pi_paid".to_owned(),
        stripe_customer_id: "cus_paid".to_owned(),
        checkout_session_id: Some("cs_paid".to_owned()),
        pack: PrepaidPack::Team,
    };
    assert_eq!(
        ledger.process_webhook(payment_intent_event, now).unwrap(),
        WebhookOutcome::DuplicateIgnored
    );
    assert_eq!(ledger.paid_minutes_remaining, 10_000);
    assert_eq!(ledger.grants.len(), 1);
}

#[test]
fn expired_checkout_and_subscription_events_do_not_provision_credits() {
    let now = Utc.with_ymd_and_hms(2026, 4, 25, 14, 0, 0).unwrap();
    let mut ledger = BillingLedger::new("workspace_1", "cus_trial");

    let expired = StripeWebhookEvent::CheckoutSessionExpired {
        event_id: "evt_expired".to_owned(),
        checkout_session_id: "cs_expired".to_owned(),
        stripe_customer_id: "cus_trial".to_owned(),
    };
    assert_eq!(
        ledger.process_webhook(expired, now).unwrap(),
        WebhookOutcome::NoGrantExpired
    );
    assert_eq!(ledger.trial_minutes_remaining, 0);

    let subscription = StripeWebhookEvent::SubscriptionCreated {
        event_id: "evt_sub".to_owned(),
        stripe_customer_id: "cus_trial".to_owned(),
        subscription_id: "sub_1".to_owned(),
    };
    assert_eq!(
        ledger.process_webhook(subscription, now),
        Err(BillingError::SubscriptionNotAllowed)
    );
}

#[test]
fn signup_requires_current_legal_acceptance_before_card_gate() {
    let now = Utc.with_ymd_and_hms(2026, 4, 25, 15, 0, 0).unwrap();
    let missing = SignupRequest {
        email: "founder@example.com".to_owned(),
        accepted_terms_version: Some(CURRENT_TERMS_VERSION.to_owned()),
        accepted_privacy_version: None,
        accepted_recording_consent_version: Some(CURRENT_RECORDING_CONSENT_VERSION.to_owned()),
    };
    assert_eq!(
        start_signup(missing, "user_1", "workspace_1", "cus_1", now),
        Err(AuthError::MissingLegalAcceptance {
            label: "privacy",
            expected: CURRENT_PRIVACY_VERSION,
        })
    );

    let onboarding = start_signup(
        valid_signup(" Founder@Example.com "),
        "user_1",
        "workspace_1",
        "cus_1",
        now,
    )
    .unwrap();
    assert_eq!(onboarding.email, "founder@example.com");
    assert_eq!(onboarding.status, AccountStatus::PendingCardSetup);
    assert_eq!(onboarding.tier, WorkspaceTier::Trial);
    assert!(onboarding
        .billing_disclosure
        .contains("No automatic charge"));

    let checkout = onboarding.checkout_for_card_gate("success", "cancel");
    assert_eq!(checkout.mode, StripeCheckoutMode::Setup);
    checkout.assert_launch_safe().unwrap();
}

#[test]
fn auth_surface_exposes_legal_links_before_payment() {
    let copy = auth::auth_surface_copy();
    assert!(copy.primary_cta.contains("card required"));
    assert!(copy.billing_disclosure.contains("No subscription"));
    let hrefs: Vec<_> = copy.legal_links.iter().map(|link| link.href).collect();
    assert!(hrefs.contains(&"/terms.html"));
    assert!(hrefs.contains(&"/privacy.html"));
    assert!(hrefs.contains(&"/recording-consent.html"));
}

#[test]
fn legal_pages_cover_billing_privacy_and_recording_consent_invariants() {
    let pages = legal::legal_pages();
    let combined = pages
        .iter()
        .map(|page| page.body)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(combined.contains("No subscription"));
    assert!(combined.contains("No automatic charge"));
    assert!(combined.contains(STRIPE_CARD_STORAGE_DISCLOSURE));
    assert!(combined.contains("permission to record and transcribe"));
    assert!(combined.contains("Provider API keys stay server-side"));

    let links = legal::legal_footer_links();
    assert!(links.contains(&("Terms", "/terms.html")));
    assert!(links.contains(&("Privacy", "/privacy.html")));
    assert!(links.contains(&("Recording Consent", "/recording-consent.html")));

    let trust_copy = legal::payment_trust_copy().join(" ");
    assert!(trust_copy.contains("No automatic renewal"));
    assert!(trust_copy.contains("Public payment links must stay disabled"));
}
