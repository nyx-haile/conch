use anyhow::{anyhow, Result};

/// Launch policy default for card-gated/free users.
///
/// The free Conch tier is free to the user, but still routes through
/// OpenRouter with Conch-owned credits. Keep this slug in the catalog so
/// entitlement checks, cost estimates, and kill switches all use the same
/// boundary as paid model selection.
pub const OPENROUTER_FREE_TIER_DEFAULT_SLUG: &str = "x-ai/grok-4-fast";
pub const OPENROUTER_PAID_DEFAULT_SLUG: &str = "x-ai/grok-4.1-fast";
pub const OPENROUTER_PREMIUM_DEFAULT_SLUG: &str = "anthropic/claude-sonnet-4.6";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Free,
    Paid,
    Premium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEntitlement {
    Free,
    Paid,
    Byok,
    Admin,
}

impl ModelEntitlement {
    pub fn allows(self, tier: ModelTier) -> bool {
        match self {
            Self::Free => tier == ModelTier::Free,
            Self::Paid => matches!(tier, ModelTier::Free | ModelTier::Paid),
            Self::Byok | Self::Admin => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelCatalogEntry {
    pub slug: &'static str,
    pub display_name: &'static str,
    pub provider: &'static str,
    pub tier: ModelTier,
    pub input_cost_per_million_tokens_usd: f32,
    pub output_cost_per_million_tokens_usd: f32,
    pub context_window_tokens: u32,
    pub supports_streaming: bool,
    pub supports_tool_calls: bool,
    pub supports_structured_outputs: bool,
    pub data_policy_label: &'static str,
    pub enabled: bool,
}

/// Server-side OpenRouter allow-list for launch.
///
/// The catalog intentionally starts small. UI/server callers should use this
/// list instead of accepting arbitrary model slugs from clients, then expand it
/// behind the same entitlement and kill-switch checks as models are approved.
pub const OPENROUTER_MODEL_CATALOG: &[ModelCatalogEntry] = &[
    ModelCatalogEntry {
        slug: OPENROUTER_FREE_TIER_DEFAULT_SLUG,
        display_name: "Grok 4 Fast",
        provider: "x-ai",
        tier: ModelTier::Free,
        input_cost_per_million_tokens_usd: 0.20,
        output_cost_per_million_tokens_usd: 0.50,
        context_window_tokens: 2_000_000,
        supports_streaming: true,
        supports_tool_calls: false,
        supports_structured_outputs: false,
        data_policy_label: "OpenRouter routed; launch free-tier default",
        enabled: true,
    },
    ModelCatalogEntry {
        slug: OPENROUTER_PAID_DEFAULT_SLUG,
        display_name: "Grok 4.1 Fast",
        provider: "x-ai",
        tier: ModelTier::Paid,
        input_cost_per_million_tokens_usd: 0.20,
        output_cost_per_million_tokens_usd: 0.50,
        context_window_tokens: 2_000_000,
        supports_streaming: true,
        supports_tool_calls: true,
        supports_structured_outputs: false,
        data_policy_label: "OpenRouter routed; paid/BYOK selectable",
        enabled: true,
    },
    ModelCatalogEntry {
        slug: OPENROUTER_PREMIUM_DEFAULT_SLUG,
        display_name: "Claude Sonnet 4.6",
        provider: "anthropic",
        tier: ModelTier::Premium,
        input_cost_per_million_tokens_usd: 3.00,
        output_cost_per_million_tokens_usd: 15.00,
        context_window_tokens: 200_000,
        supports_streaming: true,
        supports_tool_calls: true,
        supports_structured_outputs: true,
        data_policy_label: "OpenRouter routed; BYOK/admin only until paid policy approval",
        enabled: true,
    },
    ModelCatalogEntry {
        slug: "google/gemini-2.5-pro-preview",
        display_name: "Gemini 2.5 Pro Preview",
        provider: "google",
        tier: ModelTier::Premium,
        input_cost_per_million_tokens_usd: 1.25,
        output_cost_per_million_tokens_usd: 10.00,
        context_window_tokens: 1_000_000,
        supports_streaming: true,
        supports_tool_calls: true,
        supports_structured_outputs: true,
        data_policy_label: "OpenRouter routed; BYOK/admin only until paid policy approval",
        enabled: true,
    },
];

pub fn openrouter_catalog() -> &'static [ModelCatalogEntry] {
    OPENROUTER_MODEL_CATALOG
}

pub fn find_openrouter_model(slug: &str) -> Option<&'static ModelCatalogEntry> {
    OPENROUTER_MODEL_CATALOG
        .iter()
        .find(|entry| entry.slug == slug)
}

pub fn selectable_openrouter_models(
    entitlement: ModelEntitlement,
) -> impl Iterator<Item = &'static ModelCatalogEntry> {
    OPENROUTER_MODEL_CATALOG
        .iter()
        .filter(move |entry| entry.enabled && entitlement.allows(entry.tier))
}

pub fn resolve_openrouter_model(
    requested_slug: Option<&str>,
    entitlement: ModelEntitlement,
) -> Result<&'static ModelCatalogEntry> {
    let slug = requested_slug.unwrap_or(OPENROUTER_FREE_TIER_DEFAULT_SLUG);
    let entry = find_openrouter_model(slug).ok_or_else(|| {
        anyhow!(
            "OpenRouter model {:?} is not in the Conch model catalog",
            slug
        )
    })?;

    if !entry.enabled {
        return Err(anyhow!(
            "OpenRouter model {:?} is currently disabled by catalog policy",
            slug
        ));
    }

    if !entitlement.allows(entry.tier) {
        return Err(anyhow!(
            "OpenRouter model {:?} requires {:?} tier, but workspace entitlement is {:?}",
            slug,
            entry.tier,
            entitlement
        ));
    }

    Ok(entry)
}

pub fn openrouter_slug_for_depth(model: crate::model::Model) -> &'static str {
    match model {
        crate::model::Model::Haiku => OPENROUTER_FREE_TIER_DEFAULT_SLUG,
        crate::model::Model::Sonnet => OPENROUTER_PAID_DEFAULT_SLUG,
        crate::model::Model::Opus => OPENROUTER_PREMIUM_DEFAULT_SLUG,
    }
}
