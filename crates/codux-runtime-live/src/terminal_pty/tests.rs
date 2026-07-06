use super::*;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[test]
fn output_capture_keeps_limited_tail_and_total_bytes() {
    let mut capture = TerminalOutputCapture::new(5);
    capture.push(b"hello");
    capture.push(b" world");
    let snapshot = capture.snapshot();
    assert_eq!(snapshot.bytes, 11);
    assert_eq!(snapshot.tail, "world");
}

#[test]
fn output_replay_uses_terminal_history_not_limited_tail() {
    let mut history = RingHistory::new(1024);
    history.push_text("hello");
    history.push_text(" world");
    assert_eq!(history.to_text(), "hello world");

    let mut capture = TerminalOutputCapture::new(5);
    capture.push(b"hello world");
    assert_eq!(capture.snapshot().tail, "world");
}

#[test]
fn terminal_history_tail_returns_recent_window_and_offset() {
    let mut history = RingHistory::new(1024);
    history.push_text("hello");
    history.push_text(" world");

    assert_eq!(history.tail_text(5), ("world".to_string(), 6));
    assert_eq!(history.tail_text(20), ("hello world".to_string(), 0));
}

#[test]
fn terminal_history_tail_starts_after_partial_csi_sequence() {
    let mut history = RingHistory::new(1024);
    history.push_text("line 1\n");
    history.push_text("\x1b[12;27Hprompt");

    let (tail, offset) = history.tail_text(9);

    assert_eq!(tail, "prompt");
    assert_eq!(offset, "line 1\n\x1b[12;27H".chars().count());
}

#[test]
fn terminal_history_tail_starts_after_partial_osc_sequence() {
    let mut history = RingHistory::new(1024);
    history.push_text("line 1\n");
    history.push_text("\x1b]0;Codux\x07prompt");

    let (tail, offset) = history.tail_text(10);

    assert_eq!(tail, "prompt");
    assert_eq!(offset, "line 1\n\x1b]0;Codux\x07".chars().count());
}

#[test]
fn headless_screen_snapshot_replays_current_screen_not_raw_tail() {
    let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
    screen.process(b"old line\n\x1b[2J\x1b[Htop\x1b[3;5Hbottom");

    let snapshot = screen.snapshot();

    assert!(snapshot.data.contains("\x1b[H\x1b[2J"));
    assert!(snapshot.data.contains("top"));
    assert!(snapshot.data.contains("bottom"));
    assert!(!snapshot.data.contains("old line"));
    assert_eq!(snapshot.cols, 20);
    assert_eq!(snapshot.rows, 4);
}

#[test]
fn headless_screen_snapshot_tracks_resize() {
    let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
    screen.resize(30, 10);
    screen.process(b"ready");

    let snapshot = screen.snapshot();

    assert!(snapshot.data.contains("ready"));
    assert_eq!(snapshot.cols, 30);
    assert_eq!(snapshot.rows, 10);
}

#[test]
fn headless_screen_snapshot_does_not_insert_spaces_after_wide_chars() {
    let mut screen = HeadlessTerminalScreen::new(40, 4, 100);
    screen.process("第 2003行 测 试 文 本".as_bytes());

    let snapshot = screen.snapshot();

    assert!(
        snapshot.data.contains("第 2003行 测 试 文 本"),
        "{}",
        snapshot.data.escape_debug()
    );
    assert!(!snapshot.data.contains("第  2003"));
    assert!(!snapshot.data.contains("测  试"));
}

#[test]
fn input_capture_keeps_limited_history_and_total_bytes() {
    let mut capture = TerminalInputCapture::new(2);
    capture.push(b"ls\n");
    capture.push(b" ");
    capture.push(b"pwd\n");
    capture.push(b"echo ok\n");
    let snapshot = capture.snapshot();
    assert_eq!(snapshot.bytes, 16);
    assert_eq!(snapshot.history.len(), 2);
    assert_eq!(snapshot.history[0].text, "pwd\n");
    assert_eq!(snapshot.history[1].text, "echo ok\n");
}

#[test]
fn utf8_decoder_keeps_split_multibyte_characters() {
    let mut pending = Vec::new();
    assert_eq!(decode_utf8_output(&[0xe6, 0x8e], &mut pending), "");
    assert_eq!(decode_utf8_output(&[0xa8], &mut pending), "推");
    assert!(pending.is_empty());
}

#[test]
fn utf8_decoder_flushes_incomplete_tail_on_eof() {
    let mut pending = Vec::new();
    assert_eq!(decode_utf8_output(&[0xe6, 0x8e], &mut pending), "");
    assert_eq!(flush_utf8_decoder(&mut pending), "�");
    assert!(pending.is_empty());
}

#[test]
fn terminal_progress_osc_parser_detects_split_start_and_completion() {
    let mut parser = TerminalProgressOscParser::default();

    assert!(parser.push(b"noise\x1b]9;").is_empty());
    assert_eq!(parser.push(b"4;1\x07"), vec![TerminalProgressOsc::Started]);
    assert_eq!(
        parser.push(b"\x1b]9;4;0\x1b\\"),
        vec![TerminalProgressOsc::Completed]
    );
}

#[test]
fn terminal_progress_osc_parser_ignores_incomplete_sequence() {
    let mut parser = TerminalProgressOscParser::default();

    assert!(parser.push(b"\x1b]9;4;0").is_empty());
    assert_eq!(parser.push(b"\x07"), vec![TerminalProgressOsc::Completed]);
}

#[test]
fn terminal_history_bytes_respects_configured_scrollback() {
    assert_eq!(terminal_history_bytes(Some(10_000), 100), 4 * 100 * 10_000);
    assert_eq!(terminal_history_bytes(Some(1), 100), MIN_HISTORY_BYTES);
}

#[test]
fn remote_screen_scrollback_is_capped() {
    assert_eq!(
        remote_screen_scrollback_lines(None),
        REMOTE_SCREEN_SCROLLBACK_CAP
    );
    assert_eq!(
        remote_screen_scrollback_lines(Some(10_000)),
        REMOTE_SCREEN_SCROLLBACK_CAP
    );
    assert_eq!(remote_screen_scrollback_lines(Some(1200)), 1200);
}

#[test]
fn initial_remote_screen_scrollback_starts_idle() {
    assert_eq!(
        initial_remote_screen_scrollback_lines(REMOTE_SCREEN_SCROLLBACK_CAP),
        REMOTE_SCREEN_IDLE_SCROLLBACK
    );
    assert_eq!(initial_remote_screen_scrollback_lines(1200), 500);
    assert_eq!(initial_remote_screen_scrollback_lines(300), 300);
}

#[test]
fn terminal_environment_forces_utf8_locale() {
    let mut config = TerminalPtyConfig::default();
    config.env = Some(HashMap::from([
        ("LANG".to_string(), "C".to_string()),
        ("LC_ALL".to_string(), "C".to_string()),
        ("LC_CTYPE".to_string(), "POSIX".to_string()),
    ]));

    let env = terminal_environment("/bin/zsh", None, "term-1", &config, None);

    assert_eq!(env.get("LANG").map(String::as_str), Some("en_US.UTF-8"));
    assert_eq!(env.get("LC_ALL").map(String::as_str), Some("en_US.UTF-8"));
    assert_eq!(env.get("LC_CTYPE").map(String::as_str), Some("en_US.UTF-8"));
}

#[test]
fn terminal_environment_does_not_set_term_program() {
    let config = TerminalPtyConfig::default();

    let env = terminal_environment("/bin/zsh", None, "term-1", &config, None);

    assert!(!env.contains_key("TERM_PROGRAM"));
    assert!(!env.contains_key("TERM_PROGRAM_VERSION"));
}

#[test]
fn terminal_environment_preserves_real_term_program() {
    let mut config = TerminalPtyConfig::default();
    config.env = Some(HashMap::from([
        ("TERM_PROGRAM".to_string(), "Ghostty".to_string()),
        ("TERM_PROGRAM_VERSION".to_string(), "1.2.3".to_string()),
    ]));

    let env = terminal_environment("/bin/zsh", None, "term-1", &config, None);

    assert_eq!(env.get("TERM_PROGRAM").map(String::as_str), Some("Ghostty"));
    assert_eq!(
        env.get("TERM_PROGRAM_VERSION").map(String::as_str),
        Some("1.2.3")
    );
}

#[test]
fn terminal_environment_injects_codux_runtime_context() {
    let temp = std::env::temp_dir().join(format!("codux-terminal-runtime-root-{}", Uuid::new_v4()));
    let runtime_root = temp.join("runtime-root");
    fs::create_dir_all(runtime_root.join("scripts/shell-hooks/zsh")).unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zshenv"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zprofile"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zshrc"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/dmux-ai-hook.zsh"),
        "# test\n",
    )
    .unwrap();
    let context = TerminalLaunchContext {
        root_project_id: "project-1".to_string(),
        project_id: "project-1".to_string(),
        project_name: "Codux".to_string(),
        project_path: PathBuf::from("/workspace/codux"),
        support_dir: PathBuf::from("/support/Codux"),
        runtime_root: runtime_root.clone(),
        terminal_id: Some("gpui-term-1".to_string()),
        slot_id: Some("gpui-pane-1-1".to_string()),
        session_key: Some("gpui:project-1:gpui-term-1:gpui-pane-1-1".to_string()),
        session_title: Some("终端 1".to_string()),
        session_cwd: Some(PathBuf::from("/workspace/codux")),
        session_instance_id: Some("session-instance-1".to_string()),
        tool_permissions_file: Some(PathBuf::from("/tmp/codux/tool-permissions.json")),
        memory_workspace_root: Some(PathBuf::from("/tmp/codux/memory-workspaces/project-1")),
        memory_prompt_file: Some(PathBuf::from(
            "/tmp/codux/memory-workspaces/project-1/memory-prompt.txt",
        )),
        memory_index_file: Some(PathBuf::from(
            "/tmp/codux/memory-workspaces/project-1/MEMORY.md",
        )),
        host_device_id: None,
    };
    let env = terminal_environment(
        "/bin/zsh",
        Some("/workspace/codux"),
        "gpui-term-1",
        &context.to_config(),
        Some(&context),
    );
    let path = env.get("PATH").expect("PATH should be set");
    assert!(path.starts_with(runtime_root.join("scripts/wrappers/bin").to_str().unwrap()));
    assert_eq!(
        env.get("DMUX_PROJECT_PATH").map(String::as_str),
        Some("/workspace/codux")
    );
    // Claude Code defaults to its classic (scrollback) renderer for a clean
    // desktop<->mobile handoff, unless the user set the var themselves.
    assert_eq!(
        env.get("CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        env.get("CODUX_TERMINAL_ID").map(String::as_str),
        Some("gpui-term-1")
    );
    assert_eq!(
        env.get("DMUX_SESSION_INSTANCE_ID").map(String::as_str),
        Some("session-instance-1")
    );
    assert_eq!(
        env.get("DMUX_AI_MEMORY_INDEX_FILE").map(String::as_str),
        Some("/tmp/codux/memory-workspaces/project-1/MEMORY.md")
    );
    assert_eq!(
        env.get("DMUX_WRAPPER_BIN").map(String::as_str),
        Some(runtime_root.join("scripts/wrappers/bin").to_str().unwrap())
    );
    assert_eq!(
        env.get("DMUX_APP_SUPPORT_ROOT").map(String::as_str),
        Some("/support/Codux")
    );
    assert_eq!(
        env.get("CODUX_SSH_PROFILES_FILE").map(String::as_str),
        Some("/support/Codux/ssh_profiles.json")
    );
    assert_eq!(
        env.get("CODUX_DB_PROFILES_FILE").map(String::as_str),
        Some("/support/Codux/db_profiles.json")
    );
    assert_eq!(
        env.get("CODUX_DB_PROJECT_ID").map(String::as_str),
        Some("project-1")
    );
    assert_eq!(
        env.get("DMUX_USER_ZDOTDIR").map(String::as_str),
        env.get("HOME").map(String::as_str)
    );
    assert_eq!(
        env.get("ZDOTDIR").map(String::as_str),
        Some(
            runtime_root
                .join("scripts/shell-hooks/zsh")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        env.get("DMUX_ZSH_HOOK_SCRIPT").map(String::as_str),
        Some(
            runtime_root
                .join("scripts/shell-hooks/dmux-ai-hook.zsh")
                .to_str()
                .unwrap()
        )
    );
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn terminal_environment_treats_named_zsh_wrapper_as_zsh() {
    let temp = std::env::temp_dir().join(format!(
        "codux-terminal-runtime-root-named-zsh-{}",
        Uuid::new_v4()
    ));
    let runtime_root = temp.join("runtime-root");
    fs::create_dir_all(runtime_root.join("scripts/shell-hooks/zsh")).unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zshenv"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zprofile"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/zsh/.zshrc"),
        "# test\n",
    )
    .unwrap();
    fs::write(
        runtime_root.join("scripts/shell-hooks/dmux-ai-hook.zsh"),
        "# test\n",
    )
    .unwrap();
    let context = TerminalLaunchContext {
        root_project_id: "project-1".to_string(),
        project_id: "project-1".to_string(),
        project_name: "Codux".to_string(),
        project_path: PathBuf::from("/workspace/codux"),
        support_dir: PathBuf::from("/support/Codux"),
        runtime_root: runtime_root.clone(),
        terminal_id: Some("gpui-term-1".to_string()),
        slot_id: None,
        session_key: None,
        session_title: None,
        session_cwd: Some(PathBuf::from("/workspace/codux")),
        session_instance_id: None,
        tool_permissions_file: None,
        memory_workspace_root: None,
        memory_prompt_file: None,
        memory_index_file: None,
        host_device_id: None,
    };

    let env = terminal_environment(
        "/Users/example/.local/bin/zsh (kiro-cli-term)",
        Some("/workspace/codux"),
        "gpui-term-1",
        &context.to_config(),
        Some(&context),
    );

    assert_eq!(
        env.get("ZDOTDIR").map(String::as_str),
        Some(
            runtime_root
                .join("scripts/shell-hooks/zsh")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        env.get("DMUX_ZSH_HOOK_SCRIPT").map(String::as_str),
        Some(
            runtime_root
                .join("scripts/shell-hooks/dmux-ai-hook.zsh")
                .to_str()
                .unwrap()
        )
    );
    let _ = fs::remove_dir_all(temp);
}

#[cfg(not(target_os = "windows"))]
#[test]
fn terminal_shell_normalization_rejects_nested_integration_shells() {
    assert_eq!(
        normalize_terminal_shell("/Users/example/.local/bin/zsh (kiro-cli-term)"),
        None
    );
    assert_eq!(
        normalize_terminal_shell("/Users/example/.local/bin/kiro-cli-term"),
        None
    );
    assert_eq!(
        normalize_terminal_shell("/bin/zsh"),
        Some("/bin/zsh".to_string())
    );
}

#[test]
fn terminal_environment_does_not_override_zdotdir_when_runtime_zsh_hook_is_incomplete() {
    let temp = std::env::temp_dir().join(format!(
        "codux-terminal-runtime-root-missing-hook-{}",
        Uuid::new_v4()
    ));
    let runtime_root = temp.join("runtime-root");
    fs::create_dir_all(runtime_root.join("scripts/shell-hooks/zsh")).unwrap();
    let context = TerminalLaunchContext {
        root_project_id: "project-1".to_string(),
        project_id: "project-1".to_string(),
        project_name: "Codux".to_string(),
        project_path: PathBuf::from("/workspace/codux"),
        support_dir: PathBuf::from("/support/Codux"),
        runtime_root: runtime_root.clone(),
        terminal_id: Some("gpui-term-1".to_string()),
        slot_id: Some("gpui-pane-1-1".to_string()),
        session_key: Some("gpui:project-1:gpui-term-1:gpui-pane-1-1".to_string()),
        session_title: Some("Terminal 1".to_string()),
        session_cwd: Some(PathBuf::from("/workspace/codux")),
        session_instance_id: Some("session-instance-1".to_string()),
        tool_permissions_file: None,
        memory_workspace_root: None,
        memory_prompt_file: None,
        memory_index_file: None,
        host_device_id: None,
    };

    let env = terminal_environment(
        "/bin/zsh",
        Some("/workspace/codux"),
        "gpui-term-1",
        &context.to_config(),
        Some(&context),
    );

    assert_ne!(
        env.get("ZDOTDIR").map(String::as_str),
        Some(
            runtime_root
                .join("scripts/shell-hooks/zsh")
                .to_str()
                .unwrap()
        )
    );
    assert!(!env.contains_key("DMUX_ZSH_HOOK_SCRIPT"));
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn terminal_environment_keeps_runtime_context_compact() {
    let context = TerminalLaunchContext {
        root_project_id: "project-1".to_string(),
        project_id: "project-1".to_string(),
        project_name: "Codux".to_string(),
        project_path: PathBuf::from("/workspace/codux"),
        support_dir: PathBuf::from("/support/Codux"),
        runtime_root: PathBuf::from("/runtime-assets"),
        terminal_id: Some("gpui-term-1".to_string()),
        slot_id: Some("gpui-pane-1-1".to_string()),
        session_key: Some("gpui:project-1:gpui-term-1:gpui-pane-1-1".to_string()),
        session_title: Some("Terminal 1".to_string()),
        session_cwd: Some(PathBuf::from("/workspace/codux")),
        session_instance_id: Some("session-instance-1".to_string()),
        tool_permissions_file: Some(PathBuf::from("/tmp/codux/tool-permissions.json")),
        memory_workspace_root: Some(PathBuf::from("/tmp/codux/memory-workspaces/project-1")),
        memory_prompt_file: Some(PathBuf::from(
            "/tmp/codux/memory-workspaces/project-1/memory-prompt.txt",
        )),
        memory_index_file: Some(PathBuf::from(
            "/tmp/codux/memory-workspaces/project-1/MEMORY.md",
        )),
        host_device_id: None,
    };

    let env = terminal_environment(
        "/bin/zsh",
        Some("/workspace/codux"),
        "gpui-term-1",
        &context.to_config(),
        Some(&context),
    );
    let total_bytes = env
        .iter()
        .map(|(key, value)| key.len() + value.len() + 2)
        .sum::<usize>();

    assert!(total_bytes < 16 * 1024);
}

#[cfg(not(windows))]
#[test]
fn parses_noisy_shell_environment_capture() {
    let mut output = Vec::new();
    output.extend_from_slice(b"startup noise");
    output.extend_from_slice(b"__BEGIN__\0PATH=/opt/bin:/usr/bin\0HISTFILE=/tmp/history\0");
    output.extend_from_slice(b"__END__\0more noise");

    let env = parse_captured_shell_environment(&output, "__BEGIN__", "__END__").unwrap();

    assert_eq!(
        env.get("PATH").map(String::as_str),
        Some("/opt/bin:/usr/bin")
    );
    assert_eq!(
        env.get("HISTFILE").map(String::as_str),
        Some("/tmp/history")
    );
}

#[cfg(unix)]
#[test]
fn remote_visible_viewport_expires_back_to_desktop() {
    let manager = TerminalManager::new();
    let temp =
        std::env::temp_dir().join(format!("codux-terminal-viewport-lock-{}", Uuid::new_v4()));
    fs::create_dir_all(&temp).unwrap();
    let session_id = manager
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(temp.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    let session = manager.session(&session_id).expect("session");
    let handle = session.clone_handle();

    handle
        .claim_viewport("remote:phone")
        .expect("remote visible claim");
    handle
        .resize_viewport("remote:phone", 72, 18)
        .expect("remote resize")
        .expect("remote resize accepted");
    {
        let mut viewport = session.viewport.lock();
        viewport.expires_at = Instant::now() - Duration::from_secs(1);
    }

    let expired = handle
        .release_expired_viewport_lease()
        .expect("expired viewport state");
    assert_eq!(expired.owner, terminal_viewport_local_owner());
    // Ownership is a handoff token: the remote drove the FULL grid (72x18)
    // while it held the lease, so on expiry the grid keeps that size until
    // the desktop reclaims it by resizing (next assertion).
    assert_eq!((expired.cols, expired.rows), (72, 18));

    let accepted = handle
        .resize_viewport(terminal_viewport_local_owner(), 100, 32)
        .expect("desktop resize after lease expiry")
        .expect("desktop resize accepted");
    let state = handle.viewport_state();
    assert_eq!(state.owner, terminal_viewport_local_owner());
    assert_eq!((accepted.cols, accepted.rows), (100, 32));

    let _ = session.kill();
    fs::remove_dir_all(temp).ok();
}

#[test]
fn expired_remote_viewport_hands_off_to_another_active_viewer() {
    let manager = TerminalManager::new();
    let temp = std::env::temp_dir().join(format!(
        "codux-terminal-viewport-handoff-{}",
        Uuid::new_v4()
    ));
    fs::create_dir_all(&temp).unwrap();
    let session_id = manager
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(temp.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    let session = manager.session(&session_id).expect("session");
    let handle = session.clone_handle();

    handle
        .claim_viewport("remote:phone-a")
        .expect("phone-a claim");
    {
        let mut viewport = session.viewport.lock();
        viewport.expires_at = Instant::now() - Duration::from_secs(1);
    }

    // A resolver names phone-b as another active viewer: the expired lease is
    // handed to it instead of snapping back to the host desktop.
    let reclaimed = handle
        .reclaim_expired_viewport_lease(|expired| {
            assert_eq!(expired, "remote:phone-a");
            Some("remote:phone-b".to_string())
        })
        .expect("handoff state");
    assert_eq!(reclaimed.owner, "remote:phone-b");
    assert_eq!(handle.viewport_state().owner, "remote:phone-b");

    // With no replacement viewer, expiry reverts to the host desktop.
    {
        let mut viewport = session.viewport.lock();
        viewport.expires_at = Instant::now() - Duration::from_secs(1);
    }
    let reverted = handle
        .reclaim_expired_viewport_lease(|_| None)
        .expect("revert state");
    assert_eq!(reverted.owner, terminal_viewport_local_owner());

    let _ = session.kill();
    fs::remove_dir_all(temp).ok();
}

#[cfg(unix)]
#[test]
fn desktop_resize_waits_for_remote_viewport_release() {
    let manager = TerminalManager::new();
    let temp = std::env::temp_dir().join(format!(
        "codux-terminal-viewport-release-{}",
        Uuid::new_v4()
    ));
    fs::create_dir_all(&temp).unwrap();
    let session_id = manager
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(temp.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    let session = manager.session(&session_id).expect("session");
    let handle = session.clone_handle();

    handle.claim_viewport("remote:phone").expect("remote claim");
    handle
        .resize_viewport("remote:phone", 72, 18)
        .expect("remote resize")
        .expect("remote resize accepted");

    let ignored = handle
        .resize_viewport(terminal_viewport_local_owner(), 120, 40)
        .expect("desktop resize while remote owns");
    assert!(ignored.is_none());
    assert_eq!(handle.viewport_state().owner, "remote:phone");
    // The owning remote drives the FULL grid (cols AND rows), so it reflows
    // to its 72x18 -- a handoff, not a host-floored mirror.
    assert_eq!(
        (handle.viewport_state().cols, handle.viewport_state().rows),
        (72, 18)
    );

    handle
        .release_viewport("remote:phone")
        .expect("remote release")
        .expect("release state");
    let accepted = handle
        .resize_viewport(terminal_viewport_local_owner(), 120, 40)
        .expect("desktop resize after release")
        .expect("desktop resize accepted");
    assert_eq!(accepted.owner, terminal_viewport_local_owner());
    assert_eq!((accepted.cols, accepted.rows), (120, 40));

    let _ = session.kill();
    fs::remove_dir_all(temp).ok();
}

#[cfg(unix)]
#[test]
fn viewport_keepalive_prevents_remote_lease_expiry() {
    let manager = TerminalManager::new();
    let temp = std::env::temp_dir().join(format!(
        "codux-terminal-viewport-keepalive-{}",
        Uuid::new_v4()
    ));
    fs::create_dir_all(&temp).unwrap();
    let session_id = manager
        .create(
            TerminalPtyConfig {
                shell: Some("sh".to_string()),
                command: Some("printf ready".to_string()),
                cwd: Some(temp.to_string_lossy().to_string()),
                cols: Some(100),
                rows: Some(32),
                ..Default::default()
            },
            |_| {},
        )
        .expect("create terminal");
    let session = manager.session(&session_id).expect("session");
    let handle = session.clone_handle();

    handle.claim_viewport("remote:phone").expect("remote claim");
    {
        let mut viewport = session.viewport.lock();
        viewport.expires_at = Instant::now() - Duration::from_secs(1);
    }
    handle.touch_viewport_lease("remote:phone");
    assert!(handle.release_expired_viewport_lease().is_none());
    assert_eq!(handle.viewport_state().owner, "remote:phone");

    let _ = session.kill();
    fs::remove_dir_all(temp).ok();
}

#[cfg(unix)]
#[test]
fn terminal_manager_reuses_session_and_broadcasts_to_subscribers() {
    let manager = TerminalManager::new();
    let emit: EventSink = Arc::new(|_| true);
    let config = TerminalPtyConfig {
        terminal_id: Some(format!("test-terminal-{}", Uuid::new_v4())),
        shell: Some("/bin/cat".to_string()),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };

    let (first_session, first_rx) = manager
        .attach_or_create_with_context(config.clone(), None, emit.clone())
        .expect("terminal should start");
    first_session
        .write(b"first-shared-output\n")
        .expect("write should succeed");
    assert!(
        recv_until_contains(&first_rx, "first-shared-output", Duration::from_secs(2))
            .contains("first-shared-output")
    );

    let (second_session, second_rx) = manager
        .attach_or_create_with_context(config, None, emit)
        .expect("terminal should attach");
    assert!(Arc::ptr_eq(&first_session, &second_session));

    first_session
        .write(b"second-shared-output\n")
        .expect("write should succeed");
    assert!(
        recv_until_contains(&first_rx, "second-shared-output", Duration::from_secs(2))
            .contains("second-shared-output")
    );
    assert!(
        recv_until_contains(&second_rx, "second-shared-output", Duration::from_secs(2))
            .contains("second-shared-output")
    );

    let _ = first_session.kill();
}

#[cfg(unix)]
#[test]
fn reattach_appends_keyframe_only_for_alt_screen_session() {
    let manager = TerminalManager::new();
    let emit: EventSink = Arc::new(|_| true);
    let config = TerminalPtyConfig {
        terminal_id: Some(format!("test-altscreen-{}", Uuid::new_v4())),
        shell: Some("/bin/cat".to_string()),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };

    let (session, first_rx) = manager
        .attach_or_create_with_context(config.clone(), None, emit.clone())
        .expect("terminal should start");

    // Normal screen: a re-attach replays only the raw history; it never
    // appends the keyframe (identified by its cursor-hide repaint prefix).
    session
        .write(b"normal-line\n")
        .expect("write should succeed");
    assert!(
        recv_until_contains(&first_rx, "normal-line", Duration::from_secs(2))
            .contains("normal-line")
    );
    let (_normal_session, normal_rx) = manager
        .attach_or_create_with_context(config.clone(), None, emit.clone())
        .expect("terminal should attach");
    let normal_replay = recv_until_contains(&normal_rx, "normal-line", Duration::from_secs(2));
    assert!(normal_replay.contains("normal-line"));
    assert!(!normal_replay.contains("\x1b[?25l"));

    // Enter the alternate screen and let it apply to the live screen.
    session
        .write(b"\x1b[?1049h\x1b[2J\x1b[HALT_SCREEN_MARKER\n")
        .expect("write should succeed");
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !session.screen_snapshot().input_mode.alternate_screen {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(session.screen_snapshot().input_mode.alternate_screen);

    // Alt screen: the re-attach replay now carries the live keyframe, so the
    // current screen and its alt-screen mode are reconstructed even though
    // the alternate buffer never reached the raw history.
    let (_alt_session, alt_rx) = manager
        .attach_or_create_with_context(config, None, emit)
        .expect("terminal should attach");
    let alt_replay = recv_until_contains(&alt_rx, "\x1b[?25l", Duration::from_secs(2));
    assert!(alt_replay.contains("\x1b[?25l"));
    assert!(alt_replay.contains("\x1b[?1049h"));

    let _ = session.kill();
}

#[cfg(unix)]
#[test]
fn terminal_manager_ensures_session_before_ui_attach() {
    let manager = TerminalManager::new();
    let terminal_id = format!("test-prewarm-terminal-{}", Uuid::new_v4());
    let config = TerminalPtyConfig {
        terminal_id: Some(terminal_id.clone()),
        shell: Some("/bin/cat".to_string()),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };

    let ensured_id = manager
        .ensure_session_with_context(config.clone(), None)
        .expect("terminal should prewarm");
    assert_eq!(ensured_id, terminal_id);

    let emit: EventSink = Arc::new(|_| true);
    let (session, rx) = manager
        .attach_or_create_with_context(config, None, emit)
        .expect("terminal should attach");
    assert_eq!(session.id(), ensured_id);
    session
        .write(b"prewarm-shared-output\n")
        .expect("write should succeed");
    assert!(
        recv_until_contains(&rx, "prewarm-shared-output", Duration::from_secs(2))
            .contains("prewarm-shared-output")
    );

    let _ = session.kill();
}

#[cfg(unix)]
#[test]
fn terminal_manager_replaces_same_terminal_id_when_identity_changes() {
    let manager = TerminalManager::new();
    let emit: EventSink = Arc::new(|_| true);
    let terminal_id = format!("test-scoped-terminal-{}", Uuid::new_v4());
    let first_cwd = std::env::temp_dir().join(format!("codux-pty-first-{}", Uuid::new_v4()));
    let second_cwd = std::env::temp_dir().join(format!("codux-pty-second-{}", Uuid::new_v4()));
    fs::create_dir_all(&first_cwd).unwrap();
    fs::create_dir_all(&second_cwd).unwrap();

    let first_config = TerminalPtyConfig {
        terminal_id: Some(terminal_id.clone()),
        shell: Some("/bin/cat".to_string()),
        cwd: Some(first_cwd.display().to_string()),
        project_id: Some("worktree-a".to_string()),
        session_key: Some(format!("gpui:worktree-a:{terminal_id}")),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };
    let second_config = TerminalPtyConfig {
        cwd: Some(second_cwd.display().to_string()),
        project_id: Some("worktree-b".to_string()),
        session_key: Some(format!("gpui:worktree-b:{terminal_id}")),
        ..first_config.clone()
    };

    let (first_session, _) = manager
        .attach_or_create_with_context(first_config, None, emit.clone())
        .expect("first terminal should start");
    assert_eq!(first_session.info().cwd, first_cwd.display().to_string());
    assert_eq!(first_session.info().project_id, "worktree-a");

    let (second_session, _) = manager
        .attach_or_create_with_context(second_config, None, emit)
        .expect("second terminal should replace incompatible session");
    assert!(!Arc::ptr_eq(&first_session, &second_session));
    assert_eq!(second_session.id(), terminal_id);
    assert_eq!(second_session.info().cwd, second_cwd.display().to_string());
    assert_eq!(second_session.info().project_id, "worktree-b");

    let _ = second_session.kill();
    let _ = fs::remove_dir_all(first_cwd);
    let _ = fs::remove_dir_all(second_cwd);
}

#[cfg(unix)]
#[test]
fn terminal_manager_uses_context_session_cwd_for_identity() {
    let manager = TerminalManager::new();
    let emit: EventSink = Arc::new(|_| true);
    let terminal_id = format!("test-context-cwd-terminal-{}", Uuid::new_v4());
    let project_cwd = std::env::temp_dir().join(format!("codux-project-{}", Uuid::new_v4()));
    let worktree_cwd = std::env::temp_dir().join(format!("codux-worktree-{}", Uuid::new_v4()));
    fs::create_dir_all(&project_cwd).unwrap();
    fs::create_dir_all(&worktree_cwd).unwrap();
    let context = TerminalLaunchContext {
        root_project_id: "project-1".to_string(),
        project_id: "worktree-context".to_string(),
        project_name: "Context Worktree".to_string(),
        project_path: project_cwd.clone(),
        support_dir: std::env::temp_dir(),
        runtime_root: std::env::temp_dir(),
        terminal_id: Some(terminal_id.clone()),
        slot_id: None,
        session_key: Some(format!("gpui:worktree-context:{terminal_id}")),
        session_title: None,
        session_cwd: Some(worktree_cwd.clone()),
        session_instance_id: None,
        tool_permissions_file: None,
        memory_workspace_root: None,
        memory_prompt_file: None,
        memory_index_file: None,
        host_device_id: None,
    };
    let config = TerminalPtyConfig {
        terminal_id: Some(terminal_id),
        shell: Some("/bin/cat".to_string()),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };

    let (session, _) = manager
        .attach_or_create_with_context(config, Some(&context), emit)
        .expect("terminal should use context session cwd");

    assert_eq!(session.info().cwd, worktree_cwd.display().to_string());
    assert_eq!(session.info().project_id, "worktree-context");

    let _ = session.kill();
    let _ = fs::remove_dir_all(project_cwd);
    let _ = fs::remove_dir_all(worktree_cwd);
}

#[test]
fn terminal_event_subscribers_are_pruned_when_sink_is_closed() {
    let subscribers: Arc<parking_lot::Mutex<Vec<EventSubscriber>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    subscribers
        .lock()
        .push(EventSubscriber::anonymous(Arc::new(move |_| {
            tx.send(()).is_ok()
        })));

    emit_terminal_event(
        &subscribers,
        TerminalEvent::Exit {
            session_id: "session-a".to_string(),
            exit_code: None,
        },
    );
    assert_eq!(subscribers.lock().len(), 1);
    drop(rx);

    emit_terminal_event(
        &subscribers,
        TerminalEvent::Exit {
            session_id: "session-a".to_string(),
            exit_code: None,
        },
    );
    assert!(subscribers.lock().is_empty());
}

#[test]
fn keyed_terminal_event_subscribers_replace_stale_sinks() {
    let subscribers: Arc<parking_lot::Mutex<Vec<EventSubscriber>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let anonymous_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stale_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let latest_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let other_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    {
        let anonymous_hits = anonymous_hits.clone();
        subscribers
            .lock()
            .push(EventSubscriber::anonymous(Arc::new(move |_| {
                anonymous_hits.fetch_add(1, Ordering::SeqCst);
                true
            })));
    }
    {
        let stale_hits = stale_hits.clone();
        insert_keyed_event_subscriber(
            &subscribers,
            "remote-terminal:session-1".to_string(),
            Arc::new(move |_| {
                stale_hits.fetch_add(1, Ordering::SeqCst);
                true
            }),
        );
    }
    {
        let latest_hits = latest_hits.clone();
        insert_keyed_event_subscriber(
            &subscribers,
            "remote-terminal:session-1".to_string(),
            Arc::new(move |_| {
                latest_hits.fetch_add(1, Ordering::SeqCst);
                true
            }),
        );
    }
    {
        let other_hits = other_hits.clone();
        insert_keyed_event_subscriber(
            &subscribers,
            "remote-terminal:session-2".to_string(),
            Arc::new(move |_| {
                other_hits.fetch_add(1, Ordering::SeqCst);
                true
            }),
        );
    }

    emit_terminal_event(
        &subscribers,
        TerminalEvent::Output {
            session_id: "session-1".to_string(),
            text: "hello".to_string(),
            bytes: b"hello".to_vec(),
        },
    );

    assert_eq!(anonymous_hits.load(Ordering::SeqCst), 1);
    assert_eq!(stale_hits.load(Ordering::SeqCst), 0);
    assert_eq!(latest_hits.load(Ordering::SeqCst), 1);
    assert_eq!(other_hits.load(Ordering::SeqCst), 1);
    assert_eq!(subscribers.lock().len(), 3);
}

#[cfg(unix)]
#[test]
fn terminal_manager_registers_ai_runtime_terminal_lifecycle() {
    let dir = std::env::temp_dir().join(format!("codux-terminal-bridge-{}", Uuid::new_v4()));
    let bridge = Arc::new(AIRuntimeBridge::with_paths(
        dir.join("root"),
        dir.join("temp"),
        dir.join("home"),
    ));
    let manager = TerminalManager::with_ai_runtime(Arc::clone(&bridge));
    let terminal_id = format!("test-ai-terminal-{}", Uuid::new_v4());
    let config = TerminalPtyConfig {
        terminal_id: Some(terminal_id.clone()),
        project_id: Some("project-1".to_string()),
        slot_id: Some("slot-1".to_string()),
        session_key: Some("session-key-1".to_string()),
        title: Some("Codex".to_string()),
        tool: Some("codex".to_string()),
        shell: Some("/bin/cat".to_string()),
        cols: Some(80),
        rows: Some(24),
        scrollback_lines: Some(100),
        ..Default::default()
    };
    let emit: EventSink = Arc::new(|_| true);

    let (session, _) = manager
        .attach_or_create_with_context(config, None, emit)
        .expect("terminal should start");

    let terminals = bridge.registry().snapshot();
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].terminal_id, terminal_id);
    assert_eq!(terminals[0].project_id, "project-1");
    assert_eq!(terminals[0].slot_id, "slot-1");
    assert_eq!(terminals[0].tool.as_deref(), Some("codex"));

    manager.kill(session.id()).expect("terminal should stop");
    assert!(bridge.registry().snapshot().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

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

#[cfg(unix)]
fn recv_until_contains(rx: &flume::Receiver<Vec<u8>>, needle: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    let mut text = String::new();
    while Instant::now() < deadline && !text.contains(needle) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(bytes) => text.push_str(&String::from_utf8_lossy(&bytes)),
            Err(flume::RecvTimeoutError::Timeout) => {}
            Err(flume::RecvTimeoutError::Disconnected) => break,
        }
    }
    text
}

#[cfg(unix)]
fn wait_for_session_state(
    bridge: &AIRuntimeBridge,
    terminal_id: &str,
    state: &str,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if bridge
            .runtime_state_snapshot()
            .sessions
            .iter()
            .any(|session| session.terminal_id == terminal_id && session.state == state)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "terminal {terminal_id} did not reach state {state}; snapshot={:?}",
        bridge.runtime_state_snapshot().sessions
    );
}
