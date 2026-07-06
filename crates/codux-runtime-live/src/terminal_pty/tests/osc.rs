use super::*;

#[cfg(unix)]
#[test]
fn codewhale_terminal_progress_osc_starts_idle_session() {
    let dir = std::env::temp_dir().join(format!(
        "codux-codewhale-terminal-progress-start-{}",
        Uuid::new_v4()
    ));
    let bridge = Arc::new(AIRuntimeBridge::with_paths(
        dir.join("root"),
        dir.join("temp"),
        dir.join("home"),
    ));
    bridge.ensure_started().expect("runtime should start");
    let terminal_id = format!("test-codewhale-terminal-start-{}", Uuid::new_v4());
    let binding = AIRuntimeTerminalBinding {
        terminal_id: terminal_id.clone(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Terminal".to_string(),
        cwd: "/tmp/project".to_string(),
        tool: None,
        is_active: false,
        session_key: Some("codewhale-session-1".to_string()),
        terminal_instance_id: Some("terminal-instance-1".to_string()),
    };
    let mut watcher = AIRuntimeTerminalOutputWatcher::new(binding.clone(), Arc::clone(&bridge));
    bridge
        .submit_hook_event(AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            terminal_id: terminal_id.clone(),
            terminal_instance_id: binding.terminal_instance_id.clone(),
            project_id: "project-1".to_string(),
            project_name: "Codux".to_string(),
            project_path: Some("/tmp/project".to_string()),
            session_title: "Terminal".to_string(),
            tool: "codewhale".to_string(),
            ai_session_id: Some("codewhale-session-1".to_string()),
            model: None,
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: now_seconds(),
            metadata: None,
        })
        .expect("session hook should submit");
    wait_for_session_state(&bridge, &terminal_id, "idle", Duration::from_secs(2));

    watcher.handle_terminal_event(&TerminalEvent::Output {
        session_id: terminal_id.clone(),
        text: String::new(),
        bytes: b"\x1b]9;4;1\x07".to_vec(),
    });

    wait_for_session_state(&bridge, &terminal_id, "responding", Duration::from_secs(2));
    let session = bridge
        .runtime_state_snapshot()
        .sessions
        .into_iter()
        .find(|session| session.terminal_id == terminal_id)
        .expect("session should exist");
    assert_eq!(session.tool, "codewhale");
    assert!(!session.has_completed_turn);

    let _ = std::fs::remove_dir_all(dir);
}
#[cfg(unix)]
#[test]
fn codewhale_terminal_progress_osc_completes_running_session() {
    let dir = std::env::temp_dir().join(format!(
        "codux-codewhale-terminal-progress-{}",
        Uuid::new_v4()
    ));
    let bridge = Arc::new(AIRuntimeBridge::with_paths(
        dir.join("root"),
        dir.join("temp"),
        dir.join("home"),
    ));
    bridge.ensure_started().expect("runtime should start");
    let terminal_id = format!("test-codewhale-terminal-{}", Uuid::new_v4());
    let binding = AIRuntimeTerminalBinding {
        terminal_id: terminal_id.clone(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Terminal".to_string(),
        cwd: "/tmp/project".to_string(),
        tool: None,
        is_active: false,
        session_key: Some("codewhale-session-1".to_string()),
        terminal_instance_id: Some("terminal-instance-1".to_string()),
    };
    let mut watcher = AIRuntimeTerminalOutputWatcher::new(binding.clone(), Arc::clone(&bridge));
    bridge
        .submit_hook_event(AIHookEventPayload {
            kind: "promptSubmitted".to_string(),
            terminal_id: terminal_id.clone(),
            terminal_instance_id: binding.terminal_instance_id.clone(),
            project_id: "project-1".to_string(),
            project_name: "Codux".to_string(),
            project_path: Some("/tmp/project".to_string()),
            session_title: "Terminal".to_string(),
            tool: "codewhale".to_string(),
            ai_session_id: Some("codewhale-session-1".to_string()),
            model: None,
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: now_seconds(),
            metadata: None,
        })
        .expect("prompt hook should submit");
    wait_for_session_state(&bridge, &terminal_id, "responding", Duration::from_secs(2));

    watcher.handle_terminal_event(&TerminalEvent::Output {
        session_id: terminal_id.clone(),
        text: String::new(),
        bytes: b"\x1b]9;4;0\x07".to_vec(),
    });
    wait_for_session_state(&bridge, &terminal_id, "idle", Duration::from_secs(2));

    let snapshot = bridge.runtime_state_snapshot();
    let session = snapshot
        .sessions
        .iter()
        .find(|session| session.terminal_id == terminal_id)
        .expect("session should exist");
    assert_eq!(session.tool, "codewhale");
    assert!(session.has_completed_turn);

    let _ = std::fs::remove_dir_all(dir);
}
#[cfg(unix)]
#[test]
fn terminal_progress_osc_does_not_complete_non_codewhale_session() {
    let dir = std::env::temp_dir().join(format!(
        "codux-codewhale-terminal-progress-ignore-{}",
        Uuid::new_v4()
    ));
    let bridge = Arc::new(AIRuntimeBridge::with_paths(
        dir.join("root"),
        dir.join("temp"),
        dir.join("home"),
    ));
    bridge.ensure_started().expect("runtime should start");
    let terminal_id = format!("test-codex-terminal-{}", Uuid::new_v4());
    let binding = AIRuntimeTerminalBinding {
        terminal_id: terminal_id.clone(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Terminal".to_string(),
        cwd: "/tmp/project".to_string(),
        tool: None,
        is_active: false,
        session_key: Some("codex-session-1".to_string()),
        terminal_instance_id: Some("terminal-instance-1".to_string()),
    };
    let mut watcher = AIRuntimeTerminalOutputWatcher::new(binding.clone(), Arc::clone(&bridge));
    bridge
        .submit_hook_event(AIHookEventPayload {
            kind: "promptSubmitted".to_string(),
            terminal_id: terminal_id.clone(),
            terminal_instance_id: binding.terminal_instance_id.clone(),
            project_id: "project-1".to_string(),
            project_name: "Codux".to_string(),
            project_path: Some("/tmp/project".to_string()),
            session_title: "Terminal".to_string(),
            tool: "codex".to_string(),
            ai_session_id: Some("codex-session-1".to_string()),
            model: None,
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: now_seconds(),
            metadata: None,
        })
        .expect("prompt hook should submit");
    wait_for_session_state(&bridge, &terminal_id, "responding", Duration::from_secs(2));

    watcher.handle_terminal_event(&TerminalEvent::Output {
        session_id: terminal_id.clone(),
        text: String::new(),
        bytes: b"\x1b]9;4;0\x07".to_vec(),
    });
    std::thread::sleep(Duration::from_millis(150));

    let snapshot = bridge.runtime_state_snapshot();
    let session = snapshot
        .sessions
        .iter()
        .find(|session| session.terminal_id == terminal_id)
        .expect("session should exist");
    assert_eq!(session.tool, "codex");
    assert_eq!(session.state, "responding");
    assert!(!session.has_completed_turn);

    let _ = std::fs::remove_dir_all(dir);
}
#[cfg(unix)]
#[test]
fn terminal_output_refreshes_kiro_screen_signal_without_poll() {
    let dir = std::env::temp_dir().join(format!(
        "codux-kiro-terminal-screen-signal-{}",
        Uuid::new_v4()
    ));
    let bridge = Arc::new(AIRuntimeBridge::with_paths(
        dir.join("root"),
        dir.join("temp"),
        dir.join("home"),
    ));
    bridge.ensure_started().expect("runtime should start");
    let terminal_id = format!("test-kiro-terminal-{}", Uuid::new_v4());
    let binding = AIRuntimeTerminalBinding {
        terminal_id: terminal_id.clone(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Kiro".to_string(),
        cwd: "/tmp/project".to_string(),
        tool: Some("kiro".to_string()),
        is_active: false,
        session_key: Some("kiro-session-1".to_string()),
        terminal_instance_id: Some("terminal-instance-1".to_string()),
    };
    bridge.registry().upsert(binding.clone());
    let screen = Arc::new(parking_lot::Mutex::new(HeadlessTerminalScreen::new(
        80, 24, 100,
    )));
    bridge
        .registry()
        .register_screen(&terminal_id, Arc::downgrade(&screen));
    let mut watcher = AIRuntimeTerminalOutputWatcher::new(binding.clone(), Arc::clone(&bridge));
    bridge
        .submit_hook_event(AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            terminal_id: terminal_id.clone(),
            terminal_instance_id: binding.terminal_instance_id.clone(),
            project_id: "project-1".to_string(),
            project_name: "Codux".to_string(),
            project_path: Some("/tmp/project".to_string()),
            session_title: "Kiro".to_string(),
            tool: "kiro".to_string(),
            ai_session_id: Some("kiro-session-1".to_string()),
            model: None,
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: now_seconds(),
            metadata: None,
        })
        .expect("session hook should submit");
    wait_for_session_state(&bridge, &terminal_id, "idle", Duration::from_secs(2));

    let output = "kiro_default · auto\nKiro is working · Type to steer · Ctrl+S to queue";
    screen.lock().process(output.as_bytes());
    watcher.handle_terminal_event(&TerminalEvent::Output {
        session_id: terminal_id.clone(),
        text: output.to_string(),
        bytes: output.as_bytes().to_vec(),
    });

    wait_for_session_state(&bridge, &terminal_id, "responding", Duration::from_secs(2));

    let _ = std::fs::remove_dir_all(dir);
}
