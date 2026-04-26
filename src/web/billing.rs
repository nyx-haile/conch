use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, BTreeSet};

pub const TRIAL_MINUTES: u32 = 120;
pub const TRIAL_DAYS: i64 = 14;
pub const UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY: i64 = 30;
pub const CHECKOUT_TRUST_COPY: &str =
    "Card required. No subscription. No automatic charge. Buy prepaid credits only when you need more.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckoutMode {
    Setup,
    Payment,
}

impl CheckoutMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::Payment => "payment",
        }
    }
}

impl TryFrom<&str> for CheckoutMode {
    type Error = BillingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "setup" => Ok(Self::Setup),
            "payment" => Ok(Self::Payment),
            other => Err(BillingError::UnsupportedCheckoutMode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedPaymentMethodUse {
    CustomerPresentOnly,
}

impl SavedPaymentMethodUse {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CustomerPresentOnly => "on_session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepaidPack {
    Starter,
    Team,
    Pilot,
}

impl PrepaidPack {
    pub fn lookup_key(self) -> &'static str {
        match self {
            Self::Starter => "conch_starter_1000_stt_minutes",
            Self::Team => "conch_team_10000_stt_minutes",
            Self::Pilot => "conch_pilot_30000_stt_minutes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Starter => "Starter",
            Self::Team => "Team",
            Self::Pilot => "Pilot",
        }
    }

    pub fn price_cents(self) -> u32 {
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

    pub fn credit_seconds(self) -> i64 {
        i64::from(self.stt_minutes()) * 60
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutUrls {
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalUrls {
    pub terms_url: String,
    pub privacy_url: String,
    pub recording_consent_url: String,
    pub support_email: String,
    pub billing_email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutConfig {
    pub urls: CheckoutUrls,
    pub legal: LegalUrls,
    pub live_payments_enabled: bool,
}

impl CheckoutConfig {
    pub fn validate(&self) -> Result<(), BillingError> {
        if !self.live_payments_enabled {
            return Ok(());
        }

        for (label, value) in [
            ("success_url", self.urls.success_url.as_str()),
            ("cancel_url", self.urls.cancel_url.as_str()),
            ("terms_url", self.legal.terms_url.as_str()),
            ("privacy_url", self.legal.privacy_url.as_str()),
            (
                "recording_consent_url",
                self.legal.recording_consent_url.as_str(),
            ),
            ("support_email", self.legal.support_email.as_str()),
            ("billing_email", self.legal.billing_email.as_str()),
        ] {
            if is_placeholder(value) {
                return Err(BillingError::LivePaymentPlaceholder {
                    field: label,
                    value: value.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutSessionDraft {
    pub mode: CheckoutMode,
    pub success_url: String,
    pub cancel_url: String,
    pub customer_email: Option<String>,
    pub workspace_id: String,
    pub pack: Option<PrepaidPack>,
    pub price_lookup_key: Option<String>,
    pub payment_method_use: Option<SavedPaymentMethodUse>,
    pub trust_copy: &'static str,
    pub terms_url: String,
    pub privacy_url: String,
    pub recording_consent_url: String,
}

impl CheckoutSessionDraft {
    pub fn trial_setup(
        config: &CheckoutConfig,
        workspace_id: impl Into<String>,
        customer_email: impl Into<String>,
    ) -> Result<Self, BillingError> {
        config.validate()?;
        Ok(Self {
            mode: CheckoutMode::Setup,
            success_url: config.urls.success_url.clone(),
            cancel_url: config.urls.cancel_url.clone(),
            customer_email: Some(customer_email.into()),
            workspace_id: workspace_id.into(),
            pack: None,
            price_lookup_key: None,
            payment_method_use: Some(SavedPaymentMethodUse::CustomerPresentOnly),
            trust_copy: CHECKOUT_TRUST_COPY,
            terms_url: config.legal.terms_url.clone(),
            privacy_url: config.legal.privacy_url.clone(),
            recording_consent_url: config.legal.recording_consent_url.clone(),
        })
    }

    pub fn prepaid_pack(
        config: &CheckoutConfig,
        workspace_id: impl Into<String>,
        pack: PrepaidPack,
    ) -> Result<Self, BillingError> {
        config.validate()?;
        Ok(Self {
            mode: CheckoutMode::Payment,
            success_url: config.urls.success_url.clone(),
            cancel_url: config.urls.cancel_url.clone(),
            customer_email: None,
            workspace_id: workspace_id.into(),
            pack: Some(pack),
            price_lookup_key: Some(pack.lookup_key().to_string()),
            payment_method_use: None,
            trust_copy: CHECKOUT_TRUST_COPY,
            terms_url: config.legal.terms_url.clone(),
            privacy_url: config.legal.privacy_url.clone(),
            recording_consent_url: config.legal.recording_consent_url.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripeWebhookEvent {
    CheckoutSessionCompleted(CheckoutSessionCompleted),
    CheckoutSessionExpired {
        event_id: String,
        session_id: String,
    },
    PaidPackRefunded {
        event_id: String,
        workspace_id: String,
        seconds_to_revoke: i64,
    },
}

impl StripeWebhookEvent {
    fn event_id(&self) -> &str {
        match self {
            Self::CheckoutSessionCompleted(event) => &event.event_id,
            Self::CheckoutSessionExpired { event_id, .. } => event_id,
            Self::PaidPackRefunded { event_id, .. } => event_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutSessionCompleted {
    pub event_id: String,
    pub session_id: String,
    pub mode: CheckoutMode,
    pub workspace_id: String,
    pub stripe_customer_id: String,
    pub payment_method_id: Option<String>,
    pub pack: Option<PrepaidPack>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditLedger {
    balances: BTreeMap<String, WorkspaceCreditBalance>,
    processed_event_ids: BTreeSet<String>,
}

impl CreditLedger {
    pub fn new() -> Self {
        Self {
            balances: BTreeMap::new(),
            processed_event_ids: BTreeSet::new(),
        }
    }

    pub fn balance(&self, workspace_id: &str) -> Option<&WorkspaceCreditBalance> {
        self.balances.get(workspace_id)
    }

    pub fn apply(&mut self, event: StripeWebhookEvent) -> Result<LedgerOutcome, BillingError> {
        let event_id = event.event_id().to_string();
        if !self.processed_event_ids.insert(event_id) {
            return Ok(LedgerOutcome::DuplicateIgnored);
        }

        match event {
            StripeWebhookEvent::CheckoutSessionCompleted(event) => {
                self.apply_completed_checkout(event)
            }
            StripeWebhookEvent::CheckoutSessionExpired { .. } => Ok(LedgerOutcome::NoCreditGrant),
            StripeWebhookEvent::PaidPackRefunded {
                workspace_id,
                seconds_to_revoke,
                ..
            } => {
                let balance = self
                    .balances
                    .entry(workspace_id.clone())
                    .or_insert_with(|| WorkspaceCreditBalance::empty(workspace_id));
                let before = balance.paid_seconds_remaining;
                balance.paid_seconds_remaining =
                    balance.paid_seconds_remaining.saturating_sub(seconds_to_revoke.max(0));
                Ok(LedgerOutcome::PaidCreditsRevoked {
                    seconds: before - balance.paid_seconds_remaining,
                })
            }
        }
    }

    pub fn debit_usage(
        &mut self,
        workspace_id: &str,
        usage: UsageDebit,
    ) -> Result<LedgerOutcome, BillingError> {
        match usage {
            UsageDebit::LocalStt { .. } => Ok(LedgerOutcome::NoCreditGrant),
            UsageDebit::DeepgramCloudStt { seconds } => {
                let balance = self
                    .balances
                    .get_mut(workspace_id)
                    .ok_or_else(|| BillingError::UnknownWorkspace(workspace_id.to_string()))?;
                balance.debit_cloud_stt(seconds)
            }
        }
    }

    fn apply_completed_checkout(
        &mut self,
        event: CheckoutSessionCompleted,
    ) -> Result<LedgerOutcome, BillingError> {
        let balance = self
            .balances
            .entry(event.workspace_id.clone())
            .or_insert_with(|| WorkspaceCreditBalance::empty(event.workspace_id.clone()));
        balance.stripe_customer_id = Some(event.stripe_customer_id);

        match event.mode {
            CheckoutMode::Setup => {
                if event.pack.is_some() {
                    return Err(BillingError::SetupCheckoutCannotGrantPaidPack);
                }
                if balance.trial_granted_at.is_some() {
                    return Ok(LedgerOutcome::DuplicateIgnored);
                }
                balance.trial_seconds_remaining = i64::from(TRIAL_MINUTES) * 60;
                balance.trial_granted_at = Some(event.completed_at);
                balance.trial_expires_at = Some(event.completed_at + Duration::days(TRIAL_DAYS));
                balance.payment_method_id = event.payment_method_id;
                balance.payment_method_detach_after = Some(
                    event.completed_at
                        + Duration::days(
                            TRIAL_DAYS + UNUSED_TRIAL_CARD_DETACH_DAYS_AFTER_EXPIRY,
                        ),
                );
                Ok(LedgerOutcome::TrialGranted {
                    seconds: balance.trial_seconds_remaining,
                })
            }
            CheckoutMode::Payment => {
                let pack = event.pack.ok_or(BillingError::MissingPaidPack)?;
                let seconds = pack.credit_seconds();
                balance.paid_seconds_remaining += seconds;
                balance.paid_grants.push(PaidGrant {
                    pack,
                    seconds,
                    stripe_session_id: event.session_id,
                    granted_at: event.completed_at,
                });
                Ok(LedgerOutcome::PaidPackGranted { pack, seconds })
            }
        }
    }
}

impl Default for CreditLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreditBalance {
    pub workspace_id: String,
    pub stripe_customer_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub payment_method_detach_after: Option<DateTime<Utc>>,
    pub trial_seconds_remaining: i64,
    pub trial_granted_at: Option<DateTime<Utc>>,
    pub trial_expires_at: Option<DateTime<Utc>>,
    pub paid_seconds_remaining: i64,
    pub paid_grants: Vec<PaidGrant>,
}

impl WorkspaceCreditBalance {
    fn empty(workspace_id: String) -> Self {
        Self {
            workspace_id,
            stripe_customer_id: None,
            payment_method_id: None,
            payment_method_detach_after: None,
            trial_seconds_remaining: 0,
            trial_granted_at: None,
            trial_expires_at: None,
            paid_seconds_remaining: 0,
            paid_grants: Vec::new(),
        }
    }

    pub fn total_seconds_remaining(&self) -> i64 {
        self.trial_seconds_remaining + self.paid_seconds_remaining
    }

    fn debit_cloud_stt(&mut self, seconds: i64) -> Result<LedgerOutcome, BillingError> {
        if seconds <= 0 {
            return Ok(LedgerOutcome::UsageDebited { seconds: 0 });
        }
        if self.total_seconds_remaining() < seconds {
            return Err(BillingError::InsufficientCredits {
                requested_seconds: seconds,
                available_seconds: self.total_seconds_remaining(),
            });
        }

        let trial_debit = self.trial_seconds_remaining.min(seconds);
        self.trial_seconds_remaining -= trial_debit;
        let paid_debit = seconds - trial_debit;
        self.paid_seconds_remaining -= paid_debit;

        Ok(LedgerOutcome::UsageDebited { seconds })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaidGrant {
    pub pack: PrepaidPack,
    pub seconds: i64,
    pub stripe_session_id: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageDebit {
    DeepgramCloudStt { seconds: i64 },
    LocalStt { seconds: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerOutcome {
    TrialGranted { seconds: i64 },
    PaidPackGranted { pack: PrepaidPack, seconds: i64 },
    PaidCreditsRevoked { seconds: i64 },
    UsageDebited { seconds: i64 },
    DuplicateIgnored,
    NoCreditGrant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingError {
    UnsupportedCheckoutMode(String),
    LivePaymentPlaceholder {
        field: &'static str,
        value: String,
    },
    SetupCheckoutCannotGrantPaidPack,
    MissingPaidPack,
    UnknownWorkspace(String),
    InsufficientCredits {
        requested_seconds: i64,
        available_seconds: i64,
    },
}

fn is_placeholder(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.is_empty()
        || value.contains("example.com")
        || value.contains("placeholder")
        || value.contains("sign_up_url")
        || value.contains("signup_url")
        || value.contains("todo")
}
