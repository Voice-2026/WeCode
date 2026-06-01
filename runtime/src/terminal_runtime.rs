use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const RUNTIME_FILE_NAME: &str = "gpui-terminal-runtime.json";
const CLOSED_HISTORY_LIMIT: usize = 24;
const INPUT_HISTORY_LIMIT: usize = 20;
const INPUT_TEXT_LIMIT: usize = 240;
const OUTPUT_TAIL_LIMIT: usize = 12 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeSummary {
    pub path: String,
    pub active_terminal_id: String,
    pub active_slot_id: String,
    pub open_count: usize,
    pub closed_count: usize,
    pub sessions: Vec<TerminalRuntimeSessionSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeSessionSummary {
    pub terminal_id: String,
    pub slot_id: String,
    pub tab_id: String,
    pub pane_index: usize,
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
    pub source: String,
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
    pub slot_id: String,
    pub tab_id: String,
    pub pane_index: usize,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub cwd: String,
    pub input_bytes: usize,
    pub input_history: Vec<TerminalInputSummary>,
    pub output_bytes: usize,
    pub output_tail: String,
}

pub struct TerminalRuntimeService {
    runtime_file: PathBuf,
}

impl TerminalRuntimeService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            runtime_file: support_dir.join(RUNTIME_FILE_NAME),
        }
    }

    pub fn summary(&self) -> TerminalRuntimeSummary {
        match self.read_sessions() {
            Ok((active_terminal_id, active_slot_id, sessions)) => summary_from_sessions(
                self.runtime_file.display().to_string(),
                active_terminal_id,
                active_slot_id,
                sessions,
                None,
            ),
            Err(error) => TerminalRuntimeSummary {
                path: self.runtime_file.display().to_string(),
                error: Some(error),
                ..Default::default()
            },
        }
    }

    pub fn save_from_gpui(
        &self,
        active_terminal_id: String,
        active_slot_id: String,
        sessions: Vec<TerminalRuntimeSessionInput>,
    ) -> Result<TerminalRuntimeSummary, String> {
        let now = now_seconds();
        let mut raw = self.raw_snapshot();
        let existing_sessions = raw_sessions(&raw);
        let mut created_at_by_key = HashMap::new();
        for session in &existing_sessions {
            created_at_by_key.insert(
                session_key(&session.terminal_id, &session.slot_id),
                session.created_at,
            );
        }

        let open_keys = sessions
            .iter()
            .map(|session| session_key(&session.terminal_id, &session.slot_id))
            .collect::<HashSet<_>>();
        let mut next_sessions = sessions
            .into_iter()
            .map(|session| {
                let key = session_key(&session.terminal_id, &session.slot_id);
                TerminalRuntimeSessionSummary {
                    terminal_id: session.terminal_id,
                    slot_id: session.slot_id,
                    tab_id: session.tab_id,
                    pane_index: session.pane_index,
                    title: session.title,
                    project_id: session.project_id,
                    project_name: session.project_name,
                    project_path: session.project_path,
                    cwd: session.cwd,
                    status: "running".to_string(),
                    is_running: true,
                    created_at: created_at_by_key.get(&key).copied().unwrap_or(now),
                    last_active_at: now,
                    has_buffer: false,
                    buffer_characters: 0,
                    input_bytes: session.input_bytes,
                    last_input_at: session.input_history.last().map(|input| input.timestamp),
                    input_history: sanitize_input_history(session.input_history),
                    output_bytes: session.output_bytes,
                    output_tail: sanitize_output_tail(&session.output_tail),
                    source: "gpui".to_string(),
                }
            })
            .collect::<Vec<_>>();

        let mut closed = existing_sessions
            .into_iter()
            .filter(|session| {
                !open_keys.contains(&session_key(&session.terminal_id, &session.slot_id))
            })
            .map(|mut session| {
                session.status = "closed".to_string();
                session.is_running = false;
                session.last_active_at = now;
                session.has_buffer = false;
                session.buffer_characters = 0;
                session.input_history = sanitize_input_history(session.input_history);
                session.output_tail = sanitize_output_tail(&session.output_tail);
                session
            })
            .collect::<Vec<_>>();
        closed.sort_by(|a, b| b.last_active_at.total_cmp(&a.last_active_at));
        closed.truncate(CLOSED_HISTORY_LIMIT);
        next_sessions.extend(closed);

        raw.insert("schemaVersion".to_string(), json!(1));
        raw.insert("source".to_string(), json!("gpui"));
        raw.insert("activeTerminalId".to_string(), json!(active_terminal_id));
        raw.insert("activeSlotId".to_string(), json!(active_slot_id));
        raw.insert("updatedAt".to_string(), json!(now));
        raw.insert("sessions".to_string(), json!(next_sessions));
        self.save_raw_snapshot(&raw)?;
        Ok(self.summary())
    }

    fn read_sessions(
        &self,
    ) -> Result<(String, String, Vec<TerminalRuntimeSessionSummary>), String> {
        let raw = self.raw_snapshot();
        let active_terminal_id = raw
            .get("activeTerminalId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let active_slot_id = raw
            .get("activeSlotId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok((active_terminal_id, active_slot_id, raw_sessions(&raw)))
    }

    fn raw_snapshot(&self) -> Map<String, Value> {
        crate::config::ConfigStore::for_file(self.runtime_file.clone()).snapshot()
    }

    fn save_raw_snapshot(&self, snapshot: &Map<String, Value>) -> Result<(), String> {
        crate::config::ConfigStore::for_file(self.runtime_file.clone()).save_snapshot(snapshot)
    }
}

fn raw_sessions(raw: &Map<String, Value>) -> Vec<TerminalRuntimeSessionSummary> {
    raw.get("sessions")
        .and_then(Value::as_array)
        .map(|sessions| {
            sessions
                .iter()
                .filter_map(|session| {
                    serde_json::from_value::<TerminalRuntimeSessionSummary>(session.clone()).ok()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn summary_from_sessions(
    path: String,
    active_terminal_id: String,
    active_slot_id: String,
    sessions: Vec<TerminalRuntimeSessionSummary>,
    error: Option<String>,
) -> TerminalRuntimeSummary {
    let open_count = sessions.iter().filter(|session| session.is_running).count();
    let closed_count = sessions.len().saturating_sub(open_count);
    TerminalRuntimeSummary {
        path,
        active_terminal_id,
        active_slot_id,
        open_count,
        closed_count,
        sessions,
        error,
    }
}

fn session_key(terminal_id: &str, slot_id: &str) -> String {
    format!("{terminal_id}\n{slot_id}")
}

pub fn sanitized_terminal_input(text: &str) -> String {
    let normalized = text
        .trim_end_matches(['\r', '\n'])
        .chars()
        .filter_map(|ch| match ch {
            '\r' | '\n' | '\t' => Some(' '),
            ch if ch.is_control() => None,
            ch => Some(ch),
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    redact_sensitive_input(&normalized)
        .chars()
        .take(INPUT_TEXT_LIMIT)
        .collect()
}

fn redact_sensitive_input(text: &str) -> String {
    text.split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            let sensitive = [
                "api_key",
                "apikey",
                "token",
                "password",
                "secret",
                "passphrase",
            ]
            .iter()
            .any(|needle| lower.contains(needle));
            if !sensitive {
                return part.to_string();
            }
            if let Some((key, _)) = part.split_once('=') {
                format!("{key}=<redacted>")
            } else {
                "<redacted>".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_input_history(history: Vec<TerminalInputSummary>) -> Vec<TerminalInputSummary> {
    let mut history = history
        .into_iter()
        .filter_map(|mut input| {
            input.text = sanitized_terminal_input(&input.text);
            if input.text.is_empty() {
                return None;
            }
            Some(input)
        })
        .collect::<Vec<_>>();
    if history.len() > INPUT_HISTORY_LIMIT {
        history = history.split_off(history.len() - INPUT_HISTORY_LIMIT);
    }
    history
}

pub fn sanitize_output_tail(output: &str) -> String {
    let cleaned = strip_ansi_control_sequences(output)
        .chars()
        .filter_map(|ch| match ch {
            '\r' => None,
            '\n' | '\t' => Some(ch),
            ch if ch.is_control() => None,
            ch => Some(ch),
        })
        .collect::<String>();
    let bytes = cleaned.as_bytes();
    if bytes.len() <= OUTPUT_TAIL_LIMIT {
        return cleaned;
    }
    let start = bytes.len() - OUTPUT_TAIL_LIMIT;
    let start = cleaned
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| *index >= start)
        .unwrap_or(start);
    cleaned[start..].to_string()
}

fn strip_ansi_control_sequences(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            output.push(ch);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        }
    }
    output
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn save_from_gpui_preserves_unknown_fields_and_marks_closed_sessions() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-gpui-terminal-runtime-{}", Uuid::new_v4()));
        fs::create_dir_all(&support_dir).unwrap();
        let runtime_file = support_dir.join(RUNTIME_FILE_NAME);
        fs::write(
            &runtime_file,
            r#"{
  "customField": "keep",
  "activeTerminalId": "old-term",
  "activeSlotId": "old-slot",
  "sessions": [
    {
      "terminalId": "old-term",
      "slotId": "old-slot",
      "tabId": "old-tab",
      "paneIndex": 0,
      "title": "Old",
      "projectId": "project-1",
      "projectName": "Codux",
      "projectPath": "/workspace/codux",
      "cwd": "/workspace/codux",
      "status": "running",
      "isRunning": true,
      "createdAt": 10.0,
      "lastActiveAt": 10.0,
      "hasBuffer": true,
      "bufferCharacters": 99,
      "inputBytes": 11,
      "lastInputAt": 10.0,
      "inputHistory": [{"text": "old command", "bytes": 11, "timestamp": 10.0}],
      "outputBytes": 12,
      "outputTail": "old output",
      "source": "gpui"
    }
  ]
}
"#,
        )
        .unwrap();

        let service = TerminalRuntimeService::new(support_dir.clone());
        let summary = service
            .save_from_gpui(
                "new-term".to_string(),
                "new-slot".to_string(),
                vec![TerminalRuntimeSessionInput {
                    terminal_id: "new-term".to_string(),
                    slot_id: "new-slot".to_string(),
                    tab_id: "bottom-1".to_string(),
                    pane_index: 0,
                    title: "Terminal".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Codux".to_string(),
                    project_path: "/workspace/codux".to_string(),
                    cwd: "/workspace/codux".to_string(),
                    input_bytes: 8,
                    input_history: vec![TerminalInputSummary {
                        text: "codex\n".to_string(),
                        bytes: 6,
                        timestamp: 20.0,
                    }],
                    output_bytes: 12,
                    output_tail: "\u{1b}[32mok\u{1b}[0m\n".to_string(),
                }],
            )
            .unwrap();

        assert_eq!(summary.open_count, 1);
        assert_eq!(summary.closed_count, 1);
        assert!(
            summary
                .sessions
                .iter()
                .any(|session| session.terminal_id == "old-term"
                    && session.status == "closed"
                    && !session.is_running
                    && !session.has_buffer
                    && session.buffer_characters == 0)
        );
        let open = summary
            .sessions
            .iter()
            .find(|session| session.terminal_id == "new-term")
            .unwrap();
        assert_eq!(open.input_bytes, 8);
        assert_eq!(open.last_input_at, Some(20.0));
        assert_eq!(open.input_history[0].text, "codex");
        assert_eq!(open.output_bytes, 12);
        assert_eq!(open.output_tail, "ok\n");
        let raw = crate::config::ConfigStore::for_file(runtime_file).snapshot();
        assert_eq!(
            raw.get("customField").and_then(Value::as_str),
            Some("keep")
        );
        fs::remove_dir_all(support_dir).unwrap();
    }

    #[test]
    fn save_from_gpui_does_not_serialize_environment_or_secret_fields() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-gpui-terminal-runtime-{}", Uuid::new_v4()));
        let service = TerminalRuntimeService::new(support_dir.clone());
        service
            .save_from_gpui(
                "term-1".to_string(),
                "slot-1".to_string(),
                vec![TerminalRuntimeSessionInput {
                    terminal_id: "term-1".to_string(),
                    slot_id: "slot-1".to_string(),
                    tab_id: "bottom-1".to_string(),
                    pane_index: 0,
                    title: "Terminal".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Codux".to_string(),
                    project_path: "/workspace/codux".to_string(),
                    cwd: "/workspace/codux".to_string(),
                    input_bytes: 0,
                    input_history: vec![TerminalInputSummary {
                        text: "export OPENAI_API_KEY=secret\n".to_string(),
                        bytes: 29,
                        timestamp: 1.0,
                    }],
                    output_bytes: 0,
                    output_tail: String::new(),
                }],
            )
            .unwrap();

        let raw = serde_json::to_string(
            &crate::config::ConfigStore::for_file(support_dir.join(RUNTIME_FILE_NAME)).snapshot(),
        )
        .unwrap();
        assert!(!raw.contains("env"));
        assert!(!raw.contains("secret"));
        assert!(raw.contains("OPENAI_API_KEY=<redacted>"));
        assert!(!raw.contains("password"));
        fs::remove_dir_all(support_dir).unwrap();
    }

    #[test]
    fn sanitizes_terminal_input_history() {
        assert_eq!(
            sanitized_terminal_input("codex resume abc\n"),
            "codex resume abc"
        );
        assert_eq!(
            sanitized_terminal_input("\u{1b}[A\tls -la\r\n"),
            "[A ls -la"
        );
        assert_eq!(
            sanitized_terminal_input("export OPENAI_API_KEY=secret"),
            "export OPENAI_API_KEY=<redacted>"
        );

        let history = (0..25)
            .map(|index| TerminalInputSummary {
                text: format!(" command {index}\n"),
                bytes: 10,
                timestamp: index as f64,
            })
            .collect::<Vec<_>>();
        let sanitized = sanitize_input_history(history);
        assert_eq!(sanitized.len(), INPUT_HISTORY_LIMIT);
        assert_eq!(sanitized[0].text, "command 5");
        assert_eq!(sanitized.last().unwrap().text, "command 24");
    }

    #[test]
    fn sanitizes_terminal_output_tail() {
        assert_eq!(
            sanitize_output_tail("hello\r\n\u{1b}[31mred\u{1b}[0m"),
            "hello\nred"
        );
        let long = "a".repeat(OUTPUT_TAIL_LIMIT + 10);
        assert_eq!(sanitize_output_tail(&long).len(), OUTPUT_TAIL_LIMIT);
    }
}
