use std::sync::OnceLock;

pub(in crate::app) fn reduce_motion_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(detect_reduce_motion)
}

fn detect_reduce_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Some(enabled) = macos_reduce_motion_enabled() {
            return enabled;
        }
        defaults_reduce_motion_enabled()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
fn macos_reduce_motion_enabled() -> Option<bool> {
    use cocoa::base::{id, YES};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let enabled: cocoa::base::BOOL = msg_send![workspace, accessibilityDisplayShouldReduceMotion];
        Some(enabled == YES)
    }
}

#[cfg(target_os = "macos")]
fn defaults_reduce_motion_enabled() -> bool {
    std::process::Command::new("defaults")
        .args(["read", "com.apple.universalaccess", "reduceMotion"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

pub(in crate::app) fn humanize_tool_name(tool: &str) -> String {
    match tool.trim().to_ascii_lowercase().as_str() {
        "claude" => "Claude Code".to_string(),
        "codex" => "Codex".to_string(),
        "kiro" => "Kiro".to_string(),
        "opencode" => "OpenCode".to_string(),
        "agy" => "Antigravity".to_string(),
        "kimi" => "Kimi".to_string(),
        "mimo" => "Mimo".to_string(),
        "codewhale" => "CodeWhale".to_string(),
        _ => title_case_identifier(tool),
    }
}

pub(in crate::app) fn shorten_model_name(model: &str) -> String {
    let model = model.trim();
    if model.is_empty() {
        return String::new();
    }

    if let Some(shortened) = shorten_claude_model(model) {
        return shortened;
    }

    if let Some(shortened) = shorten_gpt_model(model) {
        return shortened;
    }

    truncate_model_name(model, 20)
}

fn title_case_identifier(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_word(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut result = first.to_ascii_uppercase().to_string();
    result.extend(chars.map(|c| c.to_ascii_lowercase()));
    result
}

fn shorten_claude_model(model: &str) -> Option<String> {
    let lower = model.to_ascii_lowercase();
    let rest = lower.strip_prefix("claude-")?;
    let mut parts: Vec<&str> = rest.split('-').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    if let Some(last) = parts.last()
        && last.len() == 8
        && last.chars().all(|c| c.is_ascii_digit())
    {
        parts.pop();
    }

    if parts.len() < 2 {
        return None;
    }

    let family = title_case_word(parts[0]);
    let version = parts[1..].join(".");
    Some(format!("{family} {version}"))
}

fn shorten_gpt_model(model: &str) -> Option<String> {
    let lower = model.to_ascii_lowercase();
    let rest = lower.strip_prefix("gpt-")?;
    Some(format!("GPT-{rest}"))
}

fn truncate_model_name(model: &str, max_len: usize) -> String {
    model.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::{humanize_tool_name, shorten_model_name};

    #[test]
    fn humanize_tool_name_maps_known_agents() {
        assert_eq!(humanize_tool_name("claude"), "Claude Code");
        assert_eq!(humanize_tool_name("codex"), "Codex");
        assert_eq!(humanize_tool_name("kiro"), "Kiro");
        assert_eq!(humanize_tool_name("opencode"), "OpenCode");
        assert_eq!(humanize_tool_name("codewhale"), "CodeWhale");
        assert_eq!(humanize_tool_name("agy"), "Antigravity");
        assert_eq!(humanize_tool_name("kimi"), "Kimi");
        assert_eq!(humanize_tool_name("mimo"), "Mimo");
    }

    #[test]
    fn humanize_tool_name_title_cases_unknown_agents() {
        assert_eq!(humanize_tool_name("new_agent"), "New Agent");
        assert_eq!(humanize_tool_name("new-agent"), "New Agent");
    }

    #[test]
    fn shorten_model_name_shortens_claude_family() {
        assert_eq!(
            shorten_model_name("claude-sonnet-4-5-20250514"),
            "Sonnet 4.5"
        );
        assert_eq!(shorten_model_name("claude-opus-4-1-20250805"), "Opus 4.1");
        assert_eq!(shorten_model_name("claude-haiku-3-5-20241022"), "Haiku 3.5");
    }

    #[test]
    fn shorten_model_name_maps_gpt_family() {
        assert_eq!(shorten_model_name("gpt-4o"), "GPT-4o");
        assert_eq!(shorten_model_name("gpt-4o-mini"), "GPT-4o-mini");
    }

    #[test]
    fn shorten_model_name_truncates_unknown_models() {
        let shortened = shorten_model_name("some-very-long-model-identifier-v2");
        assert!(shortened.len() <= 20);
        assert_eq!(shortened, "some-very-long-model");
    }
}
