use std::collections::HashMap;

use regex::Regex;

/// Normalize a client model name to Kiro format (dash→dot minor, strip date/latest/[1m]).
/// Faithful port of the Python `normalize_model_name`.
pub fn normalize_model_name(name: &str) -> String {
    if name.is_empty() {
        return name.to_string();
    }
    // Strip context-window suffix like [1m], [200k].
    let ctx_re = Regex::new(r"(?i)\[\d+[mk]\]$").unwrap();
    let name = ctx_re.replace(name, "").to_string();
    let lower = name.to_lowercase();

    // Pattern 1: claude-{family}-{major}-{minor}(-suffix)?
    let p1 =
        Regex::new(r"^(claude-(?:haiku|sonnet|opus)-\d+)-(\d{1,2})(?:-(?:\d{8}|latest|\d+))?$")
            .unwrap();
    if let Some(c) = p1.captures(&lower) {
        return format!("{}.{}", &c[1], &c[2]);
    }

    // Pattern 2: claude-{family}-{major}(-date)?
    let p2 = Regex::new(r"^(claude-(?:haiku|sonnet|opus)-\d+)(?:-\d{8})?$").unwrap();
    if let Some(c) = p2.captures(&lower) {
        return c[1].to_string();
    }

    // Pattern 3: legacy claude-{major}-{minor}-{family}(-suffix)?
    let p3 = Regex::new(r"^(claude)-(\d+)-(\d+)-(haiku|sonnet|opus)(?:-(?:\d{8}|latest|\d+))?$")
        .unwrap();
    if let Some(c) = p3.captures(&lower) {
        return format!("{}-{}.{}-{}", &c[1], &c[2], &c[3], &c[4]);
    }

    // Pattern 4: already dotted, with date suffix.
    let p4 =
        Regex::new(r"^(claude-(?:\d+\.\d+-)?(?:haiku|sonnet|opus)(?:-\d+\.\d+)?)-\d{8}$").unwrap();
    if let Some(c) = p4.captures(&lower) {
        return c[1].to_string();
    }

    // Pattern 5: inverted with required suffix claude-{major}.{minor}-{family}-{suffix}
    let p5 = Regex::new(r"^claude-(\d+)\.(\d+)-(haiku|sonnet|opus)-(.+)$").unwrap();
    if let Some(c) = p5.captures(&lower) {
        return format!("claude-{}-{}.{}", &c[3], &c[1], &c[2]);
    }

    name
}

/// Resolve a request model to the id to send to Kiro (alias → normalize → hidden → passthrough).
pub fn resolve_model_id(
    external: &str,
    aliases: &HashMap<String, String>,
    hidden_models: &HashMap<String, String>,
) -> String {
    let resolved = aliases
        .get(external)
        .cloned()
        .unwrap_or_else(|| external.to_string());
    let normalized = normalize_model_name(&resolved);
    hidden_models
        .get(&normalized)
        .cloned()
        .unwrap_or(normalized)
}

/// Available models for /v1/models (fallback + hidden + aliases, minus hidden_from_list).
pub fn available_models(
    aliases: &HashMap<String, String>,
    hidden_models: &HashMap<String, String>,
    hidden_from_list: &[String],
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = crate::config::FALLBACK_MODELS
        .iter()
        .map(|s| s.to_string())
        .collect();
    set.extend(hidden_models.keys().cloned());
    for h in hidden_from_list {
        set.remove(h);
    }
    set.extend(aliases.keys().cloned());
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_forms() {
        assert_eq!(
            normalize_model_name("claude-haiku-4-5-20251001"),
            "claude-haiku-4.5"
        );
        assert_eq!(
            normalize_model_name("claude-sonnet-4-5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(normalize_model_name("claude-sonnet-4"), "claude-sonnet-4");
        assert_eq!(
            normalize_model_name("claude-sonnet-4-20250514"),
            "claude-sonnet-4"
        );
        assert_eq!(
            normalize_model_name("claude-3-7-sonnet"),
            "claude-3.7-sonnet"
        );
        assert_eq!(
            normalize_model_name("claude-3-7-sonnet-20250219"),
            "claude-3.7-sonnet"
        );
        assert_eq!(
            normalize_model_name("claude-4.5-opus-high"),
            "claude-opus-4.5"
        );
        assert_eq!(
            normalize_model_name("claude-4.5-sonnet-low"),
            "claude-sonnet-4.5"
        );
        assert_eq!(normalize_model_name("auto"), "auto");
        assert_eq!(
            normalize_model_name("claude-sonnet-4-5[1m]"),
            "claude-sonnet-4.5"
        );
    }
}
