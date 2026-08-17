use conch::config::Config;
use conch::stt::corrections::CorrectionStore;

#[test]
fn round_trips_through_json_on_disk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("nested").join("corrections.json");

    let mut store = CorrectionStore::new(&path);
    store.set_preferred_transcription("pepper ex", "Pepper X");
    store.add_replacement_rule("pepper x", "Pepper X");
    store.save().expect("save");

    assert!(
        path.is_file(),
        "save should create parent dirs and the file"
    );
    let raw = std::fs::read_to_string(&path).expect("read back");
    assert!(raw.contains("\"preferred\""), "hand-editable JSON: {raw}");

    let reloaded = CorrectionStore::load(&path).expect("load");
    assert_eq!(reloaded.preferred().len(), 1);
    assert_eq!(reloaded.preferred()[0].raw, "pepper ex");
    assert_eq!(reloaded.preferred()[0].preferred, "Pepper X");
    assert_eq!(reloaded.replacements().len(), 1);
    assert_eq!(reloaded.replacements()[0].from, "pepper x");
    assert_eq!(reloaded.replacements()[0].to, "Pepper X");
}

#[test]
fn missing_file_loads_as_empty_store() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("corrections.json");
    let store = CorrectionStore::load(&path).expect("missing file is not an error");
    assert!(store.is_empty());
    assert_eq!(store.prompt_memory_text(), None);
    assert_eq!(store.apply_replacements("untouched"), "untouched");
}

#[test]
fn malformed_file_errors_on_load_but_not_on_load_or_default() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("corrections.json");
    std::fs::write(&path, "{ not json at all").expect("write");

    assert!(CorrectionStore::load(&path).is_err());
    assert!(CorrectionStore::load_or_default(&path).is_empty());
}

#[test]
fn setters_update_in_place_instead_of_duplicating() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));

    store.set_preferred_transcription("nix", "Nyx");
    store.set_preferred_transcription("NIX", "Nyx the agent");
    assert_eq!(store.preferred().len(), 1);
    assert_eq!(store.preferred()[0].preferred, "Nyx the agent");

    store.add_replacement_rule("conk", "conch");
    store.add_replacement_rule("Conk", "conch CLI");
    assert_eq!(store.replacements().len(), 1);
    assert_eq!(store.replacements()[0].to, "conch CLI");
}

#[test]
fn empty_replacement_key_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));
    store.add_replacement_rule("", "boom");
    assert!(store.replacements().is_empty());
    assert_eq!(store.apply_replacements("hello"), "hello");
}

#[test]
fn longer_rules_win_over_prefix_collisions() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));
    // Insertion order deliberately puts the SHORT rule first; ordering must be
    // decided by key length at apply time, not by insertion order.
    store.add_replacement_rule("pepper", "Pepper");
    store.add_replacement_rule("pepper x", "Pepper X");

    let ordered: Vec<&str> = store
        .rules_longest_first()
        .iter()
        .map(|r| r.from.as_str())
        .collect();
    assert_eq!(ordered, vec!["pepper x", "pepper"]);

    assert_eq!(
        store.apply_replacements("i tried pepper x and plain pepper"),
        "i tried Pepper X and plain Pepper"
    );
}

#[test]
fn replacements_are_ascii_case_insensitive_and_do_not_cascade() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));
    store.add_replacement_rule("pepper x", "Pepper X");
    // A rule whose OUTPUT contains another rule's key must not be re-scanned.
    store.add_replacement_rule("ex", "X");

    assert_eq!(store.apply_replacements("PEPPER X rules"), "Pepper X rules");
    assert_eq!(store.apply_replacements("Pepper x rules"), "Pepper X rules");
    assert_eq!(store.apply_replacements("ex ex"), "X X");
}

#[test]
fn replacements_preserve_non_ascii_text() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));
    store.add_replacement_rule("cafe", "café");
    assert_eq!(
        store.apply_replacements("naïve cafe — 日本語"),
        "naïve café — 日本語"
    );
}

#[test]
fn prompt_memory_text_formats_both_sections_in_apply_order() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut store = CorrectionStore::new(tmp.path().join("corrections.json"));
    store.set_preferred_transcription("pepper ex", "Pepper X");
    store.set_preferred_transcription("nix", "Nyx");
    store.add_replacement_rule("pepper", "Pepper");
    store.add_replacement_rule("pepper x", "Pepper X");

    let text = store.prompt_memory_text().expect("non-empty store");
    assert!(text.starts_with("Transcription memory:"));
    assert!(text.contains("heard \"pepper ex\" -> meant \"Pepper X\""));
    assert!(text.contains("heard \"nix\" -> meant \"Nyx\""));
    // Preferred examples keep insertion order.
    assert!(text.find("pepper ex").unwrap() < text.find("nix").unwrap());
    // Vocabulary follows the same longest-first order used when applying.
    let long = text.rfind("Pepper X").unwrap();
    let short = text.rfind("- Pepper\n").or_else(|| text.rfind("- Pepper"));
    assert!(short.is_some(), "short rule listed: {text}");
    assert!(long < short.unwrap(), "longest-first vocabulary: {text}");
    assert!(!text.ends_with('\n'), "trailing newline trimmed: {text:?}");
}

#[test]
fn prompt_memory_text_handles_one_sided_stores() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let mut only_preferred = CorrectionStore::new(tmp.path().join("a.json"));
    only_preferred.set_preferred_transcription("nix", "Nyx");
    let text = only_preferred.prompt_memory_text().expect("some");
    assert!(text.contains("meant \"Nyx\""));
    assert!(!text.contains("already normalized"));

    let mut only_rules = CorrectionStore::new(tmp.path().join("b.json"));
    only_rules.add_replacement_rule("conk", "conch");
    let text = only_rules.prompt_memory_text().expect("some");
    assert!(text.contains("already normalized"));
    assert!(!text.contains("meant"));
}

#[test]
fn config_exposes_corrections_file_under_conch_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = Config::with_home(tmp.path());
    assert_eq!(
        config.corrections_file(),
        tmp.path().join(".conch").join("corrections.json")
    );
    assert_eq!(
        config.corrections_file().parent(),
        Some(config.conch_dir().as_path())
    );
}
