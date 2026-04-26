//! Framework-agnostic data-model and policy primitives for the web launch.
//!
//! These structs mirror the approved launch data sketch while keeping behavior
//! testable without a database, Stripe SDK, or web framework. Persistence layers
//! should map these records one-to-one to tables/collections and keep provider
//! secrets outside all browser-facing payloads.

use super::contracts::{CreditsRemaining, LlmTurnUsage, WebSpeaker};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const FREE_DEFAULT_MODEL_SLUG: &str = "x-ai/grok-4-fast";
pub const DEFAULT_TRIAL_STT_SECONDS: u64 = 120 * 60;
pub const DEFAULT_TRIAL_TTS_CHARS: u64 = 60_000;
pub const DEFAULT_TRIAL_LLM_BUDGET_CENTS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTier {
    Trial,
    Free,
    Paid,
    Pilot,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub tier: WorkspaceTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Free,
    Balanced,
    Premium,
    Byok,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementsRecord {
    pub workspace_id: String,
    pub trial_stt_seconds: u64,
    pub paid_stt_seconds: u64,
    pub llm_budget_cents: u64,
    pub tts_char_budget: u64,
    pub model_tier: ModelTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl EntitlementsRecord {
    pub fn trial_default(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            trial_stt_seconds: DEFAULT_TRIAL_STT_SECONDS,
            paid_stt_seconds: 0,
            llm_budget_cents: DEFAULT_TRIAL_LLM_BUDGET_CENTS,
            tts_char_budget: DEFAULT_TRIAL_TTS_CHARS,
            model_tier: ModelTier::Free,
            expires_at: None,
        }
    }

    pub fn total_stt_seconds(&self) -> u64 {
        self.trial_stt_seconds + self.paid_stt_seconds
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|expires_at| now >= expires_at)
    }

    pub fn credits_remaining(&self, usage: &UsageTotals) -> CreditsRemaining {
        CreditsRemaining {
            stt_seconds: self.total_stt_seconds().saturating_sub(usage.stt_seconds),
            tts_chars: self.tts_char_budget.saturating_sub(usage.tts_chars),
            llm_budget_cents: self.llm_budget_cents.saturating_sub(usage.llm_cost_cents),
        }
    }

    pub fn can_access_tier(&self, tier: ModelTier) -> bool {
        self.model_tier == ModelTier::Admin || tier <= self.model_tier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    OpenRouter,
    XaiNative,
}

impl ModelProvider {
    pub fn as_route(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::XaiNative => "xai_native",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataPolicyLabel {
    Standard,
    NoTraining,
    ZeroDataRetention,
    ByokProviderPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub context_window_tokens: u32,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    pub slug: String,
    pub display_name: String,
    pub provider: ModelProvider,
    pub tier: ModelTier,
    pub input_cost_cents_per_million_tokens: u64,
    pub output_cost_cents_per_million_tokens: u64,
    pub enabled: bool,
    pub capabilities: ModelCapabilities,
    pub data_policy: DataPolicyLabel,
}

impl ModelCatalogEntry {
    pub fn openrouter(
        slug: impl Into<String>,
        display_name: impl Into<String>,
        tier: ModelTier,
        context_window_tokens: u32,
    ) -> Self {
        Self {
            slug: slug.into(),
            display_name: display_name.into(),
            provider: ModelProvider::OpenRouter,
            tier,
            input_cost_cents_per_million_tokens: 0,
            output_cost_cents_per_million_tokens: 0,
            enabled: true,
            capabilities: ModelCapabilities {
                context_window_tokens,
                supports_streaming: true,
                supports_tools: false,
                supports_json: false,
            },
            data_policy: DataPolicyLabel::Standard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalog {
    pub entries: Vec<ModelCatalogEntry>,
}

impl ModelCatalog {
    pub fn launch_default() -> Self {
        Self {
            entries: vec![
                ModelCatalogEntry {
                    capabilities: ModelCapabilities {
                        context_window_tokens: 2_000_000,
                        supports_streaming: true,
                        supports_tools: true,
                        supports_json: true,
                    },
                    data_policy: DataPolicyLabel::Standard,
                    ..ModelCatalogEntry::openrouter(
                        FREE_DEFAULT_MODEL_SLUG,
                        "Grok Fast · Free",
                        ModelTier::Free,
                        2_000_000,
                    )
                },
                ModelCatalogEntry {
                    capabilities: ModelCapabilities {
                        context_window_tokens: 200_000,
                        supports_streaming: true,
                        supports_tools: true,
                        supports_json: true,
                    },
                    data_policy: DataPolicyLabel::NoTraining,
                    input_cost_cents_per_million_tokens: 80,
                    output_cost_cents_per_million_tokens: 400,
                    ..ModelCatalogEntry::openrouter(
                        "anthropic/claude-haiku-4.5",
                        "Claude Haiku · Balanced",
                        ModelTier::Balanced,
                        200_000,
                    )
                },
                ModelCatalogEntry {
                    capabilities: ModelCapabilities {
                        context_window_tokens: 1_000_000,
                        supports_streaming: true,
                        supports_tools: true,
                        supports_json: true,
                    },
                    data_policy: DataPolicyLabel::NoTraining,
                    input_cost_cents_per_million_tokens: 300,
                    output_cost_cents_per_million_tokens: 1_500,
                    ..ModelCatalogEntry::openrouter(
                        "google/gemini-2.5-pro-preview",
                        "Gemini Pro · Premium",
                        ModelTier::Premium,
                        1_000_000,
                    )
                },
            ],
        }
    }

    pub fn get(&self, slug: &str) -> Option<&ModelCatalogEntry> {
        self.entries.iter().find(|entry| entry.slug == slug)
    }

    pub fn allowed_for<'a>(
        &'a self,
        policy: &WorkspaceModelPolicyRecord,
        entitlements: &EntitlementsRecord,
    ) -> Vec<&'a ModelCatalogEntry> {
        self.entries
            .iter()
            .filter(|entry| policy.ensure_model_allowed(entry, entitlements).is_ok())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceModelPolicyRecord {
    pub workspace_id: String,
    pub default_model_slug: String,
    pub allowed_tiers: Vec<ModelTier>,
    pub byok_enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_model_slugs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_provider_routes: Vec<ModelProvider>,
}

impl WorkspaceModelPolicyRecord {
    pub fn launch_default(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            default_model_slug: FREE_DEFAULT_MODEL_SLUG.to_string(),
            allowed_tiers: vec![ModelTier::Free],
            byok_enabled: false,
            disabled_model_slugs: Vec::new(),
            disabled_provider_routes: Vec::new(),
        }
    }

    pub fn ensure_model_allowed(
        &self,
        entry: &ModelCatalogEntry,
        entitlements: &EntitlementsRecord,
    ) -> Result<(), ModelPolicyError> {
        if !entry.enabled {
            return Err(ModelPolicyError::DisabledModel(entry.slug.clone()));
        }
        if self
            .disabled_model_slugs
            .iter()
            .any(|slug| slug == &entry.slug)
        {
            return Err(ModelPolicyError::DisabledModel(entry.slug.clone()));
        }
        if self.disabled_provider_routes.contains(&entry.provider) {
            return Err(ModelPolicyError::DisabledProvider(
                entry.provider.as_route().to_string(),
            ));
        }
        let tier_allowed_by_policy = self.allowed_tiers.contains(&entry.tier)
            || entry.tier == ModelTier::Byok && self.byok_enabled;
        if !tier_allowed_by_policy && entitlements.model_tier != ModelTier::Admin {
            return Err(ModelPolicyError::TierNotInWorkspacePolicy(entry.tier));
        }
        if entry.tier == ModelTier::Byok {
            if self.byok_enabled || entitlements.model_tier == ModelTier::Admin {
                return Ok(());
            }
            return Err(ModelPolicyError::ByokDisabled);
        }
        if !entitlements.can_access_tier(entry.tier) {
            return Err(ModelPolicyError::EntitlementTooLow {
                requested: entry.tier,
                allowed: entitlements.model_tier,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelPolicyError {
    #[error("model {0} is disabled")]
    DisabledModel(String),
    #[error("provider route {0} is disabled")]
    DisabledProvider(String),
    #[error("model tier {0:?} is not in the workspace policy")]
    TierNotInWorkspacePolicy(ModelTier),
    #[error("BYOK models are not enabled for this workspace")]
    ByokDisabled,
    #[error("model tier {requested:?} exceeds entitlement tier {allowed:?}")]
    EntitlementTooLow {
        requested: ModelTier,
        allowed: ModelTier,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutMode {
    Setup,
    Payment,
    Subscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentEventType {
    TrialSetupSucceeded,
    OneTimePaymentSucceeded,
    RefundAdjusted,
    CheckoutExpired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentEventRecord {
    pub id: String,
    pub workspace_id: String,
    pub stripe_event_id: String,
    pub event_type: PaymentEventType,
    pub checkout_mode: CheckoutMode,
    pub amount_cents: i64,
    pub credit_stt_seconds: i64,
    pub processed_at: DateTime<Utc>,
}

impl PaymentEventRecord {
    pub fn validate_launch_contract(&self) -> Result<(), BillingContractError> {
        if self.checkout_mode == CheckoutMode::Subscription {
            return Err(BillingContractError::SubscriptionModeForbidden);
        }

        match self.event_type {
            PaymentEventType::TrialSetupSucceeded => {
                if self.checkout_mode != CheckoutMode::Setup {
                    return Err(BillingContractError::TrialMustUseSetupMode);
                }
                if self.amount_cents != 0 {
                    return Err(BillingContractError::TrialMustNotCharge);
                }
                Ok(())
            }
            PaymentEventType::OneTimePaymentSucceeded => {
                if self.checkout_mode != CheckoutMode::Payment {
                    return Err(BillingContractError::PaidPackMustUsePaymentMode);
                }
                if self.amount_cents <= 0 || self.credit_stt_seconds <= 0 {
                    return Err(BillingContractError::PaidPackMustGrantPositiveCredits);
                }
                Ok(())
            }
            PaymentEventType::RefundAdjusted => {
                if self.checkout_mode != CheckoutMode::Payment {
                    return Err(BillingContractError::RefundMustReferencePaymentMode);
                }
                Ok(())
            }
            PaymentEventType::CheckoutExpired => Ok(()),
        }
    }

    pub fn grants_trial(&self) -> bool {
        self.event_type == PaymentEventType::TrialSetupSucceeded
            && self.checkout_mode == CheckoutMode::Setup
            && self.amount_cents == 0
            && self.credit_stt_seconds > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BillingContractError {
    #[error("Stripe subscription mode is forbidden for the web launch")]
    SubscriptionModeForbidden,
    #[error("trial card gate must use Stripe Checkout setup mode")]
    TrialMustUseSetupMode,
    #[error("trial card gate must not charge automatically")]
    TrialMustNotCharge,
    #[error("paid usage packs must use one-time payment mode")]
    PaidPackMustUsePaymentMode,
    #[error("paid usage packs must grant positive prepaid credits")]
    PaidPackMustGrantPositiveCredits,
    #[error("refund adjustments must reference a one-time payment")]
    RefundMustReferencePaymentMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Active,
    Closing,
    Closed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    pub stt_seconds: u64,
    pub tts_chars: u64,
    pub llm_tokens: u64,
    pub selected_model_slug: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnFinality {
    Partial,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub id: String,
    pub session_id: String,
    pub speaker: WebSpeaker,
    pub text: String,
    pub finality: TurnFinality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub workspace_id: String,
    pub version: String,
    pub accepted_at: DateTime<Utc>,
    pub ip_hash: String,
    pub user_agent_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    SttSeconds,
    TtsCharacters,
    LlmInputTokens,
    LlmOutputTokens,
    LlmCostCents,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEventRecord {
    pub id: String,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub kind: UsageKind,
    pub units: u64,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl UsageEventRecord {
    fn idempotency_key(&self) -> String {
        match &self.provider_request_id {
            Some(provider_request_id) => format!(
                "{}:{}:{:?}:{}:{}",
                self.workspace_id,
                self.session_id.as_deref().unwrap_or(""),
                self.kind,
                self.provider,
                provider_request_id
            ),
            None => format!("event:{}", self.id),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub stt_seconds: u64,
    pub tts_chars: u64,
    pub llm_input_tokens: u64,
    pub llm_output_tokens: u64,
    pub llm_cost_cents: u64,
}

impl UsageTotals {
    pub fn llm_tokens(self) -> u64 {
        self.llm_input_tokens + self.llm_output_tokens
    }

    pub fn add_event(&mut self, event: &UsageEventRecord) {
        match event.kind {
            UsageKind::SttSeconds => self.stt_seconds += event.units,
            UsageKind::TtsCharacters => self.tts_chars += event.units,
            UsageKind::LlmInputTokens => self.llm_input_tokens += event.units,
            UsageKind::LlmOutputTokens => self.llm_output_tokens += event.units,
            UsageKind::LlmCostCents => self.llm_cost_cents += event.units,
        }
    }

    pub fn to_turn_usage(self) -> LlmTurnUsage {
        LlmTurnUsage {
            input_tokens: self.llm_input_tokens,
            output_tokens: self.llm_output_tokens,
            cost_cents: self.llm_cost_cents,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UsageLedger {
    events: Vec<UsageEventRecord>,
    seen_keys: HashSet<String>,
}

impl UsageLedger {
    pub fn record(&mut self, event: UsageEventRecord) -> bool {
        let key = event.idempotency_key();
        if !self.seen_keys.insert(key) {
            return false;
        }
        self.events.push(event);
        true
    }

    pub fn events(&self) -> &[UsageEventRecord] {
        &self.events
    }

    pub fn totals_for_workspace(&self, workspace_id: &str) -> UsageTotals {
        let mut totals = UsageTotals::default();
        for event in self
            .events
            .iter()
            .filter(|event| event.workspace_id == workspace_id)
        {
            totals.add_event(event);
        }
        totals
    }

    pub fn totals_for_session(&self, session_id: &str) -> UsageTotals {
        let mut totals = UsageTotals::default();
        for event in self
            .events
            .iter()
            .filter(|event| event.session_id.as_deref() == Some(session_id))
        {
            totals.add_event(event);
        }
        totals
    }
}
