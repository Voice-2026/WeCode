//! Terminal domain for the headless host: spawn real PTYs with the same
//! `TerminalManager` the desktop host uses, so AI runtime tracking and terminal
//! protocol behavior stay aligned.
//!
//! Multi-client: several devices can watch the same terminal at once. The
//! viewer set, baseline catch-up, and viewport lease all reuse the shared crate
//! pieces (`RemoteTerminalSubscriptions`, `snapshot_tail` + `terminal_buffer_payloads`,
//! the `TerminalManager` lease + viewport-owner resolver) rather than a private
//! copy of the desktop host's batching/baseline machinery.

use codux_protocol::{
    REMOTE_ERROR, REMOTE_RESOURCE_SUBSCRIBE, REMOTE_RESOURCE_TERMINALS, REMOTE_RESOURCE_UNSUBSCRIBE,
    REMOTE_TERMINAL_BUFFER_MAX_CHARS, REMOTE_TERMINAL_CLOSE, REMOTE_TERMINAL_CLOSED,
    REMOTE_TERMINAL_CREATE, REMOTE_TERMINAL_CREATED, REMOTE_TERMINAL_INPUT,
    REMOTE_TERMINAL_INPUT_ACK, REMOTE_TERMINAL_LIST, REMOTE_TERMINAL_OUTPUT,
    REMOTE_TERMINAL_OUTPUT_ACK, REMOTE_TERMINAL_RESIZE, REMOTE_TERMINAL_SIGNAL,
    REMOTE_TERMINAL_VIEWPORT_CLAIM, REMOTE_TERMINAL_VIEWPORT_RELEASE,
    REMOTE_TERMINAL_VIEWPORT_RESIZE, REMOTE_TERMINAL_VIEWPORT_SCROLL,
    REMOTE_TERMINAL_VIEWPORT_SCROLLED, REMOTE_TERMINAL_VIEWPORT_STATE,
    RemoteTerminalBufferWindow, RemoteTerminalSubscriptions,
    terminal_buffer_payloads, terminal_live_output_payload,
};
use codux_remote_transport::RemoteTransport;
use codux_runtime_core::terminal::terminal_snapshot_payload;
use codux_runtime_live::terminal_pty::{
    TerminalManager, TerminalPtyConfig, terminal_viewport_remote_owner,
};
use codux_terminal_core::TerminalEvent;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type TransportSlot = Arc<Mutex<Option<Arc<dyn RemoteTransport>>>>;

/// Shared multi-client state for headless terminals: which devices view each
/// session (`subscriptions`) and the per-session output sequence (`output_seq`,
/// shared across viewers so every device sees the same `outputSeq`).
#[derive(Clone, Default)]
pub struct TerminalFanout {
    subscriptions: Arc<RemoteTerminalSubscriptions>,
    output_seq: Arc<Mutex<HashMap<String, i64>>>,
}

impl TerminalFanout {
    pub fn new() -> Self {
        Self::default()
    }

    /// The shared viewer registry, e.g. for the viewport-owner resolver.
    pub fn subscriptions(&self) -> Arc<RemoteTerminalSubscriptions> {
        Arc::clone(&self.subscriptions)
    }

    fn next_seq(&self, session_id: &str) -> i64 {
        let mut map = self.output_seq.lock().unwrap_or_else(|err| err.into_inner());
        let next = map.get(session_id).copied().unwrap_or(0) + 1;
        map.insert(session_id.to_string(), next);
        next
    }

    fn current_seq(&self, session_id: &str) -> i64 {
        self.output_seq
            .lock()
            .map(|map| map.get(session_id).copied().unwrap_or(0))
            .unwrap_or(0)
    }

    fn add_viewer(&self, session_id: &str, device_id: &str) {
        self.subscriptions.add_session_viewer(session_id, device_id);
    }

    fn remove_viewer(&self, session_id: &str, device_id: &str) {
        self.subscriptions
            .remove_session_viewer(session_id, device_id);
    }

    fn viewers(&self, session_id: &str) -> Vec<String> {
        self.subscriptions
            .viewers_for_session(session_id, None)
            .into_iter()
            .collect()
    }
}

/// True for the terminal + terminal-subscription messages this module handles.
pub fn is_terminal_kind(kind: &str) -> bool {
    matches!(
        kind,
        REMOTE_TERMINAL_LIST
            | REMOTE_TERMINAL_CREATE
            | REMOTE_TERMINAL_INPUT
            | REMOTE_TERMINAL_SIGNAL
            | REMOTE_TERMINAL_RESIZE
            | REMOTE_TERMINAL_CLOSE
            | REMOTE_TERMINAL_OUTPUT_ACK
            | REMOTE_TERMINAL_VIEWPORT_CLAIM
            | REMOTE_TERMINAL_VIEWPORT_RESIZE
            | REMOTE_TERMINAL_VIEWPORT_RELEASE
            | REMOTE_TERMINAL_VIEWPORT_SCROLL
            | REMOTE_RESOURCE_SUBSCRIBE
            | REMOTE_RESOURCE_UNSUBSCRIBE
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

/// Serialize one frame and fan it out: unicast to each viewer (the transport
/// routes by the device arg, not the envelope's `deviceId`), or broadcast when
/// no device has explicitly subscribed -- which preserves the original
/// single-device / no-device behavior.
fn fanout(transport: &TransportSlot, viewers: &[String], envelope: Value, terminal_stream: bool) {
    let Ok(bytes) = serde_json::to_vec(&envelope) else {
        return;
    };
    let Ok(guard) = transport.lock() else {
        return;
    };
    let Some(t) = guard.as_ref() else {
        return;
    };
    if viewers.is_empty() {
        if terminal_stream {
            t.send_terminal(bytes, None);
        } else {
            t.send(bytes, None);
        }
        return;
    }
    for device in viewers {
        let copy = bytes.clone();
        if terminal_stream {
            t.send_terminal(copy, Some(device));
        } else {
            t.send(copy, Some(device));
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

/// Send a session's catch-up baseline to a newly-subscribed device, reusing the
/// shared `snapshot_tail` + `terminal_buffer_payloads` helpers so the wire shape
/// matches the desktop host exactly.
fn send_terminal_baseline(
    driver: &TerminalManager,
    transport: &TransportSlot,
    fanout_state: &TerminalFanout,
    device_id: &str,
    session_id: &str,
    payload: &Value,
) {
    let max_chars = payload
        .get("maxChars")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(REMOTE_TERMINAL_BUFFER_MAX_CHARS);
    let chunk_chars = payload
        .get("chunkChars")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let request_id = payload
        .get("requestId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Ok((data, offset)) = driver.snapshot_tail(session_id, max_chars) else {
        return;
    };
    let total_characters = driver
        .buffer_characters(session_id)
        .unwrap_or_else(|_| offset + data.chars().count());
    let screen_data = driver
        .screen_snapshot(session_id)
        .ok()
        .map(|snapshot| snapshot.data)
        .filter(|data| !data.is_empty());
    let output_seq = fanout_state.current_seq(session_id);
    let window = RemoteTerminalBufferWindow {
        data,
        screen_data,
        offset,
        total_characters,
        truncated: false,
        output_seq: Some(output_seq),
        request_id,
        tail: true,
        has_previous: offset > 0,
    };
    for payload in terminal_buffer_payloads(&window, output_seq, chunk_chars) {
        let mut envelope =
            json!({ "type": REMOTE_TERMINAL_OUTPUT, "sessionId": session_id, "payload": payload });
        envelope["deviceId"] = json!(device_id);
        send(transport, Some(device_id), envelope, true);
    }
}

/// Handle one terminal envelope, sending any responses directly through the
/// transport (terminal output streams asynchronously, so this domain is
/// imperative rather than single-reply).
pub fn handle_terminal(
    driver: &Arc<TerminalManager>,
    transport: &TransportSlot,
    fanout_state: &TerminalFanout,
    device_id: Option<&str>,
    kind: &str,
    envelope: &Value,
    payload: &Value,
) {
    // Subscribe/viewport/ack carry the session id at the envelope top level;
    // create/input/resize/close carry it in the payload. Accept either.
    let session_id = || {
        payload
            .get("sessionId")
            .and_then(Value::as_str)
            .or_else(|| envelope.get("sessionId").and_then(Value::as_str))
    };
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
            // Stream this session's output to ALL of its viewers (fan-out), and
            // forward viewport-state changes (lease claim/handoff) too.
            let driver_for_emit = Arc::clone(driver);
            let transport_for_emit = Arc::clone(transport);
            let fanout_for_emit = fanout_state.clone();
            let emit = move |event: TerminalEvent| match event {
                TerminalEvent::Output { session_id, bytes, .. } => {
                    let data = String::from_utf8_lossy(&bytes).to_string();
                    let next = fanout_for_emit.next_seq(&session_id);
                    // Live output is a pure byte stream — no per-output screen
                    // keyframe. Replaying a whole-screen keyframe on top of the
                    // viewer's own scrollback duplicated the screen (badly on
                    // resize); the snapshot was also serialized on every chunk.
                    let buffer_len = driver_for_emit
                        .buffer_characters(&session_id)
                        .ok()
                        .unwrap_or(0);
                    let envelope = json!({
                        "type": REMOTE_TERMINAL_OUTPUT,
                        "sessionId": session_id,
                        "payload": terminal_live_output_payload(data, buffer_len, next),
                    });
                    fanout(
                        &transport_for_emit,
                        &fanout_for_emit.viewers(&session_id),
                        envelope,
                        true,
                    );
                }
                TerminalEvent::Viewport {
                    session_id,
                    owner,
                    cols,
                    rows,
                    generation,
                } => {
                    let envelope = json!({
                        "type": REMOTE_TERMINAL_VIEWPORT_STATE,
                        "sessionId": session_id,
                        "payload": {
                            "owner": owner,
                            "cols": cols,
                            "rows": rows,
                            "generation": generation,
                        },
                    });
                    fanout(
                        &transport_for_emit,
                        &fanout_for_emit.viewers(&session_id),
                        envelope,
                        false,
                    );
                }
                _ => {}
            };
            match driver.create(config, emit) {
                Ok(session_id) => {
                    if let Some(device_id) = device_id {
                        fanout_state.add_viewer(&session_id, device_id);
                    }
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
                fanout_state.subscriptions.remove_session(id);
                reply(transport, device_id, REMOTE_TERMINAL_CLOSED, json!({ "sessionId": id }));
                reply(transport, device_id, REMOTE_TERMINAL_LIST, list_payload(driver));
            }
        }
        REMOTE_RESOURCE_SUBSCRIBE => {
            if payload.get("resource").and_then(Value::as_str) != Some(REMOTE_RESOURCE_TERMINALS) {
                return;
            }
            let baseline = payload
                .get("baseline")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                fanout_state.add_viewer(id, device_id);
                if baseline {
                    send_terminal_baseline(driver, transport, fanout_state, device_id, id, payload);
                }
            }
        }
        REMOTE_RESOURCE_UNSUBSCRIBE => {
            if payload.get("resource").and_then(Value::as_str) != Some(REMOTE_RESOURCE_TERMINALS) {
                return;
            }
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                fanout_state.remove_viewer(id, device_id);
            }
        }
        REMOTE_TERMINAL_VIEWPORT_CLAIM => {
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                fanout_state.add_viewer(id, device_id);
                let owner = terminal_viewport_remote_owner(device_id);
                let _ = driver.claim_viewport(id, &owner);
            }
        }
        REMOTE_TERMINAL_VIEWPORT_RESIZE => {
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                fanout_state.add_viewer(id, device_id);
                let owner = terminal_viewport_remote_owner(device_id);
                let cols = payload.get("cols").and_then(Value::as_u64).unwrap_or(80) as u16;
                let rows = payload.get("rows").and_then(Value::as_u64).unwrap_or(24) as u16;
                let _ = driver.resize_viewport(id, &owner, cols, rows);
            }
        }
        REMOTE_TERMINAL_VIEWPORT_RELEASE => {
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                let owner = terminal_viewport_remote_owner(device_id);
                let _ = driver.release_viewport(id, &owner);
            }
        }
        REMOTE_TERMINAL_OUTPUT_ACK => {
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                let owner = terminal_viewport_remote_owner(device_id);
                driver.touch_viewport_lease(id, &owner);
            }
        }
        REMOTE_TERMINAL_SIGNAL => {
            // Forward a control signal to the PTY — same as the desktop host: an
            // interrupt is Ctrl-C (0x03), escape is ESC (0x1b).
            if let Some(id) = session_id() {
                let byte: &[u8] = match payload.get("signal").and_then(Value::as_str) {
                    Some("interrupt") => &[0x03],
                    Some("escape") => &[0x1b],
                    _ => &[],
                };
                if !byte.is_empty() {
                    let _ = driver.write(id, byte);
                }
            }
        }
        REMOTE_TERMINAL_VIEWPORT_SCROLL => {
            // Scroll the host's authoritative screen and reply with the viewport
            // snapshot — mirrors the desktop host so phones get scrollback on an
            // agent terminal too. `displayOffset` = a precise viewport fetch;
            // `toBottom` = jump to live; otherwise scroll by `lines` (0 = sync).
            if let (Some(id), Some(device_id)) = (session_id(), device_id) {
                let owner = terminal_viewport_remote_owner(device_id);
                driver.touch_viewport_lease(id, &owner);
                let viewport_request_id = payload.get("viewportRequestId").and_then(|value| {
                    value
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| value.as_u64().map(|number| number.to_string()))
                });
                let max_lines =
                    payload.get("maxLines").and_then(Value::as_u64).unwrap_or(0) as usize;
                let overscan_rows =
                    payload.get("overscanRows").and_then(Value::as_u64).unwrap_or(0) as usize;
                let snapshot = if let Some(display_offset) = payload
                    .get("displayOffset")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                {
                    driver.remote_viewport_snapshot(id, display_offset, overscan_rows, max_lines)
                } else if payload.get("toBottom").and_then(Value::as_bool).unwrap_or(false) {
                    driver.scroll_screen_to_bottom(id)
                } else {
                    let lines = payload
                        .get("lines")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                        .clamp(i32::MIN as i64, i32::MAX as i64)
                        as i32;
                    driver.scroll_screen_lines(id, lines)
                };
                if let Ok(snapshot) = snapshot {
                    let mut scrolled = json!({
                        "sessionId": id,
                        "displayOffset": snapshot.display_offset,
                        "totalLines": snapshot.total_lines,
                        "cols": snapshot.cols,
                        "rows": snapshot.rows,
                        "marginRows": snapshot.margin_rows,
                        "marginRowsBelow": snapshot.margin_rows_below,
                        "screenData": snapshot.data,
                    });
                    if let Some(request_id) = viewport_request_id {
                        scrolled["viewportRequestId"] = Value::String(request_id);
                    }
                    reply(
                        transport,
                        Some(device_id),
                        REMOTE_TERMINAL_VIEWPORT_SCROLLED,
                        scrolled,
                    );
                }
            }
        }
        _ => {}
    }
}
