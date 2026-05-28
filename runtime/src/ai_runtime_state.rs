use crate::{
    ai_runtime::{AIRuntimeStateSnapshot, AISessionSnapshot},
    runtime_event::{RuntimeEventSummary, RuntimeSessionSummary},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const STATE_FILE_NAME: &str = "gpui-ai-runtime-state.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRuntimeStateSummary {
    pub path: String,
    pub updated_at: f64,
    pub running_count: usize,
    pub needs_input_count: usize,
    pub completed_count: usize,
    pub session_count: usize,
    pub sessions: Vec<AIRuntimeSessionSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AIRuntimeSessionSummary {
    pub terminal_id: String,
    pub tool: String,
    pub state: String,
    pub project_name: String,
    pub session_title: String,
    pub updated_at: f64,
    pub event_count: usize,
    pub source: String,
}

pub struct AIRuntimeStateService {
    state_file: PathBuf,
}

impl AIRuntimeStateService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            state_file: support_dir.join(STATE_FILE_NAME),
        }
    }

    pub fn summary(&self) -> AIRuntimeStateSummary {
        let raw = self.raw_snapshot();
        summary_from_raw(self.state_file.display().to_string(), &raw, None)
    }

    pub fn save_from_events(
        &self,
        events: &RuntimeEventSummary,
    ) -> Result<AIRuntimeStateSummary, String> {
        let mut raw = self.raw_snapshot();
        let now = now_seconds();
        let mut sessions = events
            .sessions
            .iter()
            .map(session_from_runtime_event)
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.updated_at.total_cmp(&left.updated_at));

        raw.insert("schemaVersion".to_string(), json!(1));
        raw.insert("source".to_string(), json!("gpui"));
        raw.insert("updatedAt".to_string(), json!(now));
        raw.insert("runningCount".to_string(), json!(events.running_count));
        raw.insert(
            "needsInputCount".to_string(),
            json!(events.needs_input_count),
        );
        raw.insert("completedCount".to_string(), json!(events.completed_count));
        raw.insert("sessionCount".to_string(), json!(sessions.len()));
        raw.insert("sessions".to_string(), json!(sessions));
        self.save_raw_snapshot(&raw)?;
        Ok(summary_from_raw(
            self.state_file.display().to_string(),
            &raw,
            None,
        ))
    }

    pub fn save_from_runtime_snapshot(
        &self,
        snapshot: &AIRuntimeStateSnapshot,
    ) -> Result<AIRuntimeStateSummary, String> {
        let mut raw = self.raw_snapshot();
        let mut sessions = snapshot
            .sessions
            .iter()
            .map(session_from_runtime_snapshot)
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.updated_at.total_cmp(&left.updated_at));

        raw.insert("schemaVersion".to_string(), json!(1));
        raw.insert("source".to_string(), json!("gpui-supervisor"));
        raw.insert("updatedAt".to_string(), json!(snapshot.updated_at));
        raw.insert("runningCount".to_string(), json!(snapshot.running_count));
        raw.insert(
            "needsInputCount".to_string(),
            json!(snapshot.needs_input_count),
        );
        raw.insert(
            "completedCount".to_string(),
            json!(snapshot.completion_count),
        );
        raw.insert("sessionCount".to_string(), json!(sessions.len()));
        raw.insert("sessions".to_string(), json!(sessions));
        self.save_raw_snapshot(&raw)?;
        Ok(summary_from_raw(
            self.state_file.display().to_string(),
            &raw,
            None,
        ))
    }

    fn raw_snapshot(&self) -> Map<String, Value> {
        fs::read_to_string(&self.state_file)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    fn save_raw_snapshot(&self, snapshot: &Map<String, Value>) -> Result<(), String> {
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let content = serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?;
        fs::write(&self.state_file, format!("{content}\n")).map_err(|error| error.to_string())
    }
}

fn summary_from_raw(
    path: String,
    raw: &Map<String, Value>,
    error: Option<String>,
) -> AIRuntimeStateSummary {
    let mut sessions = raw_sessions(raw);
    sessions.sort_by(|left, right| right.updated_at.total_cmp(&left.updated_at));
    let running_count = raw
        .get("runningCount")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            sessions
                .iter()
                .filter(|session| session.state == "running")
                .count()
        });
    let needs_input_count = raw
        .get("needsInputCount")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            sessions
                .iter()
                .filter(|session| session.state == "needs-input")
                .count()
        });
    let completed_count = raw
        .get("completedCount")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| {
            sessions
                .iter()
                .filter(|session| session.state == "completed")
                .count()
        });
    AIRuntimeStateSummary {
        path,
        updated_at: raw.get("updatedAt").and_then(Value::as_f64).unwrap_or(0.0),
        running_count,
        needs_input_count,
        completed_count,
        session_count: raw
            .get("sessionCount")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(sessions.len()),
        sessions,
        error,
    }
}

fn raw_sessions(raw: &Map<String, Value>) -> Vec<AIRuntimeSessionSummary> {
    raw.get("sessions")
        .and_then(Value::as_array)
        .map(|sessions| {
            sessions
                .iter()
                .filter_map(|session| {
                    serde_json::from_value::<AIRuntimeSessionSummary>(session.clone()).ok()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn session_from_runtime_event(session: &RuntimeSessionSummary) -> AIRuntimeSessionSummary {
    AIRuntimeSessionSummary {
        terminal_id: session.terminal_id.clone(),
        tool: session.tool.clone(),
        state: session.state.clone(),
        project_name: session.project_name.clone(),
        session_title: session.session_title.clone(),
        updated_at: session.updated_at,
        event_count: session.event_count,
        source: "runtime-events".to_string(),
    }
}

fn session_from_runtime_snapshot(session: &AISessionSnapshot) -> AIRuntimeSessionSummary {
    AIRuntimeSessionSummary {
        terminal_id: session.terminal_id.clone(),
        tool: session.tool.clone(),
        state: runtime_snapshot_session_state(session).to_string(),
        project_name: session.project_name.clone(),
        session_title: session.session_title.clone(),
        updated_at: session.updated_at,
        event_count: usize::from(session.started_at.is_some())
            + usize::from(session.has_completed_turn)
            + usize::from(session.notification_type.is_some()),
        source: "supervisor".to_string(),
    }
}

fn runtime_snapshot_session_state(session: &AISessionSnapshot) -> &'static str {
    if session.state == "needsInput" || session.notification_type.is_some() {
        "needs-input"
    } else if session.is_running || session.state == "responding" {
        "running"
    } else if session.has_completed_turn {
        "completed"
    } else {
        "idle"
    }
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn save_from_events_persists_sessions_and_preserves_unknown_fields() {
        let dir = std::env::temp_dir().join(format!("codux-gpui-ai-runtime-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(STATE_FILE_NAME),
            r#"{"schemaVersion":1,"customField":{"keep":true}}"#,
        )
        .unwrap();
        let service = AIRuntimeStateService::new(dir.clone());
        let events = RuntimeEventSummary {
            running_count: 1,
            needs_input_count: 1,
            completed_count: 0,
            sessions: vec![
                RuntimeSessionSummary {
                    terminal_id: "term-a".to_string(),
                    tool: "codex".to_string(),
                    state: "running".to_string(),
                    project_name: "Codux".to_string(),
                    session_title: "Build GPUI".to_string(),
                    updated_at: 10.0,
                    event_count: 2,
                },
                RuntimeSessionSummary {
                    terminal_id: "term-b".to_string(),
                    tool: "claude".to_string(),
                    state: "needs-input".to_string(),
                    project_name: "Codux".to_string(),
                    session_title: "Review".to_string(),
                    updated_at: 20.0,
                    event_count: 3,
                },
            ],
            ..Default::default()
        };

        let summary = service.save_from_events(&events).unwrap();

        assert_eq!(summary.session_count, 2);
        assert_eq!(summary.running_count, 1);
        assert_eq!(summary.needs_input_count, 1);
        assert_eq!(summary.sessions[0].terminal_id, "term-b");
        assert_eq!(summary.sessions[0].source, "runtime-events");
        let raw = service.raw_snapshot();
        assert_eq!(
            raw.get("customField")
                .and_then(|value| value.get("keep"))
                .and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn summary_returns_default_for_missing_file() {
        let dir =
            std::env::temp_dir().join(format!("codux-gpui-ai-runtime-empty-{}", Uuid::new_v4()));
        let summary = AIRuntimeStateService::new(dir.clone()).summary();

        assert_eq!(summary.session_count, 0);
        assert_eq!(summary.running_count, 0);
        assert!(summary.path.ends_with(STATE_FILE_NAME));
    }

    #[test]
    fn save_from_runtime_snapshot_persists_live_supervisor_state() {
        let dir =
            std::env::temp_dir().join(format!("codux-gpui-ai-runtime-live-{}", Uuid::new_v4()));
        let service = AIRuntimeStateService::new(dir.clone());
        let snapshot = AIRuntimeStateSnapshot {
            running_count: 1,
            needs_input_count: 1,
            completion_count: 0,
            updated_at: 42.0,
            sessions: vec![
                AISessionSnapshot {
                    terminal_id: "term-a".to_string(),
                    terminal_instance_id: None,
                    project_id: "project-a".to_string(),
                    project_name: "Codux".to_string(),
                    project_path: None,
                    session_title: "Build".to_string(),
                    tool: "codex".to_string(),
                    ai_session_id: None,
                    model: None,
                    state: "responding".to_string(),
                    status: "running".to_string(),
                    is_running: true,
                    input_tokens: 0,
                    output_tokens: 0,
                    cached_input_tokens: 0,
                    total_tokens: 0,
                    baseline_total_tokens: 0,
                    baseline_cached_input_tokens: 0,
                    baseline_resolved: false,
                    started_at: Some(10.0),
                    updated_at: 20.0,
                    active_turn_started_at: None,
                    runtime_turn_started_at: None,
                    has_completed_turn: false,
                    was_interrupted: false,
                    transcript_path: None,
                    notification_type: None,
                    target_tool_name: None,
                    message: None,
                    latest_assistant_preview: None,
                },
                AISessionSnapshot {
                    terminal_id: "term-b".to_string(),
                    terminal_instance_id: None,
                    project_id: "project-b".to_string(),
                    project_name: "Codux".to_string(),
                    project_path: None,
                    session_title: "Review".to_string(),
                    tool: "claude".to_string(),
                    ai_session_id: None,
                    model: None,
                    state: "needsInput".to_string(),
                    status: "needs input".to_string(),
                    is_running: false,
                    input_tokens: 0,
                    output_tokens: 0,
                    cached_input_tokens: 0,
                    total_tokens: 0,
                    baseline_total_tokens: 0,
                    baseline_cached_input_tokens: 0,
                    baseline_resolved: false,
                    started_at: Some(11.0),
                    updated_at: 30.0,
                    active_turn_started_at: None,
                    runtime_turn_started_at: None,
                    has_completed_turn: false,
                    was_interrupted: false,
                    transcript_path: None,
                    notification_type: Some("approval".to_string()),
                    target_tool_name: None,
                    message: None,
                    latest_assistant_preview: None,
                },
            ],
            ..Default::default()
        };

        let summary = service.save_from_runtime_snapshot(&snapshot).unwrap();

        assert_eq!(summary.session_count, 2);
        assert_eq!(summary.running_count, 1);
        assert_eq!(summary.needs_input_count, 1);
        assert_eq!(summary.sessions[0].terminal_id, "term-b");
        assert_eq!(summary.sessions[0].state, "needs-input");
        assert_eq!(summary.sessions[1].state, "running");

        fs::remove_dir_all(dir).unwrap();
    }
}
