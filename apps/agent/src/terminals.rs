//! Terminal domain for the headless host: spawn real PTYs with the same
//! `TerminalManager` the desktop host uses, so AI runtime tracking and terminal
//! protocol behavior stay aligned.

use codux_protocol::{
    terminal_live_output_payload, REMOTE_ERROR, REMOTE_TERMINAL_CLOSE, REMOTE_TERMINAL_CLOSED,
    REMOTE_TERMINAL_CREATE, REMOTE_TERMINAL_CREATED, REMOTE_TERMINAL_INPUT,
    REMOTE_TERMINAL_INPUT_ACK, REMOTE_TERMINAL_LIST, REMOTE_TERMINAL_OUTPUT, REMOTE_TERMINAL_RESIZE,
};
use codux_remote_transport::RemoteTransport;
use codux_runtime_live::terminal_pty::{TerminalManager, TerminalPtyConfig};
use codux_runtime_core::terminal::terminal_snapshot_payload;
use codux_terminal_core::TerminalEvent;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};

type TransportSlot = Arc<Mutex<Option<Arc<dyn RemoteTransport>>>>;

/// True for the terminal messages this module handles.
pub fn is_terminal_kind(kind: &str) -> bool {
    matches!(
        kind,
        REMOTE_TERMINAL_LIST
            | REMOTE_TERMINAL_CREATE
            | REMOTE_TERMINAL_INPUT
            | REMOTE_TERMINAL_RESIZE
            | REMOTE_TERMINAL_CLOSE
    )
}

fn send(transport: &TransportSlot, device_id: Option<&str>, envelope: Value, terminal_stream: bool) {
    let Ok(bytes) = serde_json::to_vec(&envelope) else {
        return;
    };
    if let Ok(guard) = transport.lock() {
        if let Some(t) = guard.as_ref() {
            if terminal_stream {
                t.send_terminal(bytes, device_id);
            } else {
                t.send(bytes, device_id);
            }
        }
    }
}

fn reply(transport: &TransportSlot, device_id: Option<&str>, kind: &str, payload: Value) {
    let mut envelope = json!({ "type": kind, "payload": payload });
    if let Some(device_id) = device_id {
        envelope["deviceId"] = json!(device_id);
    }
    send(transport, device_id, envelope, false);
}

fn list_payload(driver: &TerminalManager) -> Value {
    let terminals = driver
        .list()
        .into_iter()
        .map(|snapshot| terminal_snapshot_payload(snapshot, "headless"))
        .collect::<Vec<_>>();
    json!({ "terminals": terminals })
}

/// Handle one terminal envelope, sending any responses directly through the
/// transport (terminal output streams asynchronously, so this domain is
/// imperative rather than single-reply).
pub fn handle_terminal(
    driver: &Arc<TerminalManager>,
    transport: &TransportSlot,
    device_id: Option<&str>,
    kind: &str,
    payload: &Value,
) {
    let session_id = || payload.get("sessionId").and_then(Value::as_str);
    match kind {
        REMOTE_TERMINAL_LIST => {
            reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
        }
        REMOTE_TERMINAL_CREATE => {
            let project_id = payload
                .get("projectId")
                .and_then(Value::as_str)
                .map(str::to_string);
            let config = TerminalPtyConfig {
                cwd: payload
                    .get("cwd")
                    .or_else(|| payload.get("projectPath"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                command: payload.get("command").and_then(Value::as_str).map(str::to_string),
                cols: payload.get("cols").and_then(Value::as_u64).map(|v| v as u16),
                rows: payload.get("rows").and_then(Value::as_u64).map(|v| v as u16),
                project_id: project_id.clone(),
                worktree_id: payload
                    .get("worktreeId")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| project_id.clone()),
                project_name: payload
                    .get("projectName")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                terminal_id: payload
                    .get("terminalId")
                    .or_else(|| payload.get("sessionId"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                slot_id: payload.get("slotId").and_then(Value::as_str).map(str::to_string),
                session_key: payload
                    .get("sessionKey")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                title: payload.get("title").and_then(Value::as_str).map(str::to_string),
                tool: payload.get("tool").and_then(Value::as_str).map(str::to_string),
                support_dir: Some(crate::projects::agent_data_dir()),
                runtime_root: Some(codux_runtime_live::runtime_paths::runtime_root_dir()),
                tool_permissions_file: Some(
                    crate::projects::agent_data_dir().join("tool_permissions.json"),
                ),
                memory_workspace_root: Some(crate::projects::agent_data_dir().join("memory")),
                memory_prompt_file: Some(
                    crate::projects::agent_data_dir()
                        .join("memory")
                        .join("AI_MEMORY.md"),
                ),
                memory_index_file: Some(
                    crate::projects::agent_data_dir()
                        .join("memory")
                        .join("memory-index.json"),
                ),
                ..Default::default()
            };
            // Stream this session's output back to the controller.
            let seq = Arc::new(AtomicI64::new(0));
            let driver_for_emit = Arc::clone(driver);
            let transport_for_emit = Arc::clone(transport);
            let device = device_id.map(str::to_string);
            let emit = move |event: TerminalEvent| {
                if let TerminalEvent::Output { session_id, bytes, .. } = event {
                    let data = String::from_utf8_lossy(&bytes).to_string();
                    let next = seq.fetch_add(1, Ordering::SeqCst) + 1;
                    let screen = driver_for_emit.snapshot(&session_id).ok();
                    let buffer_len = driver_for_emit
                        .buffer_characters(&session_id)
                        .ok()
                        .unwrap_or(0);
                    let mut envelope = json!({
                        "type": REMOTE_TERMINAL_OUTPUT,
                        "sessionId": session_id,
                        "payload": terminal_live_output_payload(data, buffer_len, next, screen),
                    });
                    if let Some(device) = device.as_deref() {
                        envelope["deviceId"] = json!(device);
                    }
                    send(&transport_for_emit, device.as_deref(), envelope, true);
                }
            };
            match driver.create(config, emit) {
                Ok(session_id) => {
                    reply(
                        transport,
                        device_id,
                        REMOTE_TERMINAL_CREATED,
                        json!({ "sessionId": session_id }),
                    );
                    reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
                }
                Err(error) => reply(
                    transport,
                    device_id,
                    REMOTE_ERROR,
                    json!({ "message": error.to_string() }),
                ),
            }
        }
        REMOTE_TERMINAL_INPUT => {
            let data = payload.get("data").and_then(Value::as_str).unwrap_or("");
            match session_id() {
                Some(id) => match driver.write(id, data.as_bytes()) {
                    Ok(()) => {
                        reply(
                            transport,
                            device_id,
                            REMOTE_TERMINAL_INPUT_ACK,
                            json!({ "sessionId": id }),
                        );
                    }
                    Err(error) => {
                        reply(
                            transport,
                            device_id,
                            REMOTE_ERROR,
                            json!({ "message": error.to_string() }),
                        );
                    }
                },
                None => reply(transport, device_id, REMOTE_ERROR, json!({ "message": "sessionId is required." })),
            }
        }
        REMOTE_TERMINAL_RESIZE => {
            let cols = payload.get("cols").and_then(Value::as_u64).unwrap_or(80) as u16;
            let rows = payload.get("rows").and_then(Value::as_u64).unwrap_or(24) as u16;
            if let Some(id) = session_id() {
                let _ = driver.resize(id, cols, rows);
            }
        }
        REMOTE_TERMINAL_CLOSE => {
            if let Some(id) = session_id() {
                let _ = driver.kill(id);
                reply(transport, device_id, REMOTE_TERMINAL_CLOSED, json!({ "sessionId": id }));
                reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
            }
        }
        _ => {}
    }
}
