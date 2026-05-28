use super::payload::AIHookEventMetadata;

pub fn canonical_tool_name(tool: &str) -> Option<String> {
    let normalized = normalized_string(Some(tool))?.to_lowercase();
    match normalized.as_str() {
        "claude-code" => Some("claude".to_string()),
        "agy" => Some("gemini".to_string()),
        _ => Some(normalized),
    }
}

pub fn runtime_state_for_hook_kind(
    kind: &str,
    metadata: Option<&AIHookEventMetadata>,
) -> &'static str {
    match kind {
        "promptSubmitted" | "memoryRefreshing" => "responding",
        "sessionStarted" => "idle",
        "needsInput" => "needsInput",
        "turnCompleted" | "sessionEnded" => "idle",
        _ if metadata
            .and_then(|metadata| metadata.notification_type.as_deref())
            .and_then(|value| normalized_string(Some(value)))
            .is_some() =>
        {
            "needsInput"
        }
        _ => "idle",
    }
}

pub fn status_for_runtime_state(state: &str) -> &'static str {
    match state {
        "responding" => "running",
        "needsInput" => "needs-input",
        _ => "idle",
    }
}

pub fn normalized_string(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_tool_names_like_tauri_runtime() {
        assert_eq!(
            canonical_tool_name("claude-code").as_deref(),
            Some("claude")
        );
        assert_eq!(canonical_tool_name("agy").as_deref(), Some("gemini"));
        assert_eq!(canonical_tool_name("codex").as_deref(), Some("codex"));
    }

    #[test]
    fn maps_hook_kind_to_runtime_status() {
        assert_eq!(
            status_for_runtime_state(runtime_state_for_hook_kind("promptSubmitted", None)),
            "running"
        );
        assert_eq!(
            status_for_runtime_state(runtime_state_for_hook_kind("needsInput", None)),
            "needs-input"
        );
        assert_eq!(
            status_for_runtime_state(runtime_state_for_hook_kind("turnCompleted", None)),
            "idle"
        );
    }
}
