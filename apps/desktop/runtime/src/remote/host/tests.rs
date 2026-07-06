use super::*;
use crate::remote::transport::RemoteTransport;
use crate::remote::types::RemoteOutgoingEnvelope;
use crate::terminal_layout::TerminalPaneSummary;
use async_trait::async_trait;
use codux_remote_transport::RemoteTransportKind;

fn temp_support_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("create temp support dir");
    dir
}

fn buffer_options(
    max_chars: usize,
    request_id: Option<&str>,
    tail: bool,
) -> TerminalBaselineOptions {
    TerminalBaselineOptions {
        max_chars,
        chunk_chars: None,
        request_id: request_id.map(ToOwned::to_owned),
        tail,
        viewport: None,
    }
}

fn viewport_buffer_options(
    max_chars: usize,
    request_id: Option<&str>,
    tail: bool,
    cols: u16,
    rows: u16,
) -> TerminalBaselineOptions {
    TerminalBaselineOptions {
        max_chars,
        chunk_chars: None,
        request_id: request_id.map(ToOwned::to_owned),
        tail,
        viewport: Some(BaselineViewport { cols, rows }),
    }
}

#[derive(Default)]
struct CapturingTransport {
    messages: Mutex<Vec<(Option<String>, Vec<u8>)>>,
}

impl CapturingTransport {
    fn take_messages(&self) -> Vec<(Option<String>, Vec<u8>)> {
        self.messages
            .lock()
            .map(|mut messages| messages.drain(..).collect())
            .unwrap_or_default()
    }

    fn wait_for_message<F>(&self, mut predicate: F) -> Option<(Option<String>, Vec<u8>)>
    where
        F: FnMut(&(Option<String>, Vec<u8>)) -> bool,
    {
        // Generous cap: slow replies (host-metrics sampling under full-suite
        // load) can exceed a few seconds; passing tests return early.
        for _ in 0..600 {
            for message in self.take_messages() {
                if predicate(&message) {
                    return Some(message);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        None
    }
}

#[async_trait]
impl RemoteTransport for CapturingTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::Iroh
    }

    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        if let Ok(mut messages) = self.messages.lock() {
            messages.push((device_id.map(str::to_string), data));
        }
        true
    }

    async fn shutdown(&self) {}
}

#[test]
fn host_transport_disconnect_clears_stale_transport_and_enters_reconnect() {
    let support_dir = temp_support_dir("codux-remote-host-reconnect");
    write_paired_remote_settings(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.connection_generation.store(7, Ordering::SeqCst);
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }

    let restart = runtime.prepare_transport_reconnect_after_disconnect(7);

    assert!(restart.is_some());
    let (_, restart_generation) = restart.expect("restart generation");
    assert_eq!(restart_generation, 8);
    assert_eq!(runtime.connection_generation.load(Ordering::SeqCst), 8);
    assert!(runtime.transport.lock().expect("transport lock").is_none());
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.status, "connecting");
    assert_eq!(
        snapshot.message,
        "Remote transport disconnected. Reconnecting..."
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn stale_host_transport_disconnect_does_not_clear_current_transport() {
    let support_dir = temp_support_dir("codux-remote-host-reconnect-stale");
    write_paired_remote_settings(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.connection_generation.store(8, Ordering::SeqCst);
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }

    let restart = runtime.prepare_transport_reconnect_after_disconnect(7);

    assert!(restart.is_none());
    assert_eq!(runtime.connection_generation.load(Ordering::SeqCst), 8);
    assert!(runtime.transport.lock().expect("transport lock").is_some());

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn pairing_preparation_restarts_host_transport() {
    let support_dir = temp_support_dir("codux-remote-host-pairing-restart");
    write_paired_remote_settings(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.connection_generation.store(11, Ordering::SeqCst);
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }

    let (transport, generation) = runtime
        .prepare_transport_for_pairing()
        .expect("prepare pairing transport");

    assert!(transport.is_some());
    assert_eq!(generation, 12);
    assert_eq!(runtime.connection_generation.load(Ordering::SeqCst), 12);
    assert!(runtime.transport.lock().expect("transport lock").is_none());
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.status, "connecting");
    assert_eq!(snapshot.message, "Connecting remote transport...");
    assert!(snapshot.pairing.is_none());

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn unauthorized_remote_message_gets_repair_response() {
    let support_dir = temp_support_dir("codux-remote-unauthorized-repair");
    write_paired_remote_settings(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }

    let raw = RemoteOutgoingEnvelope {
        kind: REMOTE_HOST_INFO.to_string(),
        device_id: Some("unknown-device".to_string()),
        session_id: None,
        seq: None,
        payload: json!({}),
    };
    runtime.clone().handle_transport_message(
        "unknown-device".to_string(),
        serde_json::to_vec(&raw).unwrap(),
    );

    let messages = transport.take_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0.as_deref(), Some("unknown-device"));
    let envelope: RemoteEnvelope =
        serde_json::from_slice(&messages[0].1).expect("unauthorized envelope");
    assert_eq!(envelope.kind, REMOTE_ERROR);
    assert_eq!(envelope.device_id.as_deref(), Some("unknown-device"));
    assert_eq!(envelope.payload["code"], "device_unauthorized");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn unauthorized_message_without_envelope_device_uses_transport_device_for_repair() {
    let support_dir = temp_support_dir("codux-remote-unauthorized-transport-device-repair");
    write_paired_remote_settings(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }

    let raw = RemoteOutgoingEnvelope {
        kind: REMOTE_HOST_INFO.to_string(),
        device_id: None,
        session_id: None,
        seq: None,
        payload: json!({}),
    };
    runtime.clone().handle_transport_message(
        "unknown-device".to_string(),
        serde_json::to_vec(&raw).unwrap(),
    );

    let messages = transport.take_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0.as_deref(), Some("unknown-device"));
    let envelope: RemoteEnvelope =
        serde_json::from_slice(&messages[0].1).expect("unauthorized envelope");
    assert_eq!(envelope.kind, REMOTE_ERROR);
    assert_eq!(envelope.device_id.as_deref(), Some("unknown-device"));
    assert_eq!(envelope.payload["code"], "device_unauthorized");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn host_metrics_request_replies_with_metrics_payload() {
    let support_dir = temp_support_dir("codux-remote-host-metrics");
    write_paired_remote_settings(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }

    let raw = RemoteOutgoingEnvelope {
        kind: REMOTE_HOST_METRICS.to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({}),
    };
    runtime.handle_transport_message("device-1".to_string(), serde_json::to_vec(&raw).unwrap());

    let (_, bytes) = transport
        .wait_for_message(|message| {
            serde_json::from_slice::<RemoteEnvelope>(&message.1)
                .map(|envelope| envelope.kind == REMOTE_HOST_METRICS)
                .unwrap_or(false)
        })
        .expect("host metrics reply");
    let envelope: RemoteEnvelope = serde_json::from_slice(&bytes).expect("metrics envelope");
    assert_eq!(envelope.kind, REMOTE_HOST_METRICS);
    assert_eq!(envelope.device_id.as_deref(), Some("device-1"));
    let metrics: codux_protocol::RemoteHostMetrics =
        serde_json::from_value(envelope.payload).expect("metrics payload");
    assert!(metrics.sampled_at_millis > 0);
    assert!(metrics.processes.len() <= 30);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn web_tunnel_requires_paired_device_token() {
    let support_dir = temp_support_dir("codux-remote-web-tunnel-token");
    write_paired_remote_settings(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());

    assert!(
        runtime
            .authorize_web_tunnel_tcp_connect(WebTunnelTcpConnectRequest {
                device_id: "device-1".to_string(),
                device_token: "device-token-1".to_string(),
                host: "localhost".to_string(),
                port: 5173,
            })
            .is_ok()
    );
    assert!(
        runtime
            .authorize_web_tunnel_tcp_connect(WebTunnelTcpConnectRequest {
                device_id: "device-1".to_string(),
                device_token: "wrong-token".to_string(),
                host: "localhost".to_string(),
                port: 5173,
            })
            .is_err()
    );
    assert!(
        runtime
            .authorize_web_tunnel_tcp_connect(WebTunnelTcpConnectRequest {
                device_id: "unknown-device".to_string(),
                device_token: "device-token-1".to_string(),
                host: "localhost".to_string(),
                port: 5173,
            })
            .is_err()
    );

    fs::remove_dir_all(support_dir).ok();
}

fn write_paired_remote_settings(support_dir: &Path) {
    fs::write(
        support_dir.join("settings.json"),
        serde_json::to_string_pretty(&json!({
            "remote": {
                "isEnabled": true,
                "relayUrl": "http://relay.example",
                "hostID": "host-1",
                "cachedDevices": [
                    {
                        "id": "device-1",
                        "token": "device-token-1",
                        "hostId": "host-1",
                        "name": "Phone"
                    }
                ]
            }
        }))
        .expect("serialize settings"),
    )
    .expect("write settings");
}

fn write_two_project_state(support_dir: &Path) -> (PathBuf, PathBuf) {
    let project_a = support_dir.join("project-a");
    let project_b = support_dir.join("project-b");
    fs::create_dir_all(&project_a).expect("create project a");
    fs::create_dir_all(&project_b).expect("create project b");
    fs::write(
        support_dir.join("state.json"),
        serde_json::to_string_pretty(&json!({
            "projects": [
                {"id": "project-a", "name": "Project A", "path": project_a.to_string_lossy()},
                {"id": "project-b", "name": "Project B", "path": project_b.to_string_lossy()}
            ],
            "worktrees": [
                {
                    "id": "worktree-b",
                    "projectId": "project-b",
                    "name": "Task B",
                    "branch": "task-b",
                    "path": project_b.to_string_lossy(),
                    "status": "active",
                    "isDefault": true,
                    "createdAt": 1,
                    "updatedAt": 1
                }
            ],
            "selectedProjectId": "project-a",
            "selectedWorktreeIdByProject": {
                "project-b": "worktree-b"
            }
        }))
        .expect("serialize state"),
    )
    .expect("write state");
    (project_a, project_b)
}

#[test]
fn remote_project_select_keeps_desktop_selected_project() {
    let support_dir = temp_support_dir("codux-remote-scope-select");
    write_two_project_state(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));

    runtime.handle_project_select(&RemoteEnvelope {
        kind: "project.select".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "projectId": "project-b" }),
    });

    let state = fs::read_to_string(support_dir.join("state.json")).expect("read state");
    let state: Value = serde_json::from_str(&state).expect("parse state");
    assert_eq!(state["selectedProjectId"], "project-a");
    assert_eq!(
        runtime.remote_project_scope_id(Some("device-1")).as_deref(),
        Some("project-b")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn secure_project_select_keeps_decrypted_device_id_for_scope_and_replies() {
    let support_dir = temp_support_dir("codux-remote-secure-scope-select");
    write_paired_remote_settings(&support_dir);
    write_two_project_state(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let encrypted = {
        let mut send_seq = HashMap::new();
        runtime
            .service()
            .outgoing_transport_text(
                "project.select",
                Some("device-1"),
                None,
                json!({ "projectId": "project-b" }),
                &mut send_seq,
            )
            .expect("secure envelope")
            .into_bytes()
    };

    Arc::clone(&runtime).handle_transport_message("relay-device".to_string(), encrypted);

    assert_eq!(
        runtime.remote_project_scope_id(Some("device-1")).as_deref(),
        Some("project-b")
    );
    assert_eq!(runtime.remote_project_scope_id(Some("relay-device")), None);
    let replies = transport.take_messages();
    assert!(
        replies
            .iter()
            .any(|(device_id, _)| device_id.as_deref() == Some("device-1")),
        "expected reply to decrypted device id"
    );
    assert!(
        replies
            .iter()
            .all(|(device_id, _)| device_id.as_deref() != Some("relay-device")),
        "must not reply to transport device id"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn transport_ping_runtime_fallback_replies_plain_pong() {
    let support_dir = temp_support_dir("codux-remote-transport-ping-pong");
    write_paired_remote_settings(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }

    Arc::clone(&runtime).handle_transport_message(
        "device-1".to_string(),
        json!({
            "type": REMOTE_TRANSPORT_PING,
            "deviceId": "device-1",
            "payload": { "id": "ping-1" },
        })
        .to_string()
        .into_bytes(),
    );

    let messages = transport.take_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0.as_deref(), Some("device-1"));
    let reply: Value = serde_json::from_slice(&messages[0].1).expect("plain pong json");
    assert_eq!(
        reply.get("type").and_then(Value::as_str),
        Some(REMOTE_TRANSPORT_PONG)
    );
    assert_eq!(
        reply.get("deviceId").and_then(Value::as_str),
        Some("device-1")
    );
    assert_eq!(reply["payload"]["id"], "ping-1");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn viewport_events_do_not_broadcast_terminal_list() {
    let support_dir = temp_support_dir("codux-remote-viewport-no-terminal-list");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    transport.take_messages();
    runtime.handle_terminal_event(TerminalEvent::Viewport {
        session_id: session_id.clone(),
        owner: "desktop".to_string(),
        cols: 100,
        rows: 32,
        generation: 1,
    });

    let mut kinds = Vec::new();
    for (_, data) in transport.take_messages() {
        let text = String::from_utf8(data).expect("utf8 transport");
        let envelope = runtime
            .service()
            .parse_incoming_envelope(&text)
            .expect("parse outgoing envelope");
        kinds.push(envelope.kind);
    }

    assert_eq!(kinds, vec!["terminal.viewport.state"]);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_project_list_reports_device_selected_project_scope() {
    let support_dir = temp_support_dir("codux-remote-project-list-scope");
    write_two_project_state(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.set_remote_project_scope(Some("device-1"), "project-b");

    let payload = runtime.remote_project_list_payload(Some("device-1"));

    assert_eq!(payload["selectedProjectId"], "project-b");
    assert!(payload["selectedWorktreeId"].is_null());
    assert_eq!(
        payload["projects"]
            .as_array()
            .expect("projects")
            .iter()
            .filter_map(|project| project.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["project-a", "project-b"],
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_project_select_starts_project_terminal_on_host() {
    let support_dir = temp_support_dir("codux-remote-project-terminal");
    let (_, project_b) = write_two_project_state(&support_dir);
    let worktree_b_path = support_dir.join("project-b-worktree");
    fs::create_dir_all(&worktree_b_path).expect("create worktree b");
    let mut state: Value = serde_json::from_str(
        &fs::read_to_string(support_dir.join("state.json")).expect("read state"),
    )
    .expect("parse state");
    state["worktrees"][0]["path"] = json!(worktree_b_path.to_string_lossy());
    fs::write(
        support_dir.join("state.json"),
        serde_json::to_string_pretty(&state).expect("serialize state"),
    )
    .expect("write state");
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));

    runtime.handle_project_select(&RemoteEnvelope {
        kind: "project.select".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "projectId": "project-b", "worktreeId": "worktree-b" }),
    });

    let terminals = runtime.remote_terminals();
    let project_terminal = terminals
        .iter()
        .find(|terminal| terminal.get("projectId").and_then(Value::as_str) == Some("project-b"))
        .expect("project terminal");
    let session_id = project_terminal
        .get("id")
        .and_then(Value::as_str)
        .expect("session id");
    assert!(!session_id.trim().is_empty());

    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");
    let layout = TerminalLayoutService::new(support_dir.clone()).load(Some(&layout_key));
    assert_eq!(layout.top_panes.len(), 1);
    assert_eq!(layout.top_panes[0].terminal_id, session_id);
    let session = runtime
        .terminals
        .session(session_id)
        .expect("terminal session");
    let expected_session_key = format!("gpui:worktree-b:{session_id}");
    assert_eq!(session.info().project_id, "worktree-b");
    assert_eq!(
        session.info().cwd,
        worktree_b_path.to_string_lossy().as_ref()
    );
    assert_eq!(
        session.info().session_key.as_deref(),
        Some(expected_session_key.as_str())
    );
    assert_eq!(project_terminal["projectId"], "project-b");
    assert_eq!(project_terminal["worktreeId"], "worktree-b");
    assert_eq!(
        project_terminal["cwd"].as_str(),
        Some(worktree_b_path.to_string_lossy().as_ref())
    );
    assert_ne!(
        project_b.to_string_lossy(),
        worktree_b_path.to_string_lossy()
    );
    assert!(
        runtime
            .drain_events()
            .iter()
            .any(|event| matches!(event, RemoteHostEvent::TerminalLayoutChanged(_)))
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_worktree_select_is_device_scoped_and_does_not_mutate_desktop_selection() {
    let support_dir = temp_support_dir("codux-remote-worktree-device-scope");
    let (_, project_b) = write_two_project_state(&support_dir);
    let mut state: Value = serde_json::from_str(
        &fs::read_to_string(support_dir.join("state.json")).expect("read state"),
    )
    .expect("parse state");
    state["worktrees"]
        .as_array_mut()
        .expect("worktrees")
        .push(json!({
            "id": "worktree-c",
            "projectId": "project-b",
            "name": "Task C",
            "branch": "task-c",
            "path": project_b.to_string_lossy(),
            "status": "active",
            "isDefault": false,
            "createdAt": 2,
            "updatedAt": 2
        }));
    state["selectedWorktreeIdByProject"]["project-b"] = json!("worktree-c");
    fs::write(
        support_dir.join("state.json"),
        serde_json::to_string_pretty(&state).expect("serialize state"),
    )
    .expect("write state");
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));

    runtime.handle_worktree_select(&RemoteEnvelope {
        kind: "worktree.select".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "projectId": "project-b",
            "worktreeId": "worktree-b",
        }),
    });

    let state = fs::read_to_string(support_dir.join("state.json")).expect("read state");
    let state: Value = serde_json::from_str(&state).expect("parse state");
    assert_eq!(state["selectedProjectId"], "project-a");
    assert_eq!(
        state["selectedWorktreeIdByProject"]["project-b"],
        "worktree-c"
    );
    assert_eq!(
        runtime.remote_project_scope_id(Some("device-1")).as_deref(),
        Some("project-b")
    );
    assert!(runtime.remote_terminals().iter().any(|terminal| {
        terminal.get("projectId").and_then(Value::as_str) == Some("project-b")
            && terminal.get("worktreeId").and_then(Value::as_str) == Some("worktree-b")
    }));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_worktree_select_replaces_saved_terminal_with_wrong_cwd() {
    let support_dir = temp_support_dir("codux-remote-worktree-wrong-cwd");
    let (_, project_b) = write_two_project_state(&support_dir);
    let worktree_b_path = support_dir.join("project-b-worktree");
    fs::create_dir_all(&worktree_b_path).expect("create worktree b");
    let mut state: Value = serde_json::from_str(
        &fs::read_to_string(support_dir.join("state.json")).expect("read state"),
    )
    .expect("parse state");
    state["worktrees"][0]["path"] = json!(worktree_b_path.to_string_lossy());
    fs::write(
        support_dir.join("state.json"),
        serde_json::to_string_pretty(&state).expect("serialize state"),
    )
    .expect("write state");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let stale_terminal_id = "terminal-stale-worktree-b";
    terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf stale".to_string()),
                cwd: Some(project_b.to_string_lossy().to_string()),
                project_id: Some("project-b".to_string()),
                terminal_id: Some(stale_terminal_id.to_string()),
                session_key: Some(format!("gpui:project-b:{stale_terminal_id}")),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create stale terminal");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-b", "worktree-b"),
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Stale".to_string(),
                terminal_id: stale_terminal_id.to_string(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save stale layout");

    runtime.handle_worktree_select(&RemoteEnvelope {
        kind: "worktree.select".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "projectId": "project-b",
            "worktreeId": "worktree-b",
        }),
    });

    let session = runtime
        .terminals
        .session(stale_terminal_id)
        .expect("recreated terminal session");
    let info = session.info();
    let expected_session_key = format!("gpui:worktree-b:{stale_terminal_id}");
    assert_eq!(info.project_id, "worktree-b");
    assert_eq!(info.cwd, worktree_b_path.to_string_lossy().as_ref());
    assert_eq!(
        info.session_key.as_deref(),
        Some(expected_session_key.as_str())
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_project_subscriptions_keep_devices_scoped_to_their_projects() {
    let support_dir = temp_support_dir("codux-remote-terminal-subscriptions");
    let (project_a, project_b) = write_two_project_state(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_a = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf a".to_string()),
                cwd: Some(project_a.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal a");
    let session_b = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf b".to_string()),
                cwd: Some(project_b.to_string_lossy().to_string()),
                project_id: Some("project-b".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal b");

    runtime.handle_terminal_subscribe(&RemoteEnvelope {
        kind: "terminal.subscribe".to_string(),
        device_id: Some("mac".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "scope": "project", "projectId": "project-a" }),
    });
    runtime.handle_terminal_subscribe(&RemoteEnvelope {
        kind: "terminal.subscribe".to_string(),
        device_id: Some("windows".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "scope": "project", "projectId": "project-b" }),
    });

    let viewers_a = runtime.terminal_output_viewers(&session_a);
    let viewers_b = runtime.terminal_output_viewers(&session_b);

    assert!(viewers_a.contains("mac"));
    assert!(!viewers_a.contains("windows"));
    assert!(viewers_b.contains("windows"));
    assert!(!viewers_b.contains("mac"));

    runtime.handle_terminal_unsubscribe(&RemoteEnvelope {
        kind: "terminal.unsubscribe".to_string(),
        device_id: Some("mac".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "scope": "project", "projectId": "project-a" }),
    });

    let viewers_a = runtime.terminal_output_viewers(&session_a);
    assert!(!viewers_a.contains("mac"));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn project_select_replaces_previous_terminal_project_viewers() {
    let support_dir = temp_support_dir("codux-project-select-terminal-viewers");
    let (project_a, project_b) = write_two_project_state(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_a = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf a".to_string()),
                cwd: Some(project_a.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal a");
    let session_b = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf b".to_string()),
                cwd: Some(project_b.to_string_lossy().to_string()),
                project_id: Some("project-b".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal b");

    runtime.handle_project_select(&RemoteEnvelope {
        kind: "project.select".to_string(),
        device_id: Some("phone".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "projectId": "project-a" }),
    });
    assert!(
        runtime
            .terminal_output_viewers(&session_a)
            .contains("phone")
    );

    runtime.handle_project_select(&RemoteEnvelope {
        kind: "project.select".to_string(),
        device_id: Some("phone".to_string()),
        session_id: None,
        seq: None,
        payload: json!({ "projectId": "project-b" }),
    });

    assert!(
        !runtime
            .terminal_output_viewers(&session_a)
            .contains("phone")
    );
    assert!(
        runtime
            .terminal_output_viewers(&session_b)
            .contains("phone")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn resource_subscriptions_broadcast_project_scoped_git_status() {
    let support_dir = temp_support_dir("codux-remote-resource-subscriptions");
    let (project_a, _) = write_two_project_state(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }

    runtime.handle_resource_subscribe(&RemoteEnvelope {
        kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
        device_id: Some("phone-a".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "resource": REMOTE_RESOURCE_GIT_STATUS,
            "projectId": "project-a",
            "projectPath": project_a.to_string_lossy(),
        }),
    });
    transport.take_messages();

    runtime.handle_git_status(&RemoteEnvelope {
        kind: REMOTE_GIT_STATUS.to_string(),
        device_id: Some("phone-b".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "projectId": "project-a",
            "projectPath": project_a.to_string_lossy(),
        }),
    });

    let messages = transport.take_messages();
    let target_devices = messages
        .iter()
        .filter_map(|(device_id, data)| {
            let value: Value = serde_json::from_slice(data).ok()?;
            let kind = value.get("type").and_then(Value::as_str);
            (kind == Some(REMOTE_GIT_STATUS)).then(|| device_id.clone())
        })
        .collect::<Vec<_>>();

    assert!(target_devices.contains(&Some("phone-a".to_string())));
    assert!(target_devices.contains(&Some("phone-b".to_string())));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_resource_subscription_sends_tail_raw_baseline() {
    let support_dir = temp_support_dir("codux-remote-resource-terminal-tail-baseline");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf abcdef".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                terminal_id: Some("terminal-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-a", "project-a"),
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Main".to_string(),
                terminal_id: session_id.clone(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save layout");

    let mut baseline = None;
    for _ in 0..20 {
        runtime.handle_resource_subscribe(&RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("phone-a".to_string()),
            session_id: None,
            seq: None,
            payload: json!({
                "resource": REMOTE_RESOURCE_TERMINALS,
                "projectId": "project-a",
                "baseline": true,
                "maxChars": 3,
                "requestId": "request-1",
            }),
        });
        for (_, data) in transport.take_messages() {
            let value: Value = serde_json::from_slice(&data).expect("json");
            if value.get("type").and_then(Value::as_str) == Some(REMOTE_TERMINAL_OUTPUT)
                && value.get("sessionId").and_then(Value::as_str) == Some(&session_id)
            {
                baseline = value.get("payload").cloned();
                break;
            }
        }
        if baseline.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let baseline = baseline.expect("terminal baseline");
    // Baseline re-attach sends the newest `maxChars` (tail window); the mobile
    // consumer treats `tail: true` as a full keyframe replacement.
    assert_eq!(baseline["data"], "def");
    assert_eq!(baseline["offset"], 3);
    assert_eq!(baseline["tail"], true);
    assert_eq!(baseline["hasPrevious"], true);
    assert_eq!(baseline["truncated"], false);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn project_list_broadcast_preserves_per_device_project_scope() {
    let support_dir = temp_support_dir("codux-remote-project-list-subscriptions");
    write_two_project_state(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));

    runtime
        .resource_subscriptions
        .subscribe_envelope(&RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("phone-a".to_string()),
            session_id: None,
            seq: None,
            payload: json!({ "resource": REMOTE_RESOURCE_PROJECTS }),
        })
        .unwrap();
    runtime
        .resource_subscriptions
        .subscribe_envelope(&RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("phone-b".to_string()),
            session_id: None,
            seq: None,
            payload: json!({ "resource": REMOTE_RESOURCE_PROJECTS }),
        })
        .unwrap();
    runtime.set_remote_project_scope(Some("phone-a"), "project-a");
    runtime.set_remote_project_scope(Some("phone-b"), "project-b");

    assert_eq!(
        runtime.remote_project_list_payload(Some("phone-a"))["selectedProjectId"],
        "project-a"
    );
    assert_eq!(
        runtime.remote_project_list_payload(Some("phone-b"))["selectedProjectId"],
        "project-b"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_project_subscribe_with_baseline_sends_buffer_baseline() {
    let support_dir = temp_support_dir("codux-remote-terminal-subscribe-baseline");
    let (project_a, _) = write_two_project_state(&support_dir);
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf baseline-data".to_string()),
                cwd: Some(project_a.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    for _ in 0..20 {
        if terminals
            .snapshot(&session_id)
            .map(|snapshot| snapshot.contains("baseline-data"))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    runtime.handle_terminal_subscribe(&RemoteEnvelope {
        kind: "terminal.subscribe".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "scope": "project",
            "projectId": "project-a",
            "baseline": true,
            "maxChars": 64,
            "chunkChars": 16
        }),
    });

    let (_, data) = transport
        .wait_for_message(|(_, data)| {
            let Ok(text) = std::str::from_utf8(data) else {
                return false;
            };
            let Ok(envelope) = runtime.service().parse_incoming_envelope(text) else {
                return false;
            };
            envelope.kind == "terminal.output"
                && envelope.session_id.as_deref() == Some(&session_id)
        })
        .expect("baseline terminal output");
    let text = String::from_utf8(data).expect("utf8 transport");
    let envelope = runtime
        .service()
        .parse_incoming_envelope(&text)
        .expect("parse outgoing envelope");
    let baseline = envelope.payload;
    assert_eq!(baseline["buffer"], true);
    assert_eq!(baseline["offset"], 0);
    assert_eq!(baseline["requestId"].as_str().is_some(), true);
    assert!(
        baseline["data"]
            .as_str()
            .unwrap_or_default()
            .contains("baseline-data")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_plan_uses_device_project_scope_without_desktop_ui_selection() {
    let support_dir = temp_support_dir("codux-remote-scope-terminal");
    write_two_project_state(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.set_remote_project_scope(Some("device-1"), "project-b");
    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &layout_key,
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Mobile".to_string(),
                terminal_id: "terminal-b".to_string(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save layout");

    let plan = runtime
        .remote_terminal_plan_from_envelope(
            &RemoteEnvelope {
                kind: "terminal.buffer".to_string(),
                device_id: Some("device-1".to_string()),
                session_id: Some("terminal-b".to_string()),
                seq: None,
                payload: json!({}),
            },
            None,
            true,
        )
        .expect("terminal plan");

    assert_eq!(plan.scope.project_id, "project-b");
    assert_eq!(plan.scope.worktree_id, "worktree-b");
    assert_eq!(plan.config.project_id.as_deref(), Some("worktree-b"));
    assert_eq!(
        plan.config.session_key.as_deref(),
        Some("gpui:worktree-b:terminal-b")
    );
    assert_eq!(plan.config.terminal_id.as_deref(), Some("terminal-b"));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_list_indexes_all_project_worktree_layouts() {
    let support_dir = temp_support_dir("codux-remote-terminal-all-worktrees");
    write_two_project_state(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let default_session = terminals
        .create(
            TerminalPtyConfig {
                command: Some("printf default".to_string()),
                project_id: Some("project-b".to_string()),
                terminal_id: Some("terminal-default".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create default terminal");
    let worktree_session = terminals
        .create(
            TerminalPtyConfig {
                command: Some("printf worktree".to_string()),
                project_id: Some("project-b".to_string()),
                terminal_id: Some("terminal-worktree".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create worktree terminal");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-b", "project-b"),
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Default".to_string(),
                terminal_id: default_session.clone(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save default layout");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-b", "worktree-b"),
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Worktree".to_string(),
                terminal_id: worktree_session.clone(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save worktree layout");

    let terminal_worktrees = runtime
        .remote_terminals()
        .into_iter()
        .filter_map(|terminal| {
            Some((
                terminal.get("id")?.as_str()?.to_string(),
                terminal.get("worktreeId")?.as_str()?.to_string(),
            ))
        })
        .collect::<HashMap<_, _>>();

    assert_eq!(
        terminal_worktrees.get(&default_session).map(String::as_str),
        Some("project-b")
    );
    assert_eq!(
        terminal_worktrees
            .get(&worktree_session)
            .map(String::as_str),
        Some("worktree-b")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_list_reports_all_worktree_splits_under_root_project() {
    let support_dir = temp_support_dir("codux-remote-terminal-worktree-splits");
    write_two_project_state(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let sessions = (0..3)
        .map(|index| {
            terminals
                .create(
                    TerminalPtyConfig {
                        command: Some(format!("printf split-{index}")),
                        project_id: Some("worktree-b".to_string()),
                        terminal_id: Some(format!("terminal-worktree-{index}")),
                        ..Default::default()
                    },
                    |_| {},
                )
                .expect("create worktree terminal")
        })
        .collect::<Vec<_>>();
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-b", "project-b"),
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Stale".to_string(),
                terminal_id: sessions[0].clone(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save stale default layout");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &terminal_layout_storage_key("project-b", "worktree-b"),
            Vec::new(),
            sessions
                .iter()
                .enumerate()
                .map(|(index, session)| TerminalPaneSummary {
                    title: format!("Split {}", index + 1),
                    terminal_id: session.clone(),
                })
                .collect(),
            vec![0.33, 0.34, 0.33],
            0.24,
        )
        .expect("save worktree split layout");

    let mut worktree_terminals = runtime
        .remote_terminals()
        .into_iter()
        .filter(|terminal| terminal.get("projectId").and_then(Value::as_str) == Some("project-b"))
        .filter(|terminal| terminal.get("worktreeId").and_then(Value::as_str) == Some("worktree-b"))
        .collect::<Vec<_>>();
    worktree_terminals.sort_by_key(|terminal| {
        terminal
            .get("layoutOrder")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX)
    });

    assert_eq!(worktree_terminals.len(), 3);
    assert_eq!(
        worktree_terminals
            .iter()
            .filter_map(|terminal| terminal.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        sessions.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        worktree_terminals
            .iter()
            .filter_map(|terminal| terminal.get("layoutOrder").and_then(Value::as_u64))
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_create_plan_does_not_reuse_saved_layout_terminal() {
    let support_dir = temp_support_dir("codux-remote-create-new-terminal");
    write_two_project_state(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    runtime.set_remote_project_scope(Some("device-1"), "project-b");
    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &layout_key,
            Vec::new(),
            vec![TerminalPaneSummary {
                title: "Mobile".to_string(),
                terminal_id: "terminal-b".to_string(),
            }],
            vec![1.0],
            0.24,
        )
        .expect("save layout");

    let create_plan = runtime
        .remote_terminal_plan_from_envelope(
            &RemoteEnvelope {
                kind: "terminal.create".to_string(),
                device_id: Some("device-1".to_string()),
                session_id: None,
                seq: None,
                payload: json!({}),
            },
            None,
            false,
        )
        .expect("create terminal plan");
    assert_eq!(create_plan.config.terminal_id, None);
    assert_eq!(create_plan.config.project_id.as_deref(), Some("worktree-b"));
    let expected_worktree_path = support_dir.join("project-b");
    let expected_worktree_path = expected_worktree_path.to_string_lossy();
    assert_eq!(
        create_plan.config.cwd.as_deref(),
        Some(expected_worktree_path.as_ref())
    );

    let restore_plan = runtime
        .remote_terminal_plan_from_envelope(
            &RemoteEnvelope {
                kind: "terminal.buffer".to_string(),
                device_id: Some("device-1".to_string()),
                session_id: None,
                seq: None,
                payload: json!({}),
            },
            None,
            true,
        )
        .expect("restore terminal plan");
    assert_eq!(
        restore_plan.config.terminal_id.as_deref(),
        Some("terminal-b")
    );
    assert_eq!(
        restore_plan.config.project_id.as_deref(),
        Some("worktree-b")
    );
    assert_eq!(
        restore_plan.config.session_key.as_deref(),
        Some("gpui:worktree-b:terminal-b")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_layout_is_persisted_to_project_worktree_scope() {
    let support_dir = temp_support_dir("codux-remote-layout-persist");
    write_two_project_state(&support_dir);
    let runtime = RemoteHostRuntime::new(support_dir.clone());
    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");

    runtime.persist_remote_terminal_layout(&layout_key, "terminal-mobile-b", "Mobile");

    let layout = TerminalLayoutService::new(support_dir.clone()).load(Some(&layout_key));
    assert_eq!(layout.active_terminal_id, "");
    assert_eq!(layout.top_panes.len(), 1);
    assert_eq!(layout.top_panes[0].terminal_id, "terminal-mobile-b");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_create_emits_layout_changed_event() {
    let support_dir = temp_support_dir("codux-remote-create-layout-event");
    write_two_project_state(&support_dir);
    let runtime = Arc::new(RemoteHostRuntime::new(support_dir.clone()));
    runtime.set_remote_project_scope(Some("device-1"), "project-b");
    runtime.drain_events();

    runtime.handle_terminal_create(&RemoteEnvelope {
        kind: "terminal.create".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "projectId": "project-b",
            "worktreeId": "worktree-b",
        }),
    });

    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");
    let layout = TerminalLayoutService::new(support_dir.clone()).load(Some(&layout_key));
    // First terminal in an empty scope seeds the main split.
    assert_eq!(layout.top_panes.len(), 1);
    assert!(layout.tabs.is_empty());
    assert!(
        runtime
            .drain_events()
            .iter()
            .any(|event| matches!(event, RemoteHostEvent::TerminalLayoutChanged(_)))
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_close_removes_layout_entry_and_kills_last_terminal() {
    let support_dir = temp_support_dir("codux-remote-close-layout-entry");
    write_two_project_state(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let layout_key = terminal_layout_storage_key("project-b", "worktree-b");
    let session_a = terminals
        .create(
            TerminalPtyConfig {
                command: Some("printf a".to_string()),
                project_id: Some("worktree-b".to_string()),
                terminal_id: Some("terminal-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal a");
    let session_b = terminals
        .create(
            TerminalPtyConfig {
                command: Some("printf b".to_string()),
                project_id: Some("worktree-b".to_string()),
                terminal_id: Some("terminal-b".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal b");
    TerminalLayoutService::new(support_dir.clone())
        .save_from_gpui(
            &layout_key,
            Vec::new(),
            vec![
                TerminalPaneSummary {
                    title: "A".to_string(),
                    terminal_id: session_a.clone(),
                },
                TerminalPaneSummary {
                    title: "B".to_string(),
                    terminal_id: session_b.clone(),
                },
            ],
            vec![0.5, 0.5],
            0.24,
        )
        .expect("save layout");
    runtime.drain_events();

    runtime.handle_terminal_close(&RemoteEnvelope {
        kind: "terminal.close".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_b.clone()),
        seq: None,
        payload: json!({ "projectId": "project-b", "worktreeId": "worktree-b" }),
    });

    let layout = TerminalLayoutService::new(support_dir.clone()).load(Some(&layout_key));
    assert_eq!(layout.top_panes.len(), 1);
    assert_eq!(layout.top_panes[0].terminal_id, session_a);
    assert!(terminals.snapshot(&session_b).is_err());
    assert!(
        runtime
            .drain_events()
            .iter()
            .any(|event| matches!(event, RemoteHostEvent::TerminalLayoutChanged(_)))
    );

    runtime.handle_terminal_close(&RemoteEnvelope {
        kind: "terminal.close".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_a.clone()),
        seq: None,
        payload: json!({ "projectId": "project-b", "worktreeId": "worktree-b" }),
    });

    let layout = TerminalLayoutService::new(support_dir.clone()).load(Some(&layout_key));
    assert_eq!(layout.top_panes.len(), 1);
    assert_eq!(layout.top_panes[0].terminal_id, session_a);
    // Closing the last terminal now tears it down (previously it no-opped so
    // the dead pane lingered on both the desktop split and the pad tab).
    assert!(terminals.snapshot(&session_a).is_err());

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_output_sequence_is_session_scoped() {
    let support_dir = temp_support_dir("codux-remote-terminal-output-seq");
    let runtime = RemoteHostRuntime::new(support_dir.clone());

    assert_eq!(runtime.current_terminal_output_seq("terminal-a"), 0);
    assert_eq!(runtime.next_terminal_output_seq("terminal-a"), 1);
    assert_eq!(runtime.next_terminal_output_seq("terminal-a"), 2);
    assert_eq!(runtime.next_terminal_output_seq("terminal-b"), 1);
    assert_eq!(runtime.current_terminal_output_seq("terminal-a"), 2);

    runtime.clear_terminal_output_seq("terminal-a");

    assert_eq!(runtime.current_terminal_output_seq("terminal-a"), 0);
    assert_eq!(runtime.current_terminal_output_seq("terminal-b"), 1);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_buffer_window_returns_retained_history_window() {
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-window");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf abcdef".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    let mut window = None;
    for _ in 0..20 {
        let current = runtime
            .terminal_buffer_window(&session_id, 0, buffer_options(3, None, false))
            .expect("terminal buffer window");
        if current.total_characters >= 6 {
            window = Some(current);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let window = window.expect("terminal output");

    assert_eq!(window.data, "abc");
    assert_eq!(window.offset, 0);
    assert_eq!(window.total_characters, 6);
    assert!(window.truncated);
    assert!(!window.has_previous);

    let next = runtime
        .terminal_buffer_window(&session_id, 3, buffer_options(3, None, false))
        .expect("next terminal buffer window");
    assert_eq!(next.data, "def");
    assert_eq!(next.offset, 3);
    assert_eq!(next.total_characters, 6);
    assert!(!next.truncated);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_buffer_window_freezes_pages_for_request_id() {
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-frozen-pages");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("cat".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    terminals
        .write(&session_id, b"abcdef")
        .expect("write initial");

    let mut first = None;
    for _ in 0..20 {
        let current = runtime
            .terminal_buffer_window(
                &session_id,
                0,
                buffer_options(3, Some("request-freeze"), false),
            )
            .expect("first terminal buffer window");
        if current.total_characters >= 6 {
            first = Some(current);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let first = first.expect("terminal output");
    assert_eq!(first.data, "abc");
    assert_eq!(first.total_characters, 6);
    assert!(first.truncated);

    terminals
        .write(&session_id, b"XYZ")
        .expect("write appended");
    std::thread::sleep(std::time::Duration::from_millis(25));

    let second = runtime
        .terminal_buffer_window(
            &session_id,
            3,
            buffer_options(3, Some("request-freeze"), false),
        )
        .expect("second terminal buffer window");
    assert_eq!(second.data, "def");
    assert_eq!(second.offset, 3);
    assert_eq!(second.total_characters, 6);
    assert_eq!(second.output_seq, first.output_seq);
    assert!(!second.truncated);

    let live = runtime
        .terminal_buffer_window(
            &session_id,
            0,
            buffer_options(16, Some("request-live"), false),
        )
        .expect("live terminal buffer window");
    assert!(live.total_characters >= 9);
    assert!(live.data.contains("XYZ"));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_buffer_window_tail_returns_history_tail() {
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-tail-window");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf abcdef".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    let mut window = None;
    for _ in 0..20 {
        let current = runtime
            .terminal_buffer_window(&session_id, 0, buffer_options(3, Some("request-1"), true))
            .expect("terminal buffer window");
        if current.data.contains("def") {
            window = Some(current);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let window = window.expect("terminal output");

    assert!(window.data.contains("def"));
    assert_eq!(window.offset, 3);
    assert_eq!(window.total_characters, 6);
    assert!(!window.truncated);
    assert_eq!(window.request_id.as_deref(), Some("request-1"));
    assert!(window.tail);
    assert!(window.has_previous);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_buffer_window_tail_omits_keyframe_for_normal_screen() {
    // A normal-screen session reconstructs fully from the raw history tail, so
    // the baseline must NOT also ship the screen keyframe: replaying both the
    // history AND the keyframe redraws the current line and leaves a duplicate
    // (ghost) first prompt line on the viewer. Alt-screen sessions still ship
    // it -- see terminal_resource_subscribe_baseline_keyframe_for_alt_screen.
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-screen-baseline");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    );
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf 'old line\\n\\033[2J\\033[Hvisible tui'".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    let mut window = None;
    for _ in 0..20 {
        let current = runtime
            .terminal_buffer_window(&session_id, 0, buffer_options(64, Some("request-1"), true))
            .expect("terminal buffer window");
        if current.data.contains("visible tui") {
            window = Some(current);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let window = window.expect("terminal buffer window");

    // The raw history alone reconstructs the visible screen ...
    assert!(window.data.contains("visible tui"));
    // ... so a normal screen must NOT carry a redundant keyframe on top.
    assert!(
        window.screen_data.is_none(),
        "normal-screen baseline must not ship a keyframe (it duplicates the prompt)"
    );
    assert!(window.tail);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_buffer_window_tail_includes_target_viewport_keyframe() {
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-viewport-keyframe");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some(
                    "printf 'wide normal screen\\n\\033[2J\\033[Hmobile keyframe'".to_string(),
                ),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    for _ in 0..20 {
        if terminals
            .screen_snapshot(&session_id)
            .map(|snapshot| snapshot.data.contains("mobile keyframe"))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    terminals
        .claim_viewport(&session_id, "remote:phone-a")
        .expect("phone owns viewport");
    runtime.send_terminal_buffer(
        &session_id,
        Some("phone-a"),
        0,
        viewport_buffer_options(128, Some("request-1"), true, 72, 18),
    );

    let (_, data) = transport
        .wait_for_message(|(_, data)| {
            let Ok(text) = std::str::from_utf8(data) else {
                return false;
            };
            let Ok(envelope) = runtime.service().parse_incoming_envelope(text) else {
                return false;
            };
            envelope.kind == REMOTE_TERMINAL_OUTPUT
                && envelope.payload.get("buffer").and_then(Value::as_bool) == Some(true)
        })
        .expect("terminal baseline");
    let text = String::from_utf8(data).expect("utf8 transport");
    let envelope = runtime
        .service()
        .parse_incoming_envelope(&text)
        .expect("parse outgoing envelope");
    let baseline = envelope.payload;

    assert_eq!(baseline["tail"], true);
    let screen_data = baseline["screenData"]
        .as_str()
        .expect("target viewport baseline must ship keyframe");
    assert!(screen_data.contains("mobile keyframe"));
    let snapshot = terminals
        .screen_snapshot(&session_id)
        .expect("screen snapshot after viewport baseline");
    assert_eq!(snapshot.cols, 72);
    assert_eq!(snapshot.rows, 18);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_baseline_viewport_does_not_steal_from_other_owner() {
    let support_dir = temp_support_dir("codux-remote-terminal-baseline-no-steal");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf 'desktop owned'".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    terminals
        .claim_viewport(&session_id, "desktop")
        .expect("desktop owns viewport");

    runtime.send_terminal_buffer(
        &session_id,
        Some("phone-a"),
        0,
        viewport_buffer_options(128, Some("request-1"), true, 72, 18),
    );

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "desktop");
    assert_eq!(state.cols, 100);
    assert_eq!(state.rows, 32);

    let (_, data) = transport
        .wait_for_message(|(_, data)| {
            let Ok(text) = std::str::from_utf8(data) else {
                return false;
            };
            let Ok(envelope) = runtime.service().parse_incoming_envelope(text) else {
                return false;
            };
            envelope.kind == REMOTE_TERMINAL_OUTPUT
                && envelope.payload.get("buffer").and_then(Value::as_bool) == Some(true)
        })
        .expect("terminal baseline");
    let text = String::from_utf8(data).expect("utf8 transport");
    let envelope = runtime
        .service()
        .parse_incoming_envelope(&text)
        .expect("parse outgoing envelope");
    assert!(envelope.payload.get("screenData").is_none());

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn project_terminal_baseline_viewport_targets_active_split_only() {
    let support_dir = temp_support_dir("codux-remote-project-baseline-active-viewport");
    write_paired_remote_settings(&support_dir);
    let project_dir = support_dir.join("project-a");
    fs::create_dir_all(&project_dir).expect("create project dir");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_a = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf '\\033[2J\\033[Hactive split'".to_string()),
                cwd: Some(project_dir.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal a");
    let session_b = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf '\\033[2J\\033[Hbackground split'".to_string()),
                cwd: Some(project_dir.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal b");

    for _ in 0..20 {
        let ready_a = terminals
            .screen_snapshot(&session_a)
            .map(|snapshot| snapshot.data.contains("active split"))
            .unwrap_or(false);
        let ready_b = terminals
            .screen_snapshot(&session_b)
            .map(|snapshot| snapshot.data.contains("background split"))
            .unwrap_or(false);
        if ready_a && ready_b {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    terminals
        .claim_viewport(&session_a, "remote:phone-a")
        .expect("phone owns active split");
    transport.take_messages();
    runtime.handle_resource_subscribe(&RemoteEnvelope {
        kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
        device_id: Some("phone-a".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "resource": REMOTE_RESOURCE_TERMINALS,
            "projectId": "project-a",
            "baseline": true,
            "baselineSessionId": session_a.clone(),
            "viewportCols": 72,
            "viewportRows": 18,
        }),
    });

    let mut active_baseline = None;
    let mut background_baseline = None;
    for _ in 0..40 {
        for (device_id, data) in transport.take_messages() {
            if device_id.as_deref() != Some("phone-a") {
                continue;
            }
            let text = String::from_utf8(data).expect("utf8 transport");
            let envelope = runtime
                .service()
                .parse_incoming_envelope(&text)
                .expect("parse outgoing envelope");
            if envelope.kind != REMOTE_TERMINAL_OUTPUT
                || envelope.payload.get("buffer").and_then(Value::as_bool) != Some(true)
            {
                continue;
            }
            if envelope.session_id.as_deref() == Some(session_a.as_str()) {
                active_baseline = Some(envelope.payload);
            } else if envelope.session_id.as_deref() == Some(session_b.as_str()) {
                background_baseline = Some(envelope.payload);
            }
        }
        if active_baseline.is_some() && background_baseline.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    assert!(
        active_baseline
            .as_ref()
            .and_then(|payload| payload.get("screenData"))
            .and_then(Value::as_str)
            .map(|screen_data| screen_data.contains("active split"))
            .unwrap_or(false),
        "active split should receive a target-viewport keyframe"
    );
    assert!(
        background_baseline
            .as_ref()
            .and_then(|payload| payload.get("screenData"))
            .is_none(),
        "background splits must not be resized or keyframed to the active split viewport"
    );
    let active_state = terminals
        .viewport_state(&session_a)
        .expect("active viewport state");
    let background_state = terminals
        .viewport_state(&session_b)
        .expect("background viewport state");
    assert_eq!(active_state.cols, 72);
    assert_eq!(active_state.rows, 18);
    assert_eq!(background_state.cols, 100);
    assert_eq!(background_state.rows, 32);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn viewport_state_marks_stale_output_per_viewer() {
    let support_dir = temp_support_dir("codux-remote-viewport-state-per-viewer-stale");
    let project_dir = support_dir.join("project-a");
    fs::create_dir_all(&project_dir).expect("create project dir");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(project_dir.to_string_lossy().to_string()),
                project_id: Some("project-a".to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.register_terminal_viewer(&session_id, Some("phone-a"));
    runtime.register_terminal_viewer(&session_id, Some("phone-b"));
    runtime.record_terminal_output_ack(&session_id, Some("phone-a"), Some(20));
    runtime.record_terminal_output_ack(&session_id, Some("phone-b"), Some(11));
    if let Ok(mut sequences) = runtime.terminal_output_seq_by_session.lock() {
        sequences.insert(session_id.clone(), 20);
    }
    transport.take_messages();
    runtime.handle_terminal_event(TerminalEvent::Viewport {
        session_id: session_id.clone(),
        owner: "desktop".to_string(),
        cols: 100,
        rows: 32,
        generation: 1,
    });

    let mut stale_by_device = HashMap::new();
    for (device_id, data) in transport.take_messages() {
        let text = String::from_utf8(data).expect("utf8 transport");
        let envelope = runtime
            .service()
            .parse_incoming_envelope(&text)
            .expect("parse outgoing envelope");
        if envelope.kind == REMOTE_TERMINAL_VIEWPORT_STATE {
            stale_by_device.insert(
                device_id.expect("device id"),
                envelope.payload["staleOutput"].as_bool().unwrap_or(false),
            );
        }
    }

    assert_eq!(stale_by_device.get("phone-a"), Some(&false));
    assert_eq!(stale_by_device.get("phone-b"), Some(&true));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_resource_subscribe_baseline_keyframe_for_alt_screen() {
    // The alternate buffer has no scrollback, so a re-attaching viewer cannot
    // reconstruct an alt-screen TUI from the raw history alone -- the baseline
    // MUST carry the screen keyframe. (A normal screen does NOT; that path is
    // covered by remote_terminal_buffer_window_tail_omits_keyframe_for_normal_screen.)
    let support_dir = temp_support_dir("codux-resource-subscribe-terminal-screen-baseline");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf '\\033[?1049h\\033[2J\\033[HALT_UI'".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    // Wait for the alternate screen to be active and painted.
    for _ in 0..40 {
        if terminals
            .screen_snapshot(&session_id)
            .map(|snapshot| {
                snapshot.input_mode.alternate_screen && snapshot.data.contains("ALT_UI")
            })
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    runtime.handle_resource_subscribe(&RemoteEnvelope {
        kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
        device_id: Some("phone-a".to_string()),
        session_id: None,
        seq: None,
        payload: json!({
            "resource": REMOTE_RESOURCE_TERMINALS,
            "sessionId": session_id,
            "baseline": true,
            "maxChars": 64,
            "requestId": "request-1",
        }),
    });

    let (_, data) = transport
        .wait_for_message(|(_, data)| {
            let Ok(text) = std::str::from_utf8(data) else {
                return false;
            };
            let Ok(envelope) = runtime.service().parse_incoming_envelope(text) else {
                return false;
            };
            envelope.kind == REMOTE_TERMINAL_OUTPUT
                && envelope.payload.get("buffer").and_then(Value::as_bool) == Some(true)
        })
        .expect("terminal baseline");
    let text = String::from_utf8(data).expect("utf8 transport");
    let envelope = runtime
        .service()
        .parse_incoming_envelope(&text)
        .expect("parse outgoing envelope");
    let baseline = envelope.payload;

    assert_eq!(baseline["requestId"], "request-1");
    assert_eq!(baseline["tail"], true);
    // The keyframe is the only way to restore the alt-screen TUI, so it must
    // be present and carry the alt UI.
    assert!(
        baseline["screenData"]
            .as_str()
            .unwrap_or_default()
            .contains("ALT_UI"),
        "alt-screen baseline must ship the screen keyframe"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn remote_terminal_live_output_is_data_only_without_screen_keyframe() {
    let support_dir = temp_support_dir("codux-remote-terminal-live-screen-keyframe");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some(
                    "printf '\\033[2J\\033[Hrestored tui\\n\\033[3;1Hinput box'".to_string(),
                ),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    for _ in 0..20 {
        if terminals
            .screen_snapshot(&session_id)
            .map(|snapshot| snapshot.data.contains("restored tui"))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    runtime.register_terminal_viewer(&session_id, Some("device-1"));
    transport.take_messages();

    runtime.queue_terminal_output_batch(session_id.clone(), "partial live raw".to_string());
    runtime.flush_terminal_output_batch(&session_id);

    let mut live = None;
    for (_, data) in transport.take_messages() {
        let text = String::from_utf8(data).expect("utf8 transport");
        let envelope = runtime
            .service()
            .parse_incoming_envelope(&text)
            .expect("parse outgoing envelope");
        if envelope.kind == "terminal.output" && envelope.session_id.as_deref() == Some(&session_id)
        {
            live = Some(envelope.payload);
            break;
        }
    }
    let live = live.expect("live terminal output");

    assert_eq!(live["data"], "partial live raw");
    assert_eq!(live["outputSeq"], 1);
    // Live output is a pure byte stream now — NO screen keyframe. Replaying a
    // whole-screen keyframe on top of the emulator's own scrollback duplicated
    // the screen (badly on resize bursts), so the host no longer sends one.
    assert!(
        live.get("screenData").is_none(),
        "live terminal output must not carry a screen keyframe"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_buffer_request_does_not_resize_remote_pty() {
    let support_dir = temp_support_dir("codux-remote-terminal-buffer-readonly");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.handle_terminal_buffer(&RemoteEnvelope {
        kind: "terminal.buffer".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "offset": 0,
            "cols": 44,
            "rows": 12,
        }),
    });

    let info = terminals
        .list()
        .into_iter()
        .find(|terminal| terminal.id == session_id)
        .expect("terminal");
    assert_eq!(info.cols, 100);
    assert_eq!(info.rows, 32);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_viewport_resize_uses_remote_owner() {
    let support_dir = temp_support_dir("codux-remote-terminal-viewport-owner");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.handle_terminal_viewport_claim(&RemoteEnvelope {
        kind: "terminal.viewport.claim".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({}),
    });
    runtime.handle_terminal_viewport_resize(&RemoteEnvelope {
        kind: "terminal.viewport.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "cols": 72,
            "rows": 18,
        }),
    });

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!(state.cols, 72);
    assert_eq!(state.rows, 18);

    let ignored = terminals
        .resize_viewport(&session_id, "remote:device-2", 120, 40)
        .expect("resize from non-owner");
    assert!(ignored.is_none());
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!(state.cols, 72);
    assert_eq!(state.rows, 18);

    let ignored = terminals
        .resize_viewport(&session_id, "desktop", 100, 32)
        .expect("resize from desktop while remote owns");
    assert!(ignored.is_none());
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!(state.cols, 72);
    assert_eq!(state.rows, 18);

    terminals
        .claim_viewport(&session_id, "desktop")
        .expect("desktop claim");
    let accepted = terminals
        .resize_viewport(&session_id, "desktop", 100, 32)
        .expect("desktop resize")
        .expect("accepted desktop resize");
    assert_eq!(accepted.owner, "desktop");
    assert_eq!(accepted.cols, 100);
    assert_eq!(accepted.rows, 32);

    let ignored = terminals
        .resize_viewport(&session_id, "remote:device-1", 72, 18)
        .expect("old remote resize after desktop claim");
    assert!(ignored.is_none());
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "desktop");
    assert_eq!(state.cols, 100);
    assert_eq!(state.rows, 32);

    runtime.handle_terminal_viewport_resize(&RemoteEnvelope {
        kind: "terminal.viewport.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "cols": 72,
            "rows": 18,
        }),
    });
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!(state.cols, 72);
    assert_eq!(state.rows, 18);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_viewport_resize_pushes_state_without_screen_keyframe() {
    let support_dir = temp_support_dir("codux-remote-terminal-viewport-keyframe");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    for _ in 0..20 {
        if terminals
            .screen_snapshot(&session_id)
            .map(|snapshot| snapshot.data.contains("ready"))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    transport.take_messages();
    runtime.handle_terminal_viewport_claim(&RemoteEnvelope {
        kind: "terminal.viewport.claim".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({}),
    });
    transport.take_messages();
    runtime.handle_terminal_viewport_resize(&RemoteEnvelope {
        kind: "terminal.viewport.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "cols": 72,
            "rows": 18,
        }),
    });

    let mut saw_state = false;
    let mut keyframe = None;
    for (device_id, data) in transport.take_messages() {
        let text = String::from_utf8(data).expect("utf8 transport");
        let envelope = runtime
            .service()
            .parse_incoming_envelope(&text)
            .expect("parse outgoing envelope");
        match envelope.kind.as_str() {
            REMOTE_TERMINAL_VIEWPORT_STATE
                if device_id.as_deref() == Some("device-1")
                    && envelope.session_id.as_deref() == Some(&session_id) =>
            {
                saw_state = true
            }
            REMOTE_TERMINAL_OUTPUT => {
                if device_id.as_deref() == Some("device-1")
                    && envelope.session_id.as_deref() == Some(&session_id)
                {
                    keyframe = Some(envelope.payload);
                }
            }
            _ => {}
        }
    }

    assert!(saw_state, "resize must still push viewport state");
    // No screen keyframe: the desktop emulator handles resize via the shell's
    // own repaint in the live byte stream (like a local terminal). Pushing a
    // whole-screen keyframe duplicated the screen on every resize event.
    assert!(
        keyframe.is_none(),
        "resize must not push a screen keyframe (it duplicated on resize)"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_subscribe_does_not_push_screen_keyframe() {
    let support_dir = temp_support_dir("codux-remote-terminal-subscribe-keyframe");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    for _ in 0..20 {
        if terminals
            .screen_snapshot(&session_id)
            .map(|snapshot| snapshot.data.contains("ready"))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    transport.take_messages();
    runtime.handle_terminal_subscribe(&RemoteEnvelope {
        kind: "terminal.subscribe".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({}),
    });

    let mut keyframe = None;
    for (device_id, data) in transport.take_messages() {
        let text = String::from_utf8(data).expect("utf8 transport");
        let envelope = runtime
            .service()
            .parse_incoming_envelope(&text)
            .expect("parse outgoing envelope");
        if device_id.as_deref() == Some("device-1")
            && envelope.kind == "terminal.output"
            && envelope.session_id.as_deref() == Some(&session_id)
        {
            keyframe = Some(envelope.payload);
            break;
        }
    }
    // A plain subscribe (no baseline requested) pushes viewport state only —
    // no screen keyframe. The keyframe duplicated the screen in the desktop's
    // own scrollback; the re-attach seed rides the baseline buffer instead.
    assert!(
        keyframe.is_none(),
        "subscribe must not push a screen keyframe (it duplicated the screen)"
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn device_disconnect_releases_owned_terminal_viewport() {
    let support_dir = temp_support_dir("codux-remote-terminal-viewport-disconnect");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.handle_terminal_viewport_resize(&RemoteEnvelope {
        kind: "terminal.viewport.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "cols": 72,
            "rows": 18,
        }),
    });
    assert_eq!(
        terminals
            .viewport_state(&session_id)
            .expect("viewport state")
            .owner,
        "remote:device-1"
    );

    runtime.handle_remote_envelope(RemoteEnvelope {
        kind: "device.disconnected".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: None,
        seq: None,
        payload: json!({}),
    });

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!((state.cols, state.rows), (72, 18));

    let expired = terminals
        .expire_viewport_lease_for_test(&session_id)
        .expect("expire viewport lease")
        .expect("expired viewport state");
    assert_eq!(expired.owner, "desktop");
    assert_eq!((expired.cols, expired.rows), (72, 18));

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn device_transport_disconnect_keeps_viewport_until_lease_expires() {
    let support_dir = temp_support_dir("codux-remote-terminal-viewport-transport-disconnect");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    runtime.connection_generation.store(7, Ordering::SeqCst);
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    terminals
        .claim_viewport(&session_id, "remote:device-1")
        .expect("remote claim");

    runtime.handle_transport_state(7, "device-1".to_string(), "disconnected".to_string());

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");

    let expired = terminals
        .expire_viewport_lease_for_test(&session_id)
        .expect("expire viewport lease")
        .expect("expired viewport state");
    assert_eq!(expired.owner, "desktop");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_input_reclaims_viewport_after_lease_expired_to_host() {
    let support_dir = temp_support_dir("codux-remote-terminal-input-reclaim");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("cat".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    terminals
        .claim_viewport(&session_id, "remote:device-1")
        .expect("remote claim");
    let expired = terminals
        .expire_viewport_lease_for_test(&session_id)
        .expect("expire viewport lease")
        .expect("expired viewport state");
    assert_eq!(expired.owner, "desktop");

    // Nobody is driving: the first remote input re-claims and is accepted.
    runtime.handle_terminal_input(&RemoteEnvelope {
        kind: "terminal.input".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({ "data": "x" }),
    });
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");

    // A different device is still rejected while the lease is live.
    runtime.handle_terminal_input(&RemoteEnvelope {
        kind: "terminal.input".to_string(),
        device_id: Some("device-2".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({ "data": "y" }),
    });
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn host_transport_disconnect_releases_remote_terminal_viewports() {
    let support_dir = temp_support_dir("codux-remote-terminal-viewport-host-disconnect");
    write_paired_remote_settings(&support_dir);
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    runtime.connection_generation.store(7, Ordering::SeqCst);
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(Arc::new(CapturingTransport::default()));
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    terminals
        .claim_viewport(&session_id, "remote:device-1")
        .expect("remote claim");

    runtime.handle_transport_state(7, String::new(), "closed".to_string());

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "desktop");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_resize_without_owner_claims_remote_viewport_for_compatibility() {
    let support_dir = temp_support_dir("codux-remote-terminal-resize-without-owner");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.handle_terminal_resize(&RemoteEnvelope {
        kind: "terminal.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({
            "cols": 80,
            "rows": 24,
        }),
    });

    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_eq!(state.owner, "remote:device-1");
    assert_eq!(state.cols, 80);
    assert_eq!(state.rows, 24);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn terminal_resize_without_dimensions_is_rejected() {
    let support_dir = temp_support_dir("codux-remote-terminal-resize-reject");
    let terminals = Arc::new(TerminalManager::new());
    let runtime = Arc::new(RemoteHostRuntime::new_with_ai_history_and_terminals(
        support_dir.clone(),
        Default::default(),
        Arc::clone(&terminals),
    ));
    let transport = Arc::new(CapturingTransport::default());
    if let Ok(mut current) = runtime.transport.lock() {
        *current = Some(transport.clone());
    }
    let session_id = terminals
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(support_dir.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");

    runtime.handle_terminal_resize(&RemoteEnvelope {
        kind: "terminal.resize".to_string(),
        device_id: Some("device-1".to_string()),
        session_id: Some(session_id.clone()),
        seq: None,
        payload: json!({}),
    });

    let messages = transport.take_messages();
    assert_eq!(messages.len(), 1);
    let envelope: RemoteEnvelope = serde_json::from_slice(&messages[0].1).expect("error envelope");
    assert_eq!(envelope.kind, REMOTE_ERROR);
    assert_eq!(
        envelope.payload["message"],
        "terminal.resize requires positive cols."
    );
    let state = terminals
        .viewport_state(&session_id)
        .expect("viewport state");
    assert_ne!(state.owner, "remote:device-1");

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn ai_stats_watcher_tracks_one_project_per_device_and_clears_on_disconnect() {
    let support_dir = temp_support_dir("codux-remote-ai-stats-watcher");
    let runtime = RemoteHostRuntime::new(support_dir.clone());

    runtime.register_ai_stats_watcher("project-a", "device-1", "project-a");
    runtime.register_ai_stats_watcher("project-a", "device-2", "worktree-x");
    {
        let watchers = runtime.ai_stats_watchers.lock().unwrap();
        assert_eq!(watchers["project-a"].len(), 2);
        assert_eq!(watchers["project-a"]["device-2"], "worktree-x");
    }

    // Switching a device to another project drops its old-project entry.
    runtime.register_ai_stats_watcher("project-b", "device-1", "project-b");
    {
        let watchers = runtime.ai_stats_watchers.lock().unwrap();
        assert!(!watchers["project-a"].contains_key("device-1"));
        assert!(watchers["project-b"].contains_key("device-1"));
        assert!(watchers["project-a"].contains_key("device-2"));
    }

    // Disconnect drops the device from every project, pruning empties.
    runtime.clear_ai_stats_watcher_device("device-1");
    runtime.clear_ai_stats_watcher_device("device-2");
    assert!(runtime.ai_stats_watchers.lock().unwrap().is_empty());

    fs::remove_dir_all(support_dir).ok();
}
