use conch::config::Config;
use conch::llm;
use conch::llm::catalog::{OPENROUTER_FREE_TIER_DEFAULT_SLUG, OPENROUTER_PAID_DEFAULT_SLUG};
use conch::model::Model;
use conch::provider::Provider;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn resolve_openrouter_provider_uses_openrouter_catalog_depth_slugs() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = HashMap::new();
    env.insert("OPENROUTER_API_KEY".to_string(), "sk-or-test".to_string());
    env.insert("CONCH_PROVIDER".to_string(), "openrouter".to_string());
    let config = Config::from_env_map(&home, &env).unwrap();

    let (_, haiku_slug) = llm::resolve(&config, Provider::OpenRouter, Model::Haiku).unwrap();
    let (_, sonnet_slug) = llm::resolve(&config, Provider::OpenRouter, Model::Sonnet).unwrap();

    assert_eq!(haiku_slug, OPENROUTER_FREE_TIER_DEFAULT_SLUG);
    assert_eq!(sonnet_slug, OPENROUTER_PAID_DEFAULT_SLUG);
}

#[test]
fn resolve_openrouter_gateway_enforces_catalog_entitlement() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = HashMap::new();
    env.insert("OPENROUTER_API_KEY".to_string(), "sk-or-test".to_string());
    let config = Config::from_env_map(&home, &env).unwrap();

    let (_, free_entry) =
        llm::resolve_openrouter_gateway(&config, None, conch::llm::catalog::ModelEntitlement::Free)
            .unwrap();
    assert_eq!(free_entry.slug, OPENROUTER_FREE_TIER_DEFAULT_SLUG);

    let paid_err = llm::resolve_openrouter_gateway(
        &config,
        Some(OPENROUTER_PAID_DEFAULT_SLUG),
        conch::llm::catalog::ModelEntitlement::Free,
    )
    .unwrap_err();
    assert!(paid_err.to_string().contains("requires Paid tier"));

    let (_, paid_entry) = llm::resolve_openrouter_gateway(
        &config,
        Some(OPENROUTER_PAID_DEFAULT_SLUG),
        conch::llm::catalog::ModelEntitlement::Paid,
    )
    .unwrap();
    assert_eq!(paid_entry.slug, OPENROUTER_PAID_DEFAULT_SLUG);
}
