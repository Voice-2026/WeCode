use codux_terminal_core::{
    TerminalDriver, TerminalEvent, TerminalSessionHandle, TerminalSessionSnapshot,
};
use serde_json::{Value, json};

pub trait TerminalDomainDriver: TerminalDriver {}

impl<T> TerminalDomainDriver for T where T: TerminalDriver {}

pub trait TerminalDomainSession: TerminalSessionHandle {}

impl<T> TerminalDomainSession for T where T: TerminalSessionHandle {}

pub fn terminal_snapshot_payload(terminal: TerminalSessionSnapshot, layout_kind: &str) -> Value {
    json!({
        "id": terminal.id,
        "title": terminal.title,
        "layoutKind": layout_kind,
        "displayTitle": if terminal.project_name.trim().is_empty() {
            terminal.title.clone()
        } else {
            format!("{} · {}", terminal.project_name, terminal.title)
        },
        "projectId": terminal.project_id,
        "worktreeId": terminal.worktree_id,
        "projectName": terminal.project_name,
        "projectPath": terminal.cwd,
        "cwd": terminal.cwd,
        "shell": terminal.shell,
        "command": terminal.command,
        "cols": terminal.cols,
        "rows": terminal.rows,
        "status": terminal.status,
        "isRunning": terminal.is_running,
        "createdAt": terminal.created_at,
        "lastActiveAt": terminal.last_active_at,
        "bufferCharacters": terminal.buffer_characters,
        "hasBuffer": terminal.has_buffer,
    })
}

pub fn terminal_order_key(value: &Value) -> (String, String) {
    let created_at = value
        .get("createdAt")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    (created_at, id)
}

pub fn terminal_event_session_id(event: &TerminalEvent) -> &str {
    match event {
        TerminalEvent::Output { session_id, .. }
        | TerminalEvent::Exit { session_id, .. }
        | TerminalEvent::Error { session_id, .. }
        | TerminalEvent::Viewport { session_id, .. } => session_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_payload_uses_project_title_for_display() {
        let payload = terminal_snapshot_payload(
            TerminalSessionSnapshot {
                id: "term-1".to_string(),
                title: "Codex".to_string(),
                slot_id: "slot".to_string(),
                session_key: None,
                project_id: "project-1".to_string(),
                worktree_id: Some("worktree-1".to_string()),
                project_name: "Codux".to_string(),
                cwd: "/tmp/codux".to_string(),
                shell: "/bin/zsh".to_string(),
                command: String::new(),
                cols: 100,
                rows: 32,
                status: "running".to_string(),
                is_running: true,
                created_at: "1".to_string(),
                last_active_at: "2".to_string(),
                buffer_characters: 42,
                has_buffer: true,
                tool: Some("codex".to_string()),
            },
            "split",
        );

        assert_eq!(payload["displayTitle"], "Codux · Codex");
        assert_eq!(payload["worktreeId"], "worktree-1");
        assert_eq!(payload["layoutKind"], "split");
        assert_eq!(payload["bufferCharacters"], 42);
    }
}
