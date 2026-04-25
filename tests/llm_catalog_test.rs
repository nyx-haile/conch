use conch::llm::catalog::{
    find_openrouter_model, openrouter_catalog, resolve_openrouter_model,
    selectable_openrouter_models, ModelEntitlement, ModelTier, OPENROUTER_FREE_TIER_DEFAULT_SLUG,
    OPENROUTER_PAID_DEFAULT_SLUG,
};

#[test]
fn openrouter_free_tier_defaults_to_grok() {
    let entry = resolve_openrouter_model(None, ModelEntitlement::Free).unwrap();

    assert_eq!(entry.slug, OPENROUTER_FREE_TIER_DEFAULT_SLUG);
    assert_eq!(entry.provider, "x-ai");
    assert_eq!(entry.tier, ModelTier::Free);
    assert!(entry.enabled);
    assert!(entry.supports_streaming);
    assert!(entry.context_window_tokens >= 1_000_000);
}

#[test]
fn free_entitlement_cannot_select_paid_model() {
    let err = resolve_openrouter_model(Some(OPENROUTER_PAID_DEFAULT_SLUG), ModelEntitlement::Free)
        .unwrap_err();

    assert!(err.to_string().contains("requires Paid tier"));
}

#[test]
fn paid_entitlement_can_select_paid_catalog_models() {
    let entry =
        resolve_openrouter_model(Some(OPENROUTER_PAID_DEFAULT_SLUG), ModelEntitlement::Paid)
            .unwrap();

    assert_eq!(entry.slug, OPENROUTER_PAID_DEFAULT_SLUG);
    assert_eq!(entry.tier, ModelTier::Paid);
    assert!(entry.supports_tool_calls);
}

#[test]
fn unknown_model_slug_is_rejected_by_catalog_boundary() {
    let err =
        resolve_openrouter_model(Some("not/a-real-model"), ModelEntitlement::Admin).unwrap_err();

    assert!(err.to_string().contains("not in the Conch model catalog"));
}

#[test]
fn selectable_models_respect_entitlement_tiers() {
    let free: Vec<_> = selectable_openrouter_models(ModelEntitlement::Free)
        .map(|entry| entry.slug)
        .collect();
    let paid: Vec<_> = selectable_openrouter_models(ModelEntitlement::Paid)
        .map(|entry| entry.slug)
        .collect();
    let byok: Vec<_> = selectable_openrouter_models(ModelEntitlement::Byok)
        .map(|entry| entry.slug)
        .collect();

    assert_eq!(free, vec![OPENROUTER_FREE_TIER_DEFAULT_SLUG]);
    assert!(paid.contains(&OPENROUTER_FREE_TIER_DEFAULT_SLUG));
    assert!(paid.contains(&OPENROUTER_PAID_DEFAULT_SLUG));
    assert!(byok.len() > paid.len());
}

#[test]
fn catalog_entries_include_required_launch_metadata() {
    assert!(!openrouter_catalog().is_empty());

    for entry in openrouter_catalog() {
        assert!(!entry.slug.is_empty());
        assert!(!entry.display_name.is_empty());
        assert!(!entry.provider.is_empty());
        assert!(entry.context_window_tokens > 0);
        assert!(entry.input_cost_per_million_tokens_usd >= 0.0);
        assert!(entry.output_cost_per_million_tokens_usd >= 0.0);
        assert!(!entry.data_policy_label.is_empty());
    }

    assert!(find_openrouter_model(OPENROUTER_FREE_TIER_DEFAULT_SLUG).is_some());
}
