use conch::interview::speculative::{partial_is_stable_long_enough, similarity, should_commit_draft};

#[test]
fn stability_gate_requires_1500ms_of_stable_partials() {
    assert!(!partial_is_stable_long_enough(500, true));
    assert!(!partial_is_stable_long_enough(1499, true));
    assert!(partial_is_stable_long_enough(1500, true));
    assert!(!partial_is_stable_long_enough(5_000, false));
}

#[test]
fn similarity_is_high_for_prefix_match() {
    let partial = "so I built this using rust";
    let final_ = "so I built this using rust and tokio";
    assert!(similarity(partial, final_) >= 0.9);
}

#[test]
fn similarity_is_low_for_divergent_text() {
    let partial = "yeah so the backend is in python";
    let final_ = "actually never mind, different question";
    assert!(similarity(partial, final_) < 0.9);
}

#[test]
fn short_prefix_does_not_auto_commit() {
    // A one-word partial that happens to be a prefix of the final should NOT
    // get the 1.0 fast-path — cosine will produce a low score instead.
    assert!(!should_commit_draft("so", "so after thinking about it I took a different approach"));
}

#[test]
fn commit_threshold_matches_spec() {
    assert!(should_commit_draft("a b c d", "a b c d e"));
    assert!(!should_commit_draft("foo bar baz", "totally different response"));
}
