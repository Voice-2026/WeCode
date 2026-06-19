//! Terminal domain for the headless host: spawn real PTYs with `LocalPtyDriver`
//! and stream their output to the controller as `terminal.output`, matching the
//! protocol the desktop/mobile controllers already render (data + screenData +
//! outputSeq + bufferLength via codux_protocol::terminal_live_output_payload).

use codux_protocol::{
    terminal_live_output_payload, REMOTE_ERROR, REMOTE_TERMINAL_CLOSE, REMOTE_TERMINAL_CLOSED,
    REMOTE_TERMINAL_CREATE, REMOTE_TERMINAL_CREATED, REMOTE_TERMINAL_INPUT,
    REMOTE_TERMINAL_INPUT_ACK, REMOTE_TERMINAL_LIST, REMOTE_TERMINAL_OUTPUT, REMOTE_TERMINAL_RESIZE,
};
use codux_remote_transport::RemoteTransport;
use codux_runtime_core::terminal::terminal_snapshot_payload;
use codux_terminal_core::{TerminalDriver, TerminalEvent, TerminalLaunchConfig, TerminalSessionHandle};
use codux_terminal_pty::LocalPtyDriver;
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

fn list_payload(driver: &LocalPtyDriver) -> Value {
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
    driver: &Arc<LocalPtyDriver>,
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
            let config = TerminalLaunchConfig {
                cwd: payload
                    .get("cwd")
                    .or_else(|| payload.get("projectPath"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                command: payload.get("command").and_then(Value::as_str).map(str::to_string),
                cols: payload.get("cols").and_then(Value::as_u64).map(|v| v as u16),
                rows: payload.get("rows").and_then(Value::as_u64).map(|v| v as u16),
                project_id: payload.get("projectId").and_then(Value::as_str).map(str::to_string),
                title: payload.get("title").and_then(Value::as_str).map(str::to_string),
                ..Default::default()
            };
            // Stream this session's output back to the controller.
            let seq = Arc::new(AtomicI64::new(0));
            let driver_for_emit = Arc::clone(driver);
            let transport_for_emit = Arc::clone(transport);
            let device = device_id.map(str::to_string);
            let emit = Box::new(move |event: TerminalEvent| -> bool {
                if let TerminalEvent::Output { session_id, bytes, .. } = event {
                    let data = String::from_utf8_lossy(&bytes).to_string();
                    let next = seq.fetch_add(1, Ordering::SeqCst) + 1;
                    let (screen, buffer_len) = driver_for_emit
                        .session(&session_id)
                        .ok()
                        .map(|s| (Some(s.snapshot()), s.buffer_characters()))
                        .unwrap_or((None, 0));
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
                true
            });
            match driver.create(config, emit) {
                Ok(session) => {
                    reply(
                        transport,
                        device_id,
                        REMOTE_TERMINAL_CREATED,
                        json!({ "sessionId": session.id() }),
                    );
                    reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
                }
                Err(error) => reply(transport, device_id, REMOTE_ERROR, json!({ "message": error })),
            }
        }
        REMOTE_TERMINAL_INPUT => {
            let data = payload.get("data").and_then(Value::as_str).unwrap_or("");
            match session_id() {
                Some(id) => match driver.session(id) {
                    Ok(session) => {
                        if let Err(error) = session.write(data.as_bytes()) {
                            reply(transport, device_id, REMOTE_ERROR, json!({ "message": error }));
                        } else {
                            reply(
                                transport,
                                device_id,
                                REMOTE_TERMINAL_INPUT_ACK,
                                json!({ "sessionId": id }),
                            );
                        }
                    }
                    Err(error) => reply(transport, device_id, REMOTE_ERROR, json!({ "message": error })),
                },
                None => reply(transport, device_id, REMOTE_ERROR, json!({ "message": "sessionId is required." })),
            }
        }
        REMOTE_TERMINAL_RESIZE => {
            let cols = payload.get("cols").and_then(Value::as_u64).unwrap_or(80) as u16;
            let rows = payload.get("rows").and_then(Value::as_u64).unwrap_or(24) as u16;
            if let Some(id) = session_id() {
                if let Ok(session) = driver.session(id) {
                    let _ = session.resize(cols, rows);
                }
            }
        }
        REMOTE_TERMINAL_CLOSE => {
            if let Some(id) = session_id() {
                if let Ok(session) = driver.session(id) {
                    let _ = session.kill();
                }
                let _ = driver.remove(id);
                reply(transport, device_id, REMOTE_TERMINAL_CLOSED, json!({ "sessionId": id }));
                reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
            }
        }
        _ => {}
    }
}
