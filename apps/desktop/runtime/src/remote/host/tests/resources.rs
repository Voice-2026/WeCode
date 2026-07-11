use super::*;

#[test]
fn terminal_project_subscriptions_keep_devices_scoped_to_their_projects() {
    let support_dir = temp_support_dir("wecode-remote-terminal-subscriptions");
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
    let support_dir = temp_support_dir("wecode-project-select-terminal-viewers");
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
    let support_dir = temp_support_dir("wecode-remote-resource-subscriptions");
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
    let support_dir = temp_support_dir("wecode-remote-resource-terminal-tail-baseline");
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
