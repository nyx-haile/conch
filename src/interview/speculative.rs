use std::collections::HashMap;

pub const STABILITY_WINDOW_MS: u64 = 1_500;
pub const COMMIT_THRESHOLD: f32 = 0.9;

pub fn partial_is_stable_long_enough(stable_ms: u64, is_stable: bool) -> bool {
    is_stable && stable_ms >= STABILITY_WINDOW_MS
}

pub fn similarity(partial: &str, final_text: &str) -> f32 {
    let p = normalize(partial);
    let f = normalize(final_text);
    if p.is_empty() || f.is_empty() {
        return 0.0;
    }
    // Only use prefix fast-path when partial covers most of the final,
    // otherwise a one-word partial would commit against any longer final.
    let p_words = p.split_whitespace().count();
    let f_words = f.split_whitespace().count();
    if f_words > 0 && p_words as f32 / f_words as f32 >= 0.7 {
        if f.starts_with(&p) || p.starts_with(&f) {
            return 1.0;
        }
    }
    cosine(&p, &f)
}

pub fn should_commit_draft(partial: &str, final_text: &str) -> bool {
    similarity(partial, final_text) >= COMMIT_THRESHOLD
}

fn normalize(s: &str) -> String {
    s.trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn cosine(a: &str, b: &str) -> f32 {
    let av = term_freqs(a);
    let bv = term_freqs(b);
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (term, &ca) in av.iter() {
        let ca = ca as f32;
        na += ca * ca;
        if let Some(&cb) = bv.get(term) {
            dot += ca * cb as f32;
        }
    }
    for (_, &cb) in bv.iter() {
        let cb = cb as f32;
        nb += cb * cb;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

fn term_freqs(s: &str) -> HashMap<String, u32> {
    let mut m = HashMap::new();
    for w in s.split_whitespace() {
        *m.entry(w.to_string()).or_insert(0) += 1;
    }
    m
}
