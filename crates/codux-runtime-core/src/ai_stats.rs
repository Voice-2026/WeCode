use codux_protocol::RemoteAICurrentSession;
use serde_json::{Value, json};

pub trait RemoteAICurrentSessionProvider: Send + Sync {
    fn current_sessions(&self, project_id: &str) -> Vec<RemoteAICurrentSession>;
}

pub fn ai_stats_payload_from_state_value(
    project_id: impl Into<String>,
    project_name: impl Into<String>,
    state: Value,
    current_sessions: Vec<RemoteAICurrentSession>,
) -> Value {
    let project_id = project_id.into();
    let project_name = project_name.into();
    let mut state = state;
    let snapshot = state
        .get_mut("snapshot")
        .map(Value::take)
        .filter(|value| !value.is_null());
    let mut payload =
        snapshot.unwrap_or_else(|| empty_ai_stats_payload(&project_id, &project_name));
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("projectId")
            .or_insert_with(|| json!(project_id.clone()));
        object
            .entry("projectName")
            .or_insert_with(|| json!(project_name.clone()));
        object.insert(
            "updatedAt".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
        object.insert("currentSessions".to_string(), json!(current_sessions));
    }
    payload
}

pub fn ai_stats_payload_from_state<T: serde::Serialize>(
    project_id: impl Into<String>,
    project_name: impl Into<String>,
    state: T,
    current_sessions: Vec<RemoteAICurrentSession>,
) -> Result<Value, String> {
    let value = serde_json::to_value(state).map_err(|error| error.to_string())?;
    Ok(ai_stats_payload_from_state_value(
        project_id,
        project_name,
        value,
        current_sessions,
    ))
}

pub fn empty_ai_stats_payload(project_id: &str, project_name: &str) -> Value {
    json!({
        "projectId": project_id,
        "projectName": project_name,
        "projectSummary": {},
        "sessions": [],
        "heatmap": [],
        "todayTimeBuckets": [],
        "toolBreakdown": [],
        "modelBreakdown": [],
        "currentSessions": [],
    })
}

pub fn current_sessions_from_payload(payload: &Value) -> Vec<RemoteAICurrentSession> {
    payload
        .get("currentSessions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|session| serde_json::from_value(session.clone()).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_snapshot_is_flattened_and_current_sessions_are_merged() {
        let payload = ai_stats_payload_from_state_value(
            "project-1",
            "Project",
            json!({
                "projectId": "project-1",
                "projectName": "Project",
                "snapshot": {
                    "projectId": "project-1",
                    "projectName": "Project",
                    "projectSummary": {"projectTotalTokens": 12},
                    "sessions": [],
                    "heatmap": [],
                    "todayTimeBuckets": [],
                    "toolBreakdown": [],
                    "modelBreakdown": []
                }
            }),
            vec![RemoteAICurrentSession {
                session_id: "ai-1".to_string(),
                terminal_id: Some("term-1".to_string()),
                project_id: "project-1".to_string(),
                title: "Codex".to_string(),
                tool: "codex".to_string(),
                status: "running".to_string(),
                is_running: true,
                total_tokens: 42,
                ..Default::default()
            }],
        );

        assert_eq!(payload["projectSummary"]["projectTotalTokens"], 12);
        assert_eq!(payload["currentSessions"][0]["sessionId"], "ai-1");
        assert_eq!(payload["currentSessions"][0]["terminalId"], "term-1");
    }

    #[test]
    fn missing_snapshot_uses_empty_mobile_shape() {
        let payload = ai_stats_payload_from_state_value(
            "project-1",
            "Project",
            json!({"snapshot": null}),
            Vec::new(),
        );

        assert_eq!(payload["projectId"], "project-1");
        assert!(payload["sessions"].as_array().unwrap().is_empty());
        assert!(payload["currentSessions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn current_sessions_parse_from_payload() {
        let sessions = current_sessions_from_payload(&json!({
            "currentSessions": [{
                "sessionId": "ai-1",
                "terminalId": "term-1",
                "title": "Codex",
                "tool": "codex",
                "status": "running",
                "isRunning": true,
                "totalTokens": 10,
                "cachedInputTokens": 4
            }]
        }));

        assert_eq!(sessions[0].session_id, "ai-1");
        assert_eq!(sessions[0].terminal_id.as_deref(), Some("term-1"));
        assert_eq!(sessions[0].total_tokens, 10);
        assert_eq!(sessions[0].cached_input_tokens, 4);
    }
}
