//! Billing primitives for the Conch web launch.
//!
//! The launch billing contract is intentionally narrow: a card-gated free
//! trial through Stripe Checkout setup mode and explicit one-time prepaid
//! packs. This module is framework-agnostic so route handlers can map these
//! specs onto Stripe API calls without ever touching raw card data or creating
//! subscriptions.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const TRIAL_STT_MINUTES: u32 = 120;
pub const TRIAL_DAYS: i64 = 14;
pub const UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY: i64 = 30;
pub const BILLING_DISCLOSURE: &str =
    "Card required for the free trial. No subscription. No automatic charge.";
pub const STRIPE_CARD_STORAGE_DISCLOSURE: &str =
    "Stripe stores payment methods; Conch never receives or stores raw card data.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeCheckoutMode {
    Setup,
    Payment,
    Subscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupIntentUsage {
    OnSession,
    OffSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepaidPack {
    Starter,
    Team,
    Pilot,
}

impl PrepaidPack {
    pub fn sku(self) -> &'static str {
        match self {
            Self::Starter => "starter_pack",
            Self::Team => "team_pack",
            Self::Pilot => "pilot_pack",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Starter => "Starter pack",
            Self::Team => "Team pack",
            Self::Pilot => "Pilot concierge pack",
        }
    }

    pub fn amount_cents(self) -> u32 {
        match self {
            Self::Starter => 2_900,
            Self::Team => 19_900,
            Self::Pilot => 49_900,
        }
    }

    pub fn stt_minutes(self) -> u32 {
        match self {
            Self::Starter => 1_000,
            Self::Team => 10_000,
            Self::Pilot => 30_000,
        }
    }

    pub fn stripe_price_env(self) -> &'static str {
        match self {
            Self::Starter => "CONCH_STRIPE_STARTER_PRICE_ID",
            Self::Team => "CONCH_STRIPE_TEAM_PRICE_ID",
            Self::Pilot => "CONCH_STRIPE_PILOT_PRICE_ID",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutLineItem {
    pub price_env: &'static str,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StripeCheckoutSpec {
    pub mode: StripeCheckoutMode,
    pub stripe_customer_id: String,
    pub workspace_id: String,
    pub success_url: String,
    pub cancel_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_intent_usage: Option<SetupIntentUsage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_items: Vec<CheckoutLineItem>,
    pub consent_copy: &'static str,
    pub stores_raw_card_data: bool,
    pub allows_subscription: bool,
    pub allows_automatic_charge: bool,
}

impl StripeCheckoutSpec {
    pub fn assert_launch_safe(&self) -> Result<(), BillingError> {
        if self.mode == StripeCheckoutMode::Subscription || self.allows_subscription {
            return Err(BillingError::SubscriptionNotAllowed);
        }
        if self.mode == StripeCheckoutMode::Setup {
            if self.setup_intent_usage != Some(SetupIntentUsage::OnSession) {
                return Err(BillingError::AutomaticChargeNotAllowed);
            }
            if !self.line_items.is_empty() {
                return Err(BillingError::SetupCheckoutMustNotCharge);
            }
        }
        if self.mode == StripeCheckoutMode::Payment && self.line_items.is_empty() {
            return Err(BillingError::PaymentCheckoutRequiresPack);
        }
        if self.stores_raw_card_data {
            return Err(BillingError::RawCardDataNotAllowed);
        }
        if self.allows_automatic_charge {
            return Err(BillingError::AutomaticChargeNotAllowed);
        }
        Ok(())
    }
}

pub fn trial_setup_checkout(
    stripe_customer_id: impl Into<String>,
    workspace_id: impl Into<String>,
    success_url: impl Into<String>,
    cancel_url: impl Into<String>,
) -> StripeCheckoutSpec {
    StripeCheckoutSpec {
        mode: StripeCheckoutMode::Setup,
        stripe_customer_id: stripe_customer_id.into(),
        workspace_id: workspace_id.into(),
        success_url: success_url.into(),
        cancel_url: cancel_url.into(),
        setup_intent_usage: Some(SetupIntentUsage::OnSession),
        line_items: vec![],
        consent_copy: BILLING_DISCLOSURE,
        stores_raw_card_data: false,
        allows_subscription: false,
        allows_automatic_charge: false,
    }
}

pub fn prepaid_pack_checkout(
    pack: PrepaidPack,
    stripe_customer_id: impl Into<String>,
    workspace_id: impl Into<String>,
    success_url: impl Into<String>,
    cancel_url: impl Into<String>,
) -> StripeCheckoutSpec {
    StripeCheckoutSpec {
        mode: StripeCheckoutMode::Payment,
        stripe_customer_id: stripe_customer_id.into(),
        workspace_id: workspace_id.into(),
        success_url: success_url.into(),
        cancel_url: cancel_url.into(),
        setup_intent_usage: None,
        line_items: vec![CheckoutLineItem {
            price_env: pack.stripe_price_env(),
            quantity: 1,
        }],
        consent_copy: "One-time prepaid usage pack. No subscription. No automatic renewal.",
        stores_raw_card_data: false,
        allows_subscription: false,
        allows_automatic_charge: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditGrant {
    pub grant_source: GrantSource,
    pub stt_minutes: u32,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub stripe_checkout_session_id: Option<String>,
    pub stripe_setup_intent_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub payment_method_detach_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantSource {
    Trial,
    StarterPack,
    TeamPack,
    PilotPack,
    ManualAdjustment,
}

impl From<PrepaidPack> for GrantSource {
    fn from(pack: PrepaidPack) -> Self {
        match pack {
            PrepaidPack::Starter => Self::StarterPack,
            PrepaidPack::Team => Self::TeamPack,
            PrepaidPack::Pilot => Self::PilotPack,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingLedger {
    pub workspace_id: String,
    pub stripe_customer_id: String,
    pub trial_card_verified: bool,
    pub trial_minutes_remaining: u32,
    pub paid_minutes_remaining: u32,
    pub grants: Vec<CreditGrant>,
    #[serde(default)]
    processed_stripe_event_ids: HashSet<String>,
    #[serde(default)]
    processed_grant_keys: HashSet<String>,
}

impl BillingLedger {
    pub fn new(workspace_id: impl Into<String>, stripe_customer_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            stripe_customer_id: stripe_customer_id.into(),
            trial_card_verified: false,
            trial_minutes_remaining: 0,
            paid_minutes_remaining: 0,
            grants: vec![],
            processed_stripe_event_ids: HashSet::new(),
            processed_grant_keys: HashSet::new(),
        }
    }

    pub fn process_webhook(
        &mut self,
        event: StripeWebhookEvent,
        now: DateTime<Utc>,
    ) -> Result<WebhookOutcome, BillingError> {
        let event_id = event.event_id().to_owned();
        if !self.processed_stripe_event_ids.insert(event_id) {
            return Ok(WebhookOutcome::DuplicateIgnored);
        }

        match event {
            StripeWebhookEvent::CheckoutSessionCompletedSetup {
                checkout_session_id,
                stripe_customer_id,
                setup_intent_id,
                payment_method_id: _,
                ..
            } => {
                self.ensure_customer(&stripe_customer_id)?;
                let grant_key = format!("trial:{checkout_session_id}:{setup_intent_id}");
                if !self.processed_grant_keys.insert(grant_key) {
                    return Ok(WebhookOutcome::DuplicateIgnored);
                }
                if self.trial_card_verified {
                    return Ok(WebhookOutcome::NoGrantAlreadyProvisioned);
                }
                let expires_at = now + Duration::days(TRIAL_DAYS);
                let payment_method_detach_after =
                    expires_at + Duration::days(UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY);
                self.trial_card_verified = true;
                self.trial_minutes_remaining = self
                    .trial_minutes_remaining
                    .saturating_add(TRIAL_STT_MINUTES);
                self.grants.push(CreditGrant {
                    grant_source: GrantSource::Trial,
                    stt_minutes: TRIAL_STT_MINUTES,
                    granted_at: now,
                    expires_at,
                    stripe_checkout_session_id: Some(checkout_session_id),
                    stripe_setup_intent_id: Some(setup_intent_id),
                    stripe_payment_intent_id: None,
                    payment_method_detach_after: Some(payment_method_detach_after),
                });
                Ok(WebhookOutcome::GrantedTrial(TRIAL_STT_MINUTES))
            }
            StripeWebhookEvent::SetupIntentSucceeded {
                setup_intent_id,
                stripe_customer_id,
                ..
            } => {
                self.ensure_customer(&stripe_customer_id)?;
                let grant_key = format!("setup_intent_seen:{setup_intent_id}");
                self.processed_grant_keys.insert(grant_key);
                Ok(WebhookOutcome::CardVerifiedReconciled)
            }
            StripeWebhookEvent::CheckoutSessionCompletedPayment {
                checkout_session_id,
                stripe_customer_id,
                payment_intent_id,
                pack,
                ..
            }
            | StripeWebhookEvent::PaymentIntentSucceeded {
                payment_intent_id,
                stripe_customer_id,
                pack,
                checkout_session_id,
                ..
            } => {
                self.ensure_customer(&stripe_customer_id)?;
                let grant_key = format!("paid:{payment_intent_id}");
                if !self.processed_grant_keys.insert(grant_key) {
                    return Ok(WebhookOutcome::DuplicateIgnored);
                }
                let minutes = pack.stt_minutes();
                self.paid_minutes_remaining = self.paid_minutes_remaining.saturating_add(minutes);
                self.grants.push(CreditGrant {
                    grant_source: GrantSource::from(pack),
                    stt_minutes: minutes,
                    granted_at: now,
                    expires_at: now + Duration::days(365),
                    stripe_checkout_session_id: checkout_session_id,
                    stripe_setup_intent_id: None,
                    stripe_payment_intent_id: Some(payment_intent_id),
                    payment_method_detach_after: None,
                });
                Ok(WebhookOutcome::GrantedPaidPack { pack, minutes })
            }
            StripeWebhookEvent::RefundSucceeded {
                stripe_customer_id,
                unused_minutes_to_revoke,
                ..
            } => {
                self.ensure_customer(&stripe_customer_id)?;
                let revoked = unused_minutes_to_revoke.min(self.paid_minutes_remaining);
                self.paid_minutes_remaining -= revoked;
                Ok(WebhookOutcome::RevokedUnusedPaidMinutes(revoked))
            }
            StripeWebhookEvent::CheckoutSessionExpired { .. } => Ok(WebhookOutcome::NoGrantExpired),
            StripeWebhookEvent::SubscriptionCreated { .. } => Err(BillingError::SubscriptionNotAllowed),
        }
    }

    fn ensure_customer(&self, stripe_customer_id: &str) -> Result<(), BillingError> {
        if self.stripe_customer_id == stripe_customer_id {
            Ok(())
        } else {
            Err(BillingError::CustomerMismatch)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StripeWebhookEvent {
    CheckoutSessionCompletedSetup {
        event_id: String,
        checkout_session_id: String,
        stripe_customer_id: String,
        setup_intent_id: String,
        payment_method_id: String,
    },
    SetupIntentSucceeded {
        event_id: String,
        setup_intent_id: String,
        stripe_customer_id: String,
        payment_method_id: String,
    },
    CheckoutSessionCompletedPayment {
        event_id: String,
        checkout_session_id: Option<String>,
        stripe_customer_id: String,
        payment_intent_id: String,
        pack: PrepaidPack,
    },
    PaymentIntentSucceeded {
        event_id: String,
        payment_intent_id: String,
        stripe_customer_id: String,
        checkout_session_id: Option<String>,
        pack: PrepaidPack,
    },
    RefundSucceeded {
        event_id: String,
        stripe_customer_id: String,
        payment_intent_id: String,
        unused_minutes_to_revoke: u32,
    },
    CheckoutSessionExpired {
        event_id: String,
        checkout_session_id: String,
        stripe_customer_id: String,
    },
    SubscriptionCreated {
        event_id: String,
        stripe_customer_id: String,
        subscription_id: String,
    },
}

impl StripeWebhookEvent {
    pub fn event_id(&self) -> &str {
        match self {
            Self::CheckoutSessionCompletedSetup { event_id, .. }
            | Self::SetupIntentSucceeded { event_id, .. }
            | Self::CheckoutSessionCompletedPayment { event_id, .. }
            | Self::PaymentIntentSucceeded { event_id, .. }
            | Self::RefundSucceeded { event_id, .. }
            | Self::CheckoutSessionExpired { event_id, .. }
            | Self::SubscriptionCreated { event_id, .. } => event_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookOutcome {
    GrantedTrial(u32),
    GrantedPaidPack { pack: PrepaidPack, minutes: u32 },
    RevokedUnusedPaidMinutes(u32),
    CardVerifiedReconciled,
    DuplicateIgnored,
    NoGrantExpired,
    NoGrantAlreadyProvisioned,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BillingError {
    #[error("Stripe subscriptions are not allowed in the Conch launch billing path")]
    SubscriptionNotAllowed,
    #[error("setup-mode Checkout must not include line items or charge the card")]
    SetupCheckoutMustNotCharge,
    #[error("payment-mode Checkout requires an explicit prepaid pack line item")]
    PaymentCheckoutRequiresPack,
    #[error("automatic or off-session charges are not allowed for the launch card gate")]
    AutomaticChargeNotAllowed,
    #[error("Conch must never store raw card data")]
    RawCardDataNotAllowed,
    #[error("Stripe customer does not match this workspace ledger")]
    CustomerMismatch,
}
