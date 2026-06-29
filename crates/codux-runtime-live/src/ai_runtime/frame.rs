use super::payload::{
    AIHookEventMetadata, AIHookEventPayload, AILifecycleHookEnvelope, AIToolUsageEnvelope,
    RuntimeEnvelope,
};
use serde_json::Value;

pub fn runtime_frame_to_hook(buffer: &[u8]) -> Option<AIHookEventPayload> {
    let buffer = buffer.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(buffer);
    let envelope = serde_json::from_slice::<RuntimeEnvelope>(buffer).ok()?;
    match envelope.kind.as_str() {
        "ai-hook" => serde_json::from_value::<AIHookEventPayload>(envelope.payload).ok(),
        "ai-lifecycle-hook" => serde_json::from_value::<AILifecycleHookEnvelope>(envelope.payload)
            .ok()
            .and_then(lifecycle_hook_to_hook),
        "opencode-runtime" => serde_json::from_value::<AIToolUsageEnvelope>(envelope.payload)
            .ok()
            .and_then(opencode_runtime_to_hook),
        _ => None,
    }
}

pub fn lifecycle_hook_to_hook(envelope: AILifecycleHookEnvelope) -> Option<AIHookEventPayload> {
    match (envelope.tool.as_str(), envelope.action.as_str()) {
        ("codewhale", "codewhale-turn-end") => codewhale_turn_end_to_hook(envelope),
        _ => None,
    }
}

fn codewhale_turn_end_to_hook(envelope: AILifecycleHookEnvelope) -> Option<AIHookEventPayload> {
    if envelope.terminal_id.trim().is_empty() || envelope.project_id.trim().is_empty() {
        return None;
    }

    let status = string_field(&envelope.payload, "status").unwrap_or_else(|| "completed".into());
    let was_interrupted = codewhale_status_is_interrupted(&status);
    let has_completed_turn = !was_interrupted;
    let input_tokens = number_from_containers(&envelope.payload, &["input_tokens"]);
    let output_tokens = number_from_containers(&envelope.payload, &["output_tokens"]);
    let cached_input_tokens = nested_number(
        &envelope.payload,
        "usage",
        &["cached_input_tokens", "prompt_cache_hit_tokens"],
    );
    let total_tokens = top_level_number(
        &envelope.payload,
        &["total_tokens", "totalTokenCount", "totalTokens"],
    )
    .or_else(|| {
        nested_number(
            &envelope.payload,
            "totals",
            &[
                "session_tokens",
                "conversation_tokens",
                "total_tokens",
                "totalTokens",
            ],
        )
    })
    .or_else(|| sum_nested_numbers(&envelope.payload, "totals", "input_tokens", "output_tokens"))
    .or_else(|| nested_number(&envelope.payload, "usage", &["total_tokens", "totalTokens"]))
    .or_else(|| sum_nested_numbers(&envelope.payload, "usage", "input_tokens", "output_tokens"));

    Some(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        terminal_id: envelope.terminal_id,
        terminal_instance_id: envelope.terminal_instance_id,
        project_id: envelope.project_id,
        project_name: envelope.project_name,
        project_path: envelope.project_path,
        session_title: envelope.session_title,
        tool: envelope.tool,
        ai_session_id: string_field(&envelope.payload, "session_id")
            .or_else(|| string_field(&envelope.payload, "sessionId")),
        model: string_field(&envelope.payload, "model").or(envelope.model),
        input_tokens,
        output_tokens,
        cached_input_tokens,
        total_tokens,
        updated_at: envelope.updated_at,
        metadata: Some(AIHookEventMetadata {
            transcript_path: None,
            notification_type: None,
            source: Some("codewhale-lifecycle".to_string()),
            reason: string_field(&envelope.payload, "error")
                .or_else(|| string_field(&envelope.payload, "reason"))
                .or(Some(status)),
            cwd: string_field(&envelope.payload, "workspace")
                .or_else(|| string_field(&envelope.payload, "cwd"))
                .or_else(|| string_field(&envelope.payload, "current_working_directory"))
                .or_else(|| string_field(&envelope.payload, "working_directory")),
            target_tool_name: None,
            message: None,
            was_interrupted: Some(was_interrupted),
            has_completed_turn: Some(has_completed_turn),
        }),
    })
}

fn codewhale_status_is_interrupted(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "interrupted"
            | "interrupt"
            | "cancelled"
            | "canceled"
            | "failed"
            | "aborted"
            | "abort"
            | "error"
    )
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn integer_value(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn top_level_number(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(integer_value)
}

fn nested_number(value: &Value, container: &str, keys: &[&str]) -> Option<i64> {
    top_level_number(value.get(container)?, keys)
}

fn sum_nested_numbers(value: &Value, container: &str, left: &str, right: &str) -> Option<i64> {
    let container = value.get(container)?;
    let left = container.get(left).and_then(integer_value);
    let right = container.get(right).and_then(integer_value);
    (left.is_some() || right.is_some())
        .then(|| left.unwrap_or(0).saturating_add(right.unwrap_or(0)))
}

fn number_from_containers(value: &Value, keys: &[&str]) -> Option<i64> {
    top_level_number(value, keys)
        .or_else(|| nested_number(value, "totals", keys))
        .or_else(|| nested_number(value, "usage", keys))
}

pub fn opencode_runtime_to_hook(envelope: AIToolUsageEnvelope) -> Option<AIHookEventPayload> {
    if envelope.session_id.trim().is_empty() || envelope.project_id.trim().is_empty() {
        return None;
    }

    let response_state = envelope.response_state.as_deref();
    let (kind, metadata) = match response_state {
        Some("responding") => ("promptSubmitted", None),
        Some("idle") if envelope.status == "completed" => (
            "turnCompleted",
            Some(opencode_runtime_metadata(&envelope.status, false, true)),
        ),
        Some("idle") => (
            "turnCompleted",
            Some(opencode_runtime_metadata(&envelope.status, true, false)),
        ),
        _ if envelope.status == "running" => ("promptSubmitted", None),
        _ => ("turnCompleted", None),
    };

    Some(AIHookEventPayload {
        kind: kind.to_string(),
        terminal_id: envelope.session_id,
        terminal_instance_id: envelope.session_instance_id,
        project_id: envelope.project_id,
        project_name: envelope.project_name,
        project_path: envelope.project_path,
        session_title: envelope.session_title,
        tool: envelope.tool,
        ai_session_id: envelope.external_session_id,
        model: envelope.model,
        input_tokens: envelope.input_tokens,
        output_tokens: envelope.output_tokens,
        cached_input_tokens: envelope.cached_input_tokens,
        total_tokens: envelope.total_tokens,
        updated_at: envelope.updated_at,
        metadata,
    })
}

fn opencode_runtime_metadata(
    status: &str,
    was_interrupted: bool,
    has_completed_turn: bool,
) -> AIHookEventMetadata {
    AIHookEventMetadata {
        transcript_path: None,
        notification_type: None,
        source: Some("opencode-runtime".to_string()),
        reason: Some(status.to_string()),
        cwd: None,
        target_tool_name: None,
        message: None,
        was_interrupted: Some(was_interrupted),
        has_completed_turn: Some(has_completed_turn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_opencode_runtime_to_ai_hook_payload() {
        let payload = runtime_frame_to_hook(
            br#"{
              "kind": "opencode-runtime",
              "payload": {
                "sessionId": "term-2",
                "sessionInstanceId": "inst-1",
                "externalSessionID": "external-1",
                "projectId": "project-1",
                "projectName": "Codux",
                "projectPath": "/Volumes/Web/codux-gpui",
                "sessionTitle": "Review",
                "tool": "opencode",
                "model": "model-a",
                "status": "completed",
                "responseState": "idle",
                "updatedAt": 20,
                "inputTokens": 10,
                "outputTokens": 5,
                "cachedInputTokens": 2,
                "totalTokens": 15
              }
            }"#,
        )
        .expect("payload");

        assert_eq!(payload.kind, "turnCompleted");
        assert_eq!(payload.terminal_id, "term-2");
        assert_eq!(payload.terminal_instance_id.as_deref(), Some("inst-1"));
        assert_eq!(payload.ai_session_id.as_deref(), Some("external-1"));
        assert_eq!(
            payload.project_path.as_deref(),
            Some("/Volumes/Web/codux-gpui")
        );
        assert_eq!(payload.total_tokens, Some(15));
        assert_eq!(
            payload
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.has_completed_turn),
            Some(true)
        );
    }

    #[test]
    fn decodes_codewhale_turn_end_lifecycle_payload() {
        let payload = runtime_frame_to_hook(
            br#"{
              "kind": "ai-lifecycle-hook",
              "payload": {
                "action": "codewhale-turn-end",
                "terminalID": "term-codewhale",
                "terminalInstanceID": "inst-codewhale",
                "projectID": "project-1",
                "projectName": "Codux",
                "projectPath": "/tmp/project",
                "sessionTitle": "CodeWhale",
                "tool": "codewhale",
                "model": null,
                "updatedAt": 30,
                "payload": {
                  "event": "turn_end",
                  "session_id": "cw-session-1",
                  "workspace": "/tmp/project",
                  "model": "deepseek-chat",
                  "status": "interrupted",
                  "usage": {
                    "input_tokens": 1200,
                    "output_tokens": 180,
                    "prompt_cache_hit_tokens": 900
                  },
                  "totals": {
                    "session_tokens": 1380,
                    "conversation_tokens": 1380,
                    "input_tokens": 1200,
                    "output_tokens": 180
                  }
                }
              }
            }"#,
        )
        .expect("payload");

        assert_eq!(payload.kind, "turnCompleted");
        assert_eq!(payload.tool, "codewhale");
        assert_eq!(payload.terminal_id, "term-codewhale");
        assert_eq!(payload.ai_session_id.as_deref(), Some("cw-session-1"));
        assert_eq!(payload.model.as_deref(), Some("deepseek-chat"));
        assert_eq!(payload.input_tokens, Some(1200));
        assert_eq!(payload.output_tokens, Some(180));
        assert_eq!(payload.cached_input_tokens, Some(900));
        assert_eq!(payload.total_tokens, Some(1380));
        let metadata = payload.metadata.expect("metadata");
        assert_eq!(metadata.was_interrupted, Some(true));
        assert_eq!(metadata.has_completed_turn, Some(false));
        assert_eq!(metadata.source.as_deref(), Some("codewhale-lifecycle"));
        assert_eq!(metadata.reason.as_deref(), Some("interrupted"));
    }
}
