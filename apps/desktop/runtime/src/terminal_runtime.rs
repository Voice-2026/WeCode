use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const TERMINAL_RUNTIME_NAMESPACE: &str = "terminal-runtime";
const MAX_PERSISTED_OUTPUT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeSummary {
    pub path: String,
    pub active_terminal_id: String,
    pub open_count: usize,
    pub closed_count: usize,
    pub sessions: Vec<TerminalRuntimeSessionSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeSessionSummary {
    pub terminal_id: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub cwd: String,
    pub status: String,
    pub is_running: bool,
    pub created_at: f64,
    pub last_active_at: f64,
    pub has_buffer: bool,
    pub buffer_characters: usize,
    #[serde(default)]
    pub input_bytes: usize,
    #[serde(default)]
    pub last_input_at: Option<f64>,
    #[serde(default)]
    pub input_history: Vec<TerminalInputSummary>,
    #[serde(default)]
    pub output_bytes: usize,
    #[serde(default)]
    pub output_tail: String,
    #[serde(default)]
    pub ai_tool: Option<String>,
    #[serde(default)]
    pub ai_session_id: Option<String>,
    #[serde(default)]
    pub ai_model: Option<String>,
    #[serde(default)]
    pub ai_launch_mode: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInputSummary {
    pub text: String,
    pub bytes: usize,
    pub timestamp: f64,
}

#[derive(Clone, Debug)]
pub struct TerminalRuntimeSessionInput {
    pub terminal_id: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub cwd: String,
    pub input_bytes: usize,
    pub input_history: Vec<TerminalInputSummary>,
    pub output_bytes: usize,
    pub output_tail: String,
    pub ai_tool: Option<String>,
    pub ai_session_id: Option<String>,
    pub ai_model: Option<String>,
    pub ai_launch_mode: Option<String>,
}

pub struct TerminalRuntimeService {
    support_dir: PathBuf,
}

impl TerminalRuntimeService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self { support_dir }
    }

    pub fn load(&self, owner_id: Option<&str>) -> TerminalRuntimeSummary {
        let Some(owner_id) = owner_id else {
            return TerminalRuntimeSummary::default();
        };
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())
            .ok()
            .and_then(|cache| {
                cache
                    .get_json::<TerminalRuntimeSummary>(TERMINAL_RUNTIME_NAMESPACE, owner_id)
                    .ok()
                    .flatten()
            })
            .unwrap_or_default()
    }

    pub fn save(
        &self,
        owner_id: &str,
        mut summary: TerminalRuntimeSummary,
    ) -> Result<TerminalRuntimeSummary, String> {
        for session in &mut summary.sessions {
            session.output_tail = persisted_output_tail(&session.output_tail);
        }
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())?
            .put_json(TERMINAL_RUNTIME_NAMESPACE, owner_id, &summary)?;
        Ok(summary)
    }
}

fn persisted_output_tail(output: &str) -> String {
    if output.len() <= MAX_PERSISTED_OUTPUT_BYTES {
        return output.to_string();
    }
    let mut start = output.len() - MAX_PERSISTED_OUTPUT_BYTES;
    while start < output.len() && !output.is_char_boundary(start) {
        start += 1;
    }
    output[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_output_tail_keeps_utf8_boundary() {
        let output = format!("{}done", "\u{754c}".repeat(MAX_PERSISTED_OUTPUT_BYTES));
        let tail = persisted_output_tail(&output);
        assert!(tail.len() <= MAX_PERSISTED_OUTPUT_BYTES);
        assert!(tail.ends_with("done"));
    }

    #[test]
    fn terminal_runtime_round_trips_through_persistent_cache() {
        let support_dir = std::env::temp_dir().join(format!(
            "wecode-terminal-runtime-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = TerminalRuntimeService::new(support_dir.clone());
        let summary = TerminalRuntimeSummary {
            sessions: vec![TerminalRuntimeSessionSummary {
                terminal_id: "terminal-1".to_string(),
                title: "Build".to_string(),
                project_id: "project-1".to_string(),
                project_name: "Project".to_string(),
                project_path: "/tmp/project".to_string(),
                cwd: "/tmp/project".to_string(),
                status: "running".to_string(),
                is_running: true,
                created_at: 1.0,
                last_active_at: 2.0,
                has_buffer: true,
                buffer_characters: 6,
                input_bytes: 0,
                last_input_at: None,
                input_history: Vec::new(),
                output_bytes: 6,
                output_tail: "output".to_string(),
                ai_tool: Some("claude".to_string()),
                ai_session_id: Some("session-1".to_string()),
                ai_model: Some("claude-opus-4-8".to_string()),
                ai_launch_mode: Some("kiroGateway".to_string()),
            }],
            ..TerminalRuntimeSummary::default()
        };

        service.save("owner-1", summary).unwrap();
        let restored = service.load(Some("owner-1"));
        assert_eq!(restored.sessions.len(), 1);
        assert_eq!(restored.sessions[0].title, "Build");
        assert_eq!(restored.sessions[0].output_tail, "output");
        assert_eq!(restored.sessions[0].ai_tool.as_deref(), Some("claude"));
        assert_eq!(
            restored.sessions[0].ai_session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            restored.sessions[0].ai_launch_mode.as_deref(),
            Some("kiroGateway")
        );
        let _ = std::fs::remove_dir_all(support_dir);
    }
}
