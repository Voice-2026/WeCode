use super::*;

#[test]
fn host_transport_disconnect_clears_stale_transport_and_enters_reconnect() {
    let support_dir = temp_support_dir("wecode-remote-host-reconnect");
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
    let support_dir = temp_support_dir("wecode-remote-host-reconnect-stale");
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
    let support_dir = temp_support_dir("wecode-remote-host-pairing-restart");
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
    let support_dir = temp_support_dir("wecode-remote-unauthorized-repair");
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
    let support_dir = temp_support_dir("wecode-remote-unauthorized-transport-device-repair");
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
    let support_dir = temp_support_dir("wecode-remote-host-metrics");
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
    let metrics: wecode_protocol::RemoteHostMetrics =
        serde_json::from_value(envelope.payload).expect("metrics payload");
    assert!(metrics.sampled_at_millis > 0);
    assert!(metrics.processes.len() <= 30);

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn web_tunnel_requires_paired_device_token() {
    let support_dir = temp_support_dir("wecode-remote-web-tunnel-token");
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

#[test]
fn transport_ping_runtime_fallback_replies_plain_pong() {
    let support_dir = temp_support_dir("wecode-remote-transport-ping-pong");
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
    let support_dir = temp_support_dir("wecode-remote-viewport-no-terminal-list");
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
