use super::*;
use crate::ai_runtime::{
    AIHookEventMetadata, AIProjectPhase, binding::AIRuntimeBinding,
    constants::CODEX_STALE_PRELAUNCH_OPEN_TURN_SOURCE,
};
use std::fs;
use uuid::Uuid;

#[test]
fn hook_lifecycle_tracks_running_and_completion() {
    let store = AIRuntimeStateStore::default();
    let start = store.apply_hook(test_hook("promptSubmitted", 1000.0));
    assert!(start.did_change);
    assert!(start.completion.is_none());

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.sessions[0].state, "responding");

    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        total_tokens: Some(150),
        updated_at: 1010.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook("turnCompleted", 1010.0)
    });

    assert!(complete.did_change);
    assert!(complete.completion.is_some());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.completion_count, 1);
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Completed { .. }
    ));
}

#[test]
fn workspace_placeholder_project_name_falls_back_to_project_path_for_completion() {
    let store = AIRuntimeStateStore::default();
    let mut prompt = test_hook("promptSubmitted", 1000.0);
    prompt.project_name = "Workspace".to_string();
    prompt.project_path = Some("/tmp/codux-gpui".to_string());
    assert!(store.apply_hook(prompt).did_change);

    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        project_name: "Workspace".to_string(),
        project_path: Some("/tmp/codux-gpui".to_string()),
        total_tokens: Some(150),
        updated_at: 1010.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook("turnCompleted", 1010.0)
    });

    assert_eq!(
        complete.completion.expect("completion").project_name,
        "codux-gpui"
    );
}

#[test]
fn merged_mutation_preserves_multiple_completion_events() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(AIHookEventPayload {
                project_id: "project-a".to_string(),
                ..test_hook_for("codex", "terminal-a", "session-a", 1000.0)
            })
            .did_change
    );
    assert!(
        store
            .apply_hook(AIHookEventPayload {
                project_id: "project-b".to_string(),
                ..test_hook_for("codex", "terminal-b", "session-b", 1001.0)
            })
            .did_change
    );

    let first = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        project_id: "project-a".to_string(),
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-a", "session-a", 1010.0)
    });
    let second = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        project_id: "project-b".to_string(),
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-b", "session-b", 1011.0)
    });

    let mut merged = AIRuntimeStateMutation::default();
    merged.merge(first);
    merged.merge(second);

    assert_eq!(merged.completions.len(), 2);
    let projects = merged
        .completions
        .iter()
        .filter_map(|event| {
            event
                .session
                .as_ref()
                .map(|session| session.project_id.as_str())
        })
        .collect::<std::collections::HashSet<_>>();
    assert!(projects.contains("project-a"));
    assert!(projects.contains("project-b"));
}

#[test]
fn runtime_snapshot_sets_restored_session_baseline() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: None,
            model: Some("gpt-5.5".to_string()),
            assistant_preview: None,
            input_tokens: 1_000,
            output_tokens: 200,
            cached_input_tokens: 3_000,
            total_tokens: 1_200,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1005.0,
            started_at: Some(900.0),
            completed_at: None,
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "restored".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.baseline_total_tokens, 1_200);
    assert_eq!(session.baseline_cached_input_tokens, 3_000);
    assert!(session.baseline_resolved);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds())
            .cached_input_tokens,
        0
    );
}

#[test]
fn tool_activity_without_loading_is_ignored() {
    let store = AIRuntimeStateStore::default();
    let mut event = test_hook("promptSubmitted", 1000.0);
    event.metadata = Some(AIHookEventMetadata {
        source: Some("tool-use".to_string()),
        ..empty_metadata()
    });

    let mutation = store.apply_hook(event);

    assert!(!mutation.did_change);
    assert!(store.snapshot().sessions.is_empty());
}

#[test]
fn codewhale_hook_is_tracked_as_runtime_session() {
    let store = AIRuntimeStateStore::default();
    let mutation = store.apply_hook(test_hook_for(
        "codewhale",
        "codewhale-term-1",
        "codewhale-session-1",
        1000.0,
    ));

    assert!(mutation.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.sessions[0].tool, "codewhale");
    assert_eq!(snapshot.sessions[0].terminal_id, "codewhale-term-1");
    assert_eq!(
        snapshot.sessions[0].ai_session_id.as_deref(),
        Some("codewhale-session-1")
    );
}

#[test]
fn detected_codewhale_terminal_is_canonicalized_not_filtered() {
    let terminal = AIRuntimeTerminalState {
        terminal_id: "codewhale-term-1".to_string(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "CodeWhale".to_string(),
        cwd: "/tmp/codewhale-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
        terminal_instance_id: Some("instance-1".to_string()),
    };

    let session = detected_terminal_session(&terminal, "codewhale", 1000.0).expect("session");

    assert_eq!(session.tool, "codewhale");
    assert_eq!(session.terminal_id, "codewhale-term-1");
    assert_eq!(session.project_name, "codewhale-project");
    assert_eq!(session.state, "idle");
    assert!(session.ai_session_id.is_none());
}

#[test]
fn screen_signal_does_not_start_codewhale_detected_turn() {
    let store = AIRuntimeStateStore::default();
    let terminal = AIRuntimeTerminalState {
        terminal_id: "codewhale-term-1".to_string(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "CodeWhale".to_string(),
        cwd: "/tmp/codewhale-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
        terminal_instance_id: Some("instance-1".to_string()),
    };
    let detected = std::collections::HashMap::from([(
        "codewhale-term-1".to_string(),
        "codewhale".to_string(),
    )]);
    assert!(
        store
            .ensure_detected_sessions(&[terminal], &detected, 1000.0)
            .did_change
    );
    assert_eq!(store.snapshot().sessions[0].state, "idle");

    let mutation = store.apply_screen_signal("codewhale-term-1", ScreenSignal::Running);

    assert!(!mutation.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions[0].tool, "codewhale");
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert_eq!(snapshot.running_count, 0);
}

#[test]
fn process_liveness_retires_undetected_hookless_sessions() {
    let store = AIRuntimeStateStore::default();
    let terminal = AIRuntimeTerminalState {
        terminal_id: "kiro-term-1".to_string(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Kiro".to_string(),
        cwd: "/tmp/kiro-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
        terminal_instance_id: Some("instance-1".to_string()),
    };
    assert!(
        store
            .apply_hook(test_hook_for(
                "kiro",
                "kiro-term-1",
                "kiro-session-1",
                1000.0
            ))
            .did_change
    );
    assert_eq!(store.snapshot().sessions[0].state, "responding");

    let shell_pids = vec![("kiro-term-1".to_string(), 1234)];
    let empty_detected = std::collections::HashMap::new();
    let first = store.retire_undetected_hookless_sessions(
        &[terminal.clone()],
        &shell_pids,
        &empty_detected,
        1007.0,
    );

    assert!(first.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(!snapshot.sessions[0].has_completed_turn);
    assert!(!snapshot.sessions[0].was_interrupted);

    let second = store.retire_undetected_hookless_sessions(
        &[terminal],
        &shell_pids,
        &empty_detected,
        1008.0,
    );

    assert!(second.did_change);
    assert!(store.snapshot().sessions.is_empty());
}

#[test]
fn process_liveness_does_not_retire_hook_driven_tools() {
    let store = AIRuntimeStateStore::default();
    let terminal = AIRuntimeTerminalState {
        terminal_id: "codex-term-1".to_string(),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Codex".to_string(),
        cwd: "/tmp/codex-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
        terminal_instance_id: Some("instance-1".to_string()),
    };
    assert!(
        store
            .apply_hook(test_hook_for("codex", "codex-term-1", "session-1", 1000.0))
            .did_change
    );

    let mutation = store.retire_undetected_hookless_sessions(
        &[terminal],
        &[("codex-term-1".to_string(), 1234)],
        &std::collections::HashMap::new(),
        1010.0,
    );

    assert!(!mutation.did_change);
    assert_eq!(store.snapshot().sessions[0].state, "responding");
}

#[test]
fn codewhale_completion_merges_realtime_probe_snapshot() {
    let root = std::env::temp_dir().join(format!("codux-codewhale-store-probe-{}", Uuid::new_v4()));
    let project = root.join("project");
    let session_dir = root.join(".codewhale").join("sessions");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&session_dir).unwrap();
    let session_file = session_dir.join("session-1.json");
    fs::write(
        &session_file,
        format!(
            r#"{{
                "metadata": {{
                    "id": "session-1",
                    "workspace": "{}",
                    "model": "deepseek-chat",
                    "total_tokens": 789,
                    "created_at": "2026-06-06T01:00:00Z",
                    "updated_at": "2026-06-06T01:01:00Z"
                }},
                "messages": [
                    {{ "role": "assistant", "content": "done" }}
                ]
            }}"#,
            project.display()
        ),
    )
    .unwrap();
    let store = AIRuntimeStateStore::default();
    let mut prompt = test_hook_for("codewhale", "codewhale-term-1", "session-1", 1000.0);
    prompt.project_path = Some(project.display().to_string());
    prompt.model = None;
    assert!(store.apply_hook(prompt).did_change);

    let mut complete = test_hook_for("codewhale", "codewhale-term-1", "session-1", 1010.0);
    complete.kind = "turnCompleted".to_string();
    complete.project_path = Some(project.display().to_string());
    complete.model = None;
    complete.metadata = Some(AIHookEventMetadata {
        transcript_path: Some(session_file.display().to_string()),
        has_completed_turn: Some(true),
        ..empty_metadata()
    });
    assert!(store.apply_hook(complete).did_change);

    let snapshot = store.snapshot();
    let session = snapshot
        .sessions
        .iter()
        .find(|session| session.terminal_id == "codewhale-term-1")
        .unwrap();
    assert_eq!(session.tool, "codewhale");
    assert_eq!(session.model.as_deref(), Some("deepseek-chat"));
    assert_eq!(session.total_tokens, 789);
    assert_eq!(session.state, "idle");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn codewhale_interrupted_turn_end_clears_loading() {
    let store = AIRuntimeStateStore::default();
    let prompt = test_hook_for("codewhale", "codewhale-term-1", "session-1", 1000.0);
    assert!(store.apply_hook(prompt).did_change);
    assert_eq!(store.snapshot().sessions[0].state, "responding");

    let mut interrupted = test_hook_for("codewhale", "codewhale-term-1", "session-1", 1010.0);
    interrupted.kind = "turnCompleted".to_string();
    interrupted.metadata = Some(AIHookEventMetadata {
        was_interrupted: Some(true),
        has_completed_turn: Some(false),
        reason: Some("interrupted".to_string()),
        ..empty_metadata()
    });
    assert!(store.apply_hook(interrupted).did_change);

    let snapshot = store.snapshot();
    let session = &snapshot.sessions[0];
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(session.state, "idle");
    assert!(!session.is_running);
    assert!(session.was_interrupted);
    assert!(!session.has_completed_turn);
}

#[test]
fn codewhale_interrupted_turn_end_is_authoritative_over_responding_probe() {
    let previous = AISessionSnapshot {
        tool: "codewhale".to_string(),
        terminal_id: "codewhale-term-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        project_name: "Project".to_string(),
        project_path: Some("/tmp/codewhale-project".to_string()),
        session_title: "CodeWhale".to_string(),
        ai_session_id: Some("session-1".to_string()),
        model: Some("deepseek-v4-flash".to_string()),
        state: "responding".to_string(),
        status: "running".to_string(),
        is_running: true,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_tokens: 51_562,
        baseline_total_tokens: 51_562,
        baseline_cached_input_tokens: 0,
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        baseline_resolved: true,
        started_at: Some(1000.0),
        updated_at: 1000.0,
        active_turn_started_at: Some(1000.0),
        runtime_turn_started_at: None,
        completed_turn_started_at: None,
        has_completed_turn: false,
        was_interrupted: false,
        transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
        notification_type: None,
        target_tool_name: None,
        message: None,
        latest_assistant_preview: None,
        plan: None,
    };
    let resolved = merge_snapshot_into_hook(
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            tool: "codewhale".to_string(),
            terminal_id: "codewhale-term-1".to_string(),
            terminal_instance_id: Some("instance-1".to_string()),
            project_id: "project-1".to_string(),
            project_name: "Project".to_string(),
            project_path: Some("/tmp/codewhale-project".to_string()),
            session_title: "CodeWhale".to_string(),
            ai_session_id: Some("session-1".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: Some(51_562),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                was_interrupted: Some(true),
                has_completed_turn: Some(false),
                reason: Some("interrupted".to_string()),
                source: Some("codewhale-lifecycle".to_string()),
                ..empty_metadata()
            }),
        },
        AIRuntimeContextSnapshot {
            tool: "codewhale".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 51_562,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1011.0,
            started_at: Some(990.0),
            completed_at: None,
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "fresh".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
        Some(&previous),
    );

    assert_eq!(resolved.kind, "turnCompleted");
    assert_eq!(
        resolved
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.was_interrupted),
        Some(true)
    );
    assert_eq!(
        resolved
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.has_completed_turn),
        Some(false)
    );
}

#[test]
fn codewhale_lifecycle_turn_end_is_authoritative_over_responding_probe() {
    let previous = AISessionSnapshot {
        tool: "codewhale".to_string(),
        terminal_id: "codewhale-term-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        project_name: "Project".to_string(),
        project_path: Some("/tmp/codewhale-project".to_string()),
        session_title: "CodeWhale".to_string(),
        ai_session_id: Some("session-1".to_string()),
        model: Some("deepseek-v4-flash".to_string()),
        state: "responding".to_string(),
        status: "running".to_string(),
        is_running: true,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_tokens: 51_562,
        baseline_total_tokens: 51_562,
        baseline_cached_input_tokens: 0,
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        baseline_resolved: true,
        started_at: Some(1000.0),
        updated_at: 1000.0,
        active_turn_started_at: Some(1000.0),
        runtime_turn_started_at: None,
        completed_turn_started_at: None,
        has_completed_turn: false,
        was_interrupted: false,
        transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
        notification_type: None,
        target_tool_name: None,
        message: None,
        latest_assistant_preview: None,
        plan: None,
    };
    let resolved = merge_snapshot_into_hook(
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            tool: "codewhale".to_string(),
            terminal_id: "codewhale-term-1".to_string(),
            terminal_instance_id: Some("instance-1".to_string()),
            project_id: "project-1".to_string(),
            project_name: "Project".to_string(),
            project_path: Some("/tmp/codewhale-project".to_string()),
            session_title: "CodeWhale".to_string(),
            ai_session_id: Some("session-1".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: Some(51_562),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                was_interrupted: Some(false),
                has_completed_turn: Some(true),
                reason: Some("unknown".to_string()),
                source: Some("codewhale-lifecycle".to_string()),
                ..empty_metadata()
            }),
        },
        AIRuntimeContextSnapshot {
            tool: "codewhale".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 51_562,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1011.0,
            started_at: Some(990.0),
            completed_at: None,
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "fresh".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
        Some(&previous),
    );

    assert_eq!(resolved.kind, "turnCompleted");
    assert_eq!(
        resolved
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.was_interrupted),
        Some(false)
    );
    assert_eq!(
        resolved
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.has_completed_turn),
        Some(true)
    );
}

#[test]
fn codewhale_interrupted_turn_end_clears_loading_when_session_file_still_looks_responding() {
    let root = std::env::temp_dir().join(format!(
        "codux-codewhale-interrupt-probe-{}",
        Uuid::new_v4()
    ));
    let project = root.join("project");
    let session_dir = root.join(".codewhale").join("sessions");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&session_dir).unwrap();
    let session_file = session_dir.join("session-1.json");
    fs::write(
        &session_file,
        format!(
            r#"{{
                "metadata": {{
                    "id": "session-1",
                    "workspace": "{}",
                    "model": "deepseek-v4-flash",
                    "total_tokens": 51562,
                    "created_at": "2026-06-29T05:00:00Z",
                    "updated_at": "2026-06-29T05:04:40Z"
                }},
                "messages": [
                    {{ "role": "user", "content": "interrupt me" }}
                ]
            }}"#,
            project.display()
        ),
    )
    .unwrap();

    let store = AIRuntimeStateStore::default();
    let mut prompt = test_hook_for(
        "codewhale",
        "codewhale-term-1",
        "session-1",
        1_782_630_000.0,
    );
    prompt.project_path = Some(project.display().to_string());
    prompt.total_tokens = Some(51_562);
    assert!(store.apply_hook(prompt).did_change);

    let mut interrupted = test_hook_for(
        "codewhale",
        "codewhale-term-1",
        "session-1",
        1_782_630_010.0,
    );
    interrupted.kind = "turnCompleted".to_string();
    interrupted.project_path = Some(project.display().to_string());
    interrupted.total_tokens = Some(51_562);
    interrupted.metadata = Some(AIHookEventMetadata {
        transcript_path: Some(session_file.display().to_string()),
        was_interrupted: Some(true),
        has_completed_turn: Some(false),
        reason: Some("interrupted".to_string()),
        source: Some("codewhale-lifecycle".to_string()),
        ..empty_metadata()
    });
    assert!(store.apply_hook(interrupted).did_change);

    let snapshot = store.snapshot();
    let session = &snapshot.sessions[0];
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(session.state, "idle");
    assert!(!session.is_running);
    assert!(session.was_interrupted);
    assert!(!session.has_completed_turn);

    let stale_probe = store.apply_runtime_snapshot(
        "codewhale-term-1",
        AIRuntimeContextSnapshot {
            tool: "codewhale".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some(session_file.display().to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 51_562,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1_782_630_020.0,
            started_at: Some(1_782_630_000.0),
            completed_at: None,
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "fresh".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
    );
    assert!(!stale_probe.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(snapshot.sessions[0].was_interrupted);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn codewhale_prompt_with_existing_total_starts_current_usage_at_zero() {
    let mut core = AIRuntimeStateCore::default();
    let prompt = AIHookEventPayload {
        total_tokens: Some(51_562),
        ..test_hook_for("codewhale", "codewhale-term-1", "session-1", 1000.0)
    };

    assert!(apply_hook_unlocked(&mut core, prompt));

    let session = core.sessions.get("codewhale-term-1").unwrap();
    assert_eq!(session.total_tokens, 51_562);
    assert_eq!(session.baseline_total_tokens, 51_562);
    assert!(session.baseline_resolved);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );
}

#[test]
fn codewhale_restored_prompt_keeps_existing_total_as_baseline() {
    let previous = AISessionSnapshot {
        tool: "codewhale".to_string(),
        terminal_id: "codewhale-term-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        project_name: "Project".to_string(),
        project_path: Some("/tmp/codewhale-project".to_string()),
        session_title: "CodeWhale".to_string(),
        ai_session_id: Some("session-1".to_string()),
        model: Some("deepseek-v4-flash".to_string()),
        state: "idle".to_string(),
        status: "idle".to_string(),
        is_running: false,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_tokens: 51_562,
        baseline_total_tokens: 51_562,
        baseline_cached_input_tokens: 0,
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        baseline_resolved: true,
        started_at: Some(1000.0),
        updated_at: 1000.0,
        active_turn_started_at: None,
        runtime_turn_started_at: None,
        completed_turn_started_at: None,
        has_completed_turn: true,
        was_interrupted: false,
        transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
        notification_type: None,
        target_tool_name: None,
        message: None,
        latest_assistant_preview: None,
        plan: None,
    };

    let resolved = merge_snapshot_into_hook(
        AIHookEventPayload {
            kind: "promptSubmitted".to_string(),
            tool: "codewhale".to_string(),
            terminal_id: "codewhale-term-1".to_string(),
            terminal_instance_id: Some("instance-1".to_string()),
            project_id: "project-1".to_string(),
            project_name: "Project".to_string(),
            project_path: Some("/tmp/codewhale-project".to_string()),
            session_title: "CodeWhale".to_string(),
            ai_session_id: Some("session-1".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: 1010.0,
            metadata: None,
        },
        AIRuntimeContextSnapshot {
            tool: "codewhale".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codewhale-project/session-1.json".to_string()),
            model: Some("deepseek-v4-flash".to_string()),
            assistant_preview: Some("done".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 132_786,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1020.0,
            started_at: Some(900.0),
            completed_at: Some(1020.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "restored".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
        Some(&previous),
    );

    assert_eq!(resolved.kind, "promptSubmitted");
    assert_eq!(resolved.total_tokens, Some(51_562));
}

#[test]
fn codewhale_restored_session_prompt_resets_current_usage_baseline() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            total_tokens: Some(276_000),
            ..test_hook_for("codewhale", "codewhale-term-1", "session-1", 1000.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            total_tokens: Some(276_000),
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook_for("codewhale", "codewhale-term-1", "session-1", 1010.0)
        }
    ));
    let session = core.sessions.get("codewhale-term-1").unwrap();
    assert_eq!(session.baseline_total_tokens, 276_000);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );

    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "promptSubmitted".to_string(),
            total_tokens: Some(276_000),
            ..test_hook_for("codewhale", "codewhale-term-1", "session-1", 2000.0)
        }
    ));

    let session = core.sessions.get("codewhale-term-1").unwrap();
    assert_eq!(session.state, "responding");
    assert_eq!(session.total_tokens, 276_000);
    assert_eq!(session.baseline_total_tokens, 276_000);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );

    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            total_tokens: Some(303_783),
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook_for("codewhale", "codewhale-term-1", "session-1", 2010.0)
        }
    ));

    let session = core.sessions.get("codewhale-term-1").unwrap();
    assert_eq!(session.total_tokens, 303_783);
    assert_eq!(session.baseline_total_tokens, 276_000);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        27_783
    );
}

#[test]
fn codex_stale_completed_turn_after_new_prompt_stays_running() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook("turnCompleted", 1010.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1020.0)
    ));
    let previous = core.sessions.get("terminal-1").cloned().unwrap();

    let resolved = merge_snapshot_into_hook(
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1021.0,
            metadata: Some(AIHookEventMetadata {
                transcript_path: Some("/tmp/codex.jsonl".to_string()),
                ..empty_metadata()
            }),
            ..test_hook("turnCompleted", 1021.0)
        },
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 150,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1010.0,
            started_at: Some(1000.0),
            completed_at: Some(1010.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
        Some(&previous),
    );

    assert_eq!(resolved.kind, "promptSubmitted");
    assert_eq!(
        resolved
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.has_completed_turn),
        Some(false)
    );
    assert!(apply_hook_unlocked(&mut core, resolved));
    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.state, "responding");
    assert!(session.has_completed_turn);
    assert!(matches!(
        completed_phase_unlocked(&core, "project-1", now_seconds()),
        AIProjectPhase::Idle
    ));
}

#[test]
fn stale_session_started_does_not_override_running_prompt() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));

    assert!(!apply_hook_unlocked(
        &mut core,
        test_hook("sessionStarted", 999.0)
    ));

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.state, "responding");
    assert_eq!(session.updated_at, 1000.0);
}

#[test]
fn session_started_clears_previous_completion_flag() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook("turnCompleted", 1010.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("sessionStarted", 1020.0)
    ));

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.state, "idle");
    assert!(!session.has_completed_turn);
    assert!(matches!(
        completed_phase_unlocked(&core, "project-1", now_seconds()),
        AIProjectPhase::Idle
    ));
}

#[test]
fn stale_needs_input_is_not_visible_in_snapshot() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "needsInput".to_string(),
            updated_at: 1.0,
            metadata: Some(AIHookEventMetadata {
                notification_type: Some("permission-request".to_string()),
                target_tool_name: Some("AskUserQuestion".to_string()),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", 1.0)
        }
    ));

    let snapshot = state_snapshot_unlocked(&core);

    assert_eq!(snapshot.needs_input_count, 0);
    assert_eq!(snapshot.global_totals.needs_input, 0);
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(snapshot.sessions[0].notification_type.is_none());
    assert!(matches!(
        snapshot.projects[0].project_phase,
        AIProjectPhase::Idle
    ));
}

#[test]
fn fresh_needs_input_remains_visible_in_snapshot() {
    let now = now_seconds();
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "needsInput".to_string(),
            updated_at: now,
            metadata: Some(AIHookEventMetadata {
                notification_type: Some("permission-request".to_string()),
                target_tool_name: Some("AskUserQuestion".to_string()),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", now)
        }
    ));

    let snapshot = state_snapshot_unlocked(&core);

    assert_eq!(snapshot.needs_input_count, 1);
    assert_eq!(snapshot.global_totals.needs_input, 1);
    assert_eq!(snapshot.sessions[0].state, "needsInput");
    assert!(matches!(
        snapshot.projects[0].project_phase,
        AIProjectPhase::NeedsInput { .. }
    ));
}

#[test]
fn prompt_submitted_after_needs_input_restores_running_state() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "needsInput".to_string(),
            updated_at: 1000.0,
            metadata: Some(AIHookEventMetadata {
                notification_type: Some("permission-request".to_string()),
                target_tool_name: Some("AskUserQuestion".to_string()),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", 1000.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "promptSubmitted".to_string(),
            updated_at: 1001.0,
            metadata: Some(AIHookEventMetadata {
                source: Some("permission-auto-allowed".to_string()),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", 1001.0)
        }
    ));

    let snapshot = state_snapshot_unlocked(&core);

    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.needs_input_count, 0);
    assert_eq!(snapshot.sessions[0].state, "responding");
    assert!(snapshot.sessions[0].notification_type.is_none());
}

#[test]
fn claude_needs_input_clears_when_probe_sees_resume_after_completed_turn() {
    // Repro of the desktop pet sticking on "等待允许" after a manual permission
    // approval: a prior turn completed (has_completed_turn=true), the user sends
    // a new prompt, Claude asks for permission (needsInput). On approval no hook
    // fires (Claude has no "granted" hook), so the 5s probe is what must clear
    // it once Claude resumes responding.
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook_for("claude", "claude-term-1", "claude-external-1", 1000.0)
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", 1010.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook_for("claude", "claude-term-1", "claude-external-1", 1020.0)
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "needsInput".to_string(),
            updated_at: 1025.0,
            metadata: Some(AIHookEventMetadata {
                notification_type: Some("permission-request".to_string()),
                target_tool_name: Some("Skill".to_string()),
                ..empty_metadata()
            }),
            ..test_hook_for("claude", "claude-term-1", "claude-external-1", 1025.0)
        }
    ));
    assert_eq!(
        core.sessions.get("claude-term-1").unwrap().state,
        "needsInput"
    );

    // User approves; Claude resumes. The probe reads the log: the turn is still
    // responding (last user prompt newer than last completion) and new output
    // advanced `updated_at`.
    apply_runtime_snapshot_unlocked(
        &mut core,
        "claude-term-1",
        AIRuntimeContextSnapshot {
            tool: "claude".to_string(),
            external_session_id: Some("claude-external-1".to_string()),
            transcript_path: None,
            model: Some("sonnet".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 200,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1030.0,
            // Log's user-message time sits slightly before the hook's prompt
            // wall-clock time (real-world skew between the two clocks).
            started_at: Some(1018.0),
            completed_at: Some(1010.0),
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
    );

    assert_eq!(
        core.sessions.get("claude-term-1").unwrap().state,
        "responding",
        "needsInput must clear once the probe sees the turn resume after approval"
    );
}

#[test]
fn probe_derived_needs_input_marks_and_clears_permission_wait() {
    // The pure-file path with NO needsInput hook: the probe alone detects a tool
    // call blocked on the user (response_state="needsInput"), the store surfaces
    // it, then recovers to responding on approval and resolves to idle on
    // completion -- the full lifecycle without a single hook event.
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook_for("claude", "claude-term-1", "claude-external-1", 1000.0)
    ));
    assert_eq!(
        core.sessions.get("claude-term-1").unwrap().state,
        "responding"
    );

    // Probe sees a pending tool call past the idle gap.
    apply_runtime_snapshot_unlocked(
        &mut core,
        "claude-term-1",
        needs_input_probe_snapshot(1005.0),
    );
    assert_eq!(
        core.sessions.get("claude-term-1").unwrap().state,
        "needsInput"
    );

    // Approval -> the probe sees the turn resume (no "granted" hook exists).
    apply_runtime_snapshot_unlocked(
        &mut core,
        "claude-term-1",
        AIRuntimeContextSnapshot {
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1010.0,
            response_state: Some("responding".to_string()),
            ..needs_input_probe_snapshot(1010.0)
        },
    );
    assert_eq!(
        core.sessions.get("claude-term-1").unwrap().state,
        "responding"
    );

    // Turn completes -> idle.
    apply_runtime_snapshot_unlocked(
        &mut core,
        "claude-term-1",
        AIRuntimeContextSnapshot {
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1020.0,
            completed_at: Some(1019.0),
            response_state: Some("idle".to_string()),
            has_completed_turn: true,
            ..needs_input_probe_snapshot(1020.0)
        },
    );
    assert_eq!(core.sessions.get("claude-term-1").unwrap().state, "idle");
}

#[test]
fn screen_signal_refines_active_turn_only() {
    // The universal screen-scrape path: it only refines an already-active turn
    // between responding<->needsInput, and never touches an idle session.
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook_for("codex", "term-s", "ext-s", 1000.0)
    ));
    assert_eq!(core.sessions.get("term-s").unwrap().state, "responding");

    // Screen shows an approval prompt -> needsInput.
    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-s",
        ScreenSignal::Waiting,
        false
    ));
    assert_eq!(core.sessions.get("term-s").unwrap().state, "needsInput");

    // Prompt cleared / work resumed -> back to responding.
    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-s",
        ScreenSignal::Running,
        false
    ));
    assert_eq!(core.sessions.get("term-s").unwrap().state, "responding");

    // Unknown is always a no-op.
    assert!(!apply_screen_signal_unlocked(
        &mut core,
        "term-s",
        ScreenSignal::Unknown,
        false
    ));

    // A completed/idle session is never started by a screen prompt.
    apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                has_completed_turn: Some(true),
                ..empty_metadata()
            }),
            ..test_hook_for("codex", "term-s", "ext-s", 1010.0)
        },
    );
    assert_eq!(core.sessions.get("term-s").unwrap().state, "idle");
    assert!(!apply_screen_signal_unlocked(
        &mut core,
        "term-s",
        ScreenSignal::Waiting,
        false
    ));
    assert_eq!(core.sessions.get("term-s").unwrap().state, "idle");
}

#[test]
fn screen_signal_starts_kiro_turn_because_files_are_completion_delayed() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro", "", 1000.0)
        }
    ));
    assert_eq!(core.sessions.get("term-kiro").unwrap().state, "idle");

    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-kiro",
        ScreenSignal::Running,
        true
    ));

    let session = core.sessions.get("term-kiro").unwrap();
    assert_eq!(session.state, "responding");
    assert!(session.is_running);
    assert!(!session.has_completed_turn);
    assert!(session.runtime_turn_started_at.is_some());
}

#[test]
fn screen_signal_starts_next_kiro_turn_after_completed_turn() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro-next", "", 1000.0)
        }
    ));
    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-next",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("kiro-session-1".to_string()),
            transcript_path: Some("/tmp/kiro-session-1.json".to_string()),
            model: Some("auto".to_string()),
            assistant_preview: Some("done".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.03,
            }],
            baseline_usage_amounts: Vec::new(),
            updated_at: 1010.0,
            started_at: Some(1005.0),
            completed_at: Some(1010.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));
    let session = core.sessions.get("term-kiro-next").unwrap();
    assert_eq!(session.state, "idle");
    assert!(session.has_completed_turn);
    assert!(should_poll_runtime_session(session, "interval", 1015.0));
    assert!(!should_poll_runtime_session(session, "interval", 2000.0));

    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-kiro-next",
        ScreenSignal::Running,
        true
    ));

    let session = core.sessions.get("term-kiro-next").unwrap();
    assert_eq!(session.state, "responding");
    assert!(session.is_running);
    assert!(!session.has_completed_turn);
    assert_eq!(session.usage_amounts[0].unit, "credit");
}

#[test]
fn kiro_completion_snapshot_clears_screen_started_turn_without_timestamp_gate() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro-complete", "", 1000.0)
        }
    ));
    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-kiro-complete",
        ScreenSignal::Running,
        true
    ));
    assert_eq!(
        core.sessions.get("term-kiro-complete").unwrap().state,
        "responding"
    );

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-complete",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("kiro-session-complete".to_string()),
            transcript_path: Some("/tmp/kiro-session-complete.json".to_string()),
            model: Some("auto".to_string()),
            assistant_preview: Some("done".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.05,
            }],
            baseline_usage_amounts: Vec::new(),
            updated_at: 1000.0,
            started_at: Some(999.0),
            completed_at: Some(1000.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let session = core.sessions.get("term-kiro-complete").unwrap();
    assert_eq!(session.state, "idle");
    assert!(!session.is_running);
    assert!(session.has_completed_turn);
}

#[test]
fn kiro_fast_completion_after_detection_keeps_current_credit_usage() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro-fast", "", 1000.0)
        }
    ));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-fast",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("kiro-session-fast".to_string()),
            transcript_path: Some("/tmp/kiro-session-fast.json".to_string()),
            model: Some("auto".to_string()),
            assistant_preview: Some("done".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.041,
            }],
            baseline_usage_amounts: Vec::new(),
            updated_at: 1006.0,
            started_at: Some(999.0),
            completed_at: Some(1006.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "unknown".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let session = core.sessions.get("term-kiro-fast").unwrap();
    assert_eq!(session.state, "idle");
    assert!(session.baseline_usage_amounts.is_empty());
    assert_eq!(session.usage_amounts[0].unit, "credit");
    assert_eq!(session.usage_amounts[0].value, 0.041);
}

#[test]
fn kiro_restored_completion_uses_prelaunch_credit_as_baseline() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro-restore", "", 1_782_718_000.0)
        }
    ));
    assert!(apply_screen_signal_unlocked(
        &mut core,
        "term-kiro-restore",
        ScreenSignal::Running,
        true
    ));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-restore",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("0f55b186-4800-4353-a791-5ae222d2faf6".to_string()),
            transcript_path: Some("/tmp/0f55b186-4800-4353-a791-5ae222d2faf6.json".to_string(),),
            model: Some("auto".to_string()),
            assistant_preview: Some("new".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.06803484112769487,
            }],
            baseline_usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.041447917081260374,
            }],
            updated_at: 1_782_718_038.0,
            started_at: Some(1_782_718_036.0),
            completed_at: Some(1_782_718_038.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "unknown".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let session = core.sessions.get("term-kiro-restore").unwrap();
    assert_eq!(session.state, "idle");
    assert_eq!(session.usage_amounts[0].unit, "credit");
    assert!((session.usage_amounts[0].value - 0.06803484112769487).abs() < 0.000_000_001);
    assert_eq!(session.baseline_usage_amounts[0].unit, "credit");
    assert!((session.baseline_usage_amounts[0].value - 0.041447917081260374).abs() < 0.000_000_001);
    let summary = crate::ai_runtime_state::AIRuntimeStateService::new(&std::env::temp_dir())
        .summary_from_runtime_snapshot(&summary::state_snapshot_unlocked(&core));
    let current = summary
        .sessions
        .iter()
        .find(|session| session.terminal_id == "term-kiro-restore")
        .expect("current session");
    assert_eq!(current.raw_usage_amounts[0].unit, "credit");
    assert!((current.raw_usage_amounts[0].value - 0.06803484112769487).abs() < 0.000_000_001);
    assert_eq!(current.usage_amounts[0].unit, "credit");
    assert!((current.usage_amounts[0].value - 0.026586924046434493).abs() < 0.000_000_001);
}

#[test]
fn kiro_late_prelaunch_credit_baseline_overrides_empty_resolved_baseline() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "sessionStarted".to_string(),
            ..test_hook_for("kiro", "term-kiro-late-baseline", "", 1_782_718_000.0)
        }
    ));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-late-baseline",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("kiro-session".to_string()),
            transcript_path: Some("/tmp/kiro-session.json".to_string()),
            model: Some("auto".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1_782_718_001.0,
            started_at: None,
            completed_at: None,
            response_state: None,
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "unknown".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));
    assert!(
        core.sessions
            .get("term-kiro-late-baseline")
            .unwrap()
            .baseline_resolved
    );

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "term-kiro-late-baseline",
        AIRuntimeContextSnapshot {
            tool: "kiro".to_string(),
            external_session_id: Some("kiro-session".to_string()),
            transcript_path: Some("/tmp/kiro-session.json".to_string()),
            model: Some("auto".to_string()),
            assistant_preview: Some("new".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 0,
            usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.06803484112769487,
            }],
            baseline_usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                unit: "credit".to_string(),
                value: 0.041447917081260374,
            }],
            updated_at: 1_782_718_038.0,
            started_at: Some(1_782_718_036.0),
            completed_at: Some(1_782_718_038.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "unknown".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let summary = crate::ai_runtime_state::AIRuntimeStateService::new(&std::env::temp_dir())
        .summary_from_runtime_snapshot(&summary::state_snapshot_unlocked(&core));
    let current = summary
        .sessions
        .iter()
        .find(|session| session.terminal_id == "term-kiro-late-baseline")
        .expect("current session");
    assert_eq!(current.usage_amounts[0].unit, "credit");
    assert!((current.usage_amounts[0].value - 0.026586924046434493).abs() < 0.000_000_001);
}

fn needs_input_probe_snapshot(updated_at: f64) -> AIRuntimeContextSnapshot {
    AIRuntimeContextSnapshot {
        tool: "claude".to_string(),
        external_session_id: Some("claude-external-1".to_string()),
        transcript_path: None,
        model: Some("sonnet".to_string()),
        assistant_preview: None,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_tokens: 0,
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        updated_at,
        started_at: Some(1001.0),
        completed_at: None,
        response_state: Some("needsInput".to_string()),
        was_interrupted: false,
        has_completed_turn: false,
        session_origin: "live".to_string(),
        source: "probe".to_string(),
        plan: None,
    }
}

#[test]
fn prompt_submitted_clears_previous_interruption_flag() {
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        AIHookEventPayload {
            kind: "turnCompleted".to_string(),
            updated_at: 1010.0,
            metadata: Some(AIHookEventMetadata {
                was_interrupted: Some(true),
                has_completed_turn: Some(false),
                ..empty_metadata()
            }),
            ..test_hook("turnCompleted", 1010.0)
        }
    ));
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1020.0)
    ));

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.state, "responding");
    assert!(!session.was_interrupted);
    assert!(!session.has_completed_turn);
}

#[test]
fn reconcile_without_live_terminal_silently_retires_running_session() {
    // A turn whose terminal has vanished is retired silently: loading stops but
    // it must NOT masquerade as an interruption or completion, so no
    // notification fires and it is not enqueued for memory extraction.
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1000.0))
            .did_change
    );

    let mutation = store.reconcile_bridge_snapshot(&[]);

    assert!(mutation.did_change);
    assert!(mutation.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.completion_count, 0);
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(!snapshot.sessions[0].was_interrupted);
    assert!(!snapshot.sessions[0].has_completed_turn);
}

#[test]
fn reconcile_prunes_orphaned_idle_session_after_retention() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1000.0))
            .did_change
    );
    {
        let mut core = store.core.lock().unwrap();
        let session = core.sessions.get_mut("terminal-1").unwrap();
        session.state = "idle".to_string();
        session.updated_at =
            now_seconds() - crate::ai_runtime::constants::IDLE_SESSION_RETENTION_SECONDS - 10.0;
    }
    // Terminal is gone and the idle session is well past retention -> reclaimed.
    let mutation = store.reconcile_bridge_snapshot(&[]);
    assert!(mutation.did_change);
    assert!(store.snapshot().sessions.is_empty());

    // A recently-idled orphan is still retained (within the window).
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", now_seconds()))
            .did_change
    );
    store.reconcile_bridge_snapshot(&[]);
    assert_eq!(store.snapshot().sessions.len(), 1);
}

fn responding_probe_snapshot(updated_at: f64) -> AIRuntimeContextSnapshot {
    AIRuntimeContextSnapshot {
        tool: "codex".to_string(),
        external_session_id: Some("session-1".to_string()),
        transcript_path: None,
        model: Some("gpt-5.5".to_string()),
        assistant_preview: None,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        total_tokens: 0,
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        updated_at,
        started_at: None,
        completed_at: None,
        response_state: Some("responding".to_string()),
        was_interrupted: false,
        has_completed_turn: false,
        session_origin: "live".to_string(),
        source: "probe".to_string(),
        plan: None,
    }
}

fn codex_bridge_terminal() -> crate::ai_runtime::registry::AIRuntimeTerminalState {
    crate::ai_runtime::registry::AIRuntimeTerminalState {
        terminal_id: "terminal-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Codex".to_string(),
        cwd: "/tmp/codex-project".to_string(),
        tool: Some("codex".to_string()),
        is_active: true,
        session_key: Some("session-1".to_string()),
    }
}

#[test]
fn ensure_detected_sessions_creates_idle_session_without_hook_or_active_binding() {
    let store = AIRuntimeStateStore::default();
    // A plain terminal binding: no `is_active`, no `session_key` — exactly what
    // production upserts. Hook-free discovery must still create a session purely
    // from the process-detected tool.
    let terminal = crate::ai_runtime::registry::AIRuntimeTerminalState {
        terminal_id: "terminal-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "zsh".to_string(),
        cwd: "/tmp/codex-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
    };
    let detected =
        std::collections::HashMap::from([("terminal-1".to_string(), "codex".to_string())]);

    let mutation = store.ensure_detected_sessions(&[terminal.clone()], &detected, 1000.0);
    assert!(mutation.did_change);

    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(snapshot.sessions[0].terminal_id, "terminal-1");
    assert_eq!(snapshot.sessions[0].tool, "codex");
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(snapshot.sessions[0].ai_session_id.is_none());

    // Idempotent: a second detection on the same terminal does not duplicate or
    // clobber the existing session.
    let again = store.ensure_detected_sessions(&[terminal], &detected, 1001.0);
    assert!(again.did_change);
    assert_eq!(store.snapshot().sessions.len(), 1);
    assert_eq!(store.snapshot().sessions[0].updated_at, 1001.0);
}

#[test]
fn ensure_detected_sessions_switches_same_terminal_to_new_tool() {
    let store = AIRuntimeStateStore::default();
    let terminal = crate::ai_runtime::registry::AIRuntimeTerminalState {
        terminal_id: "terminal-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "zsh".to_string(),
        cwd: "/tmp/project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
    };
    assert!(
        store
            .ensure_detected_sessions(
                &[terminal.clone()],
                &std::collections::HashMap::from([(
                    "terminal-1".to_string(),
                    "opencode".to_string(),
                )]),
                1000.0,
            )
            .did_change
    );
    assert!(
        store
            .apply_runtime_snapshot(
                "terminal-1",
                AIRuntimeContextSnapshot {
                    tool: "opencode".to_string(),
                    external_session_id: Some("opencode-session".to_string()),
                    transcript_path: Some("/tmp/opencode.db".to_string()),
                    model: Some("gpt-5.4".to_string()),
                    assistant_preview: Some("done".to_string()),
                    input_tokens: 20_000,
                    output_tokens: 1_800,
                    cached_input_tokens: 0,
                    total_tokens: 21_800,
                    usage_amounts: Vec::new(),
                    baseline_usage_amounts: Vec::new(),
                    updated_at: 1010.0,
                    started_at: Some(1001.0),
                    completed_at: Some(1010.0),
                    response_state: Some("idle".to_string()),
                    was_interrupted: false,
                    has_completed_turn: true,
                    session_origin: "fresh".to_string(),
                    source: "probe".to_string(),
                    plan: None,
                },
            )
            .did_change
    );
    assert_eq!(store.snapshot().sessions[0].tool, "opencode");

    assert!(
        store
            .ensure_detected_sessions(
                &[terminal],
                &std::collections::HashMap::from([("terminal-1".to_string(), "agy".to_string(),)]),
                1020.0,
            )
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(snapshot.sessions[0].terminal_id, "terminal-1");
    assert_eq!(snapshot.sessions[0].tool, "agy");
    assert!(snapshot.sessions[0].ai_session_id.is_none());
    assert!(snapshot.sessions[0].model.is_none());
    assert_eq!(snapshot.sessions[0].total_tokens, 0);
    assert_eq!(snapshot.sessions[0].state, "idle");
}

#[test]
fn ensure_detected_sessions_refreshes_existing_idle_kiro_session() {
    let store = AIRuntimeStateStore::default();
    let terminal = crate::ai_runtime::registry::AIRuntimeTerminalState {
        terminal_id: "terminal-kiro".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        slot_id: "slot-1".to_string(),
        title: "Kiro".to_string(),
        cwd: "/tmp/kiro-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
    };
    let detected =
        std::collections::HashMap::from([("terminal-kiro".to_string(), "kiro".to_string())]);

    assert!(
        store
            .ensure_detected_sessions(&[terminal.clone()], &detected, 1000.0)
            .did_change
    );
    assert!(
        store
            .apply_runtime_snapshot(
                "terminal-kiro",
                AIRuntimeContextSnapshot {
                    tool: "kiro".to_string(),
                    external_session_id: Some("kiro-session-1".to_string()),
                    transcript_path: Some("/tmp/kiro-session-1.json".to_string()),
                    model: Some("auto".to_string()),
                    assistant_preview: Some("done".to_string()),
                    input_tokens: 0,
                    output_tokens: 0,
                    cached_input_tokens: 0,
                    total_tokens: 0,
                    usage_amounts: vec![crate::ai_runtime::AIUsageAmountSnapshot {
                        unit: "credit".to_string(),
                        value: 0.03,
                    }],
                    baseline_usage_amounts: Vec::new(),
                    updated_at: 1010.0,
                    started_at: Some(1005.0),
                    completed_at: Some(1010.0),
                    response_state: Some("idle".to_string()),
                    was_interrupted: false,
                    has_completed_turn: true,
                    session_origin: "live".to_string(),
                    source: "probe".to_string(),
                    plan: None,
                },
            )
            .did_change
    );

    assert!(
        !store
            .ensure_detected_sessions(&[terminal], &detected, 2000.0)
            .did_change
    );
    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions[0].tool, "kiro");
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert_eq!(snapshot.sessions[0].updated_at, 1010.0);
}

#[test]
fn runtime_binding_creates_idle_session_without_process_detection() {
    let store = AIRuntimeStateStore::default();

    let mutation = store.apply_binding(test_binding("binding-1", "terminal-1", "instance-1", 10.0));

    assert!(mutation.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(snapshot.sessions[0].terminal_id, "terminal-1");
    assert_eq!(
        snapshot.sessions[0].terminal_instance_id.as_deref(),
        Some("instance-1")
    );
    assert_eq!(snapshot.sessions[0].tool, "codex");
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert_eq!(snapshot.sessions[0].started_at, Some(10.0));
}

#[test]
fn runtime_binding_replaces_reused_terminal_with_new_instance() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_binding(test_binding("binding-1", "terminal-1", "instance-1", 10.0))
            .did_change
    );
    assert!(
        store
            .apply_binding(test_binding("binding-2", "terminal-1", "instance-2", 20.0))
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(
        snapshot.sessions[0].terminal_instance_id.as_deref(),
        Some("instance-2")
    );
    assert_eq!(snapshot.sessions[0].started_at, Some(20.0));
}

#[test]
fn runtime_binding_resets_reused_terminal_for_new_ai_process() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_binding(test_binding("binding-1", "terminal-1", "instance-1", 10.0))
            .did_change
    );
    assert!(
        store
            .apply_runtime_snapshot(
                "terminal-1",
                AIRuntimeContextSnapshot {
                    tool: "codex".to_string(),
                    external_session_id: Some("old-session".to_string()),
                    transcript_path: Some("/tmp/old.jsonl".to_string()),
                    model: Some("old-model".to_string()),
                    assistant_preview: Some("old preview".to_string()),
                    input_tokens: 400,
                    output_tokens: 100,
                    cached_input_tokens: 50,
                    total_tokens: 500,
                    usage_amounts: Vec::new(),
                    baseline_usage_amounts: Vec::new(),
                    updated_at: 12.0,
                    started_at: Some(11.0),
                    completed_at: None,
                    response_state: Some("responding".to_string()),
                    was_interrupted: false,
                    has_completed_turn: false,
                    session_origin: "fresh".to_string(),
                    source: "probe".to_string(),
                    plan: None,
                },
            )
            .did_change
    );

    let mut next_binding = test_binding("binding-2", "terminal-1", "instance-1", 20.0);
    next_binding.external_session_id = Some("new-session".to_string());
    assert!(store.apply_binding(next_binding).did_change);

    let snapshot = store.snapshot();
    assert_eq!(snapshot.sessions.len(), 1);
    let session = &snapshot.sessions[0];
    assert_eq!(session.terminal_id, "terminal-1");
    assert_eq!(session.terminal_instance_id.as_deref(), Some("instance-1"));
    assert_eq!(session.ai_session_id.as_deref(), Some("new-session"));
    assert!(session.transcript_path.is_none());
    assert!(session.model.is_none());
    assert_eq!(session.state, "idle");
    assert_eq!(session.total_tokens, 0);
    assert_eq!(session.baseline_total_tokens, 0);
    assert_eq!(session.started_at, Some(20.0));
}

#[test]
fn binding_first_old_probe_snapshot_becomes_baseline_not_current_usage() {
    let mut core = AIRuntimeStateCore::default();
    let binding = test_binding("binding-1", "terminal-1", "instance-1", 1000.0);
    core.sessions
        .insert("terminal-1".to_string(), binding_terminal_session(&binding));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("old-session".to_string()),
            transcript_path: Some("/tmp/old-codex.jsonl".to_string()),
            model: Some("gpt-5.5".to_string()),
            assistant_preview: None,
            input_tokens: 4_000,
            output_tokens: 1_000,
            cached_input_tokens: 2_000,
            total_tokens: 5_000,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1000.0,
            started_at: Some(900.0),
            completed_at: Some(950.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "unknown".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.baseline_total_tokens, 5_000);
    assert_eq!(session.baseline_cached_input_tokens, 2_000);
    assert_eq!(session.total_tokens, 5_000);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );
}

#[test]
fn probe_backfills_logical_session_identity_on_existing_terminal_binding() {
    let mut core = AIRuntimeStateCore::default();
    let binding = test_binding("binding-1", "terminal-1", "instance-1", 1000.0);
    core.sessions
        .insert("terminal-1".to_string(), binding_terminal_session(&binding));

    assert!(apply_runtime_snapshot_unlocked(
        &mut core,
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "kimi".to_string(),
            external_session_id: Some("driver-session-1".to_string()),
            transcript_path: Some("/tmp/kimi/wire.jsonl".to_string()),
            model: Some("kimi-k2".to_string()),
            assistant_preview: None,
            input_tokens: 1_000,
            output_tokens: 200,
            cached_input_tokens: 300,
            total_tokens: 1_200,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1005.0,
            started_at: Some(900.0),
            completed_at: None,
            response_state: Some("responding".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "restored".to_string(),
            source: "probe".to_string(),
            plan: None,
        }
    ));

    assert_eq!(core.sessions.len(), 1);
    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.tool, "kimi");
    assert_eq!(session.ai_session_id.as_deref(), Some("driver-session-1"));
    assert_eq!(
        session.transcript_path.as_deref(),
        Some("/tmp/kimi/wire.jsonl")
    );
    assert_eq!(session.model.as_deref(), Some("kimi-k2"));
    assert_eq!(session.state, "responding");
    assert_eq!(session.baseline_total_tokens, 1_200);
    assert_eq!(
        summary::project_totals_unlocked(&core, Some("project-1"), now_seconds()).total_tokens,
        0
    );
}

#[test]
fn responding_heartbeat_stops_renewing_after_turn_exceeds_ceiling() {
    // A turn that started long ago and whose transcript still merely *parses* as
    // "responding" (e.g. the CLI was killed mid-turn while the terminal tab
    // stayed open) must not have its heartbeat synthesized forever, otherwise
    // reconcile never ages it and the pet bubble stays pinned.
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));
    assert_eq!(core.sessions.get("terminal-1").unwrap().state, "responding");

    // The probe keeps reporting "responding" with no genuine transcript
    // progress (snapshot.updated_at stays in the distant past).
    apply_runtime_snapshot_unlocked(&mut core, "terminal-1", responding_probe_snapshot(1000.0));

    // Because the turn is older than the renewal ceiling, updated_at is NOT
    // pulled forward to "now" — it stays anchored in the past so staleness aging
    // can fire.
    let session = core.sessions.get("terminal-1").unwrap();
    assert!(
        session.updated_at
            < now_seconds() - crate::ai_runtime::constants::RESPONDING_RENEWAL_MAX_SECONDS,
        "stale responding turn should not renew its heartbeat (updated_at={})",
        session.updated_at
    );

    // reconcile, with the terminal still live, now sees a stale responding
    // session and silently retires it -> idle, releasing the pet bubble without
    // firing a spurious "interrupted" notification.
    let store = AIRuntimeStateStore::default();
    *store.core.lock().unwrap() = core;
    let result = store.reconcile_bridge_snapshot(&[codex_bridge_terminal()]);
    assert!(result.did_change);
    assert!(result.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(!snapshot.sessions[0].was_interrupted);
}

#[test]
fn remove_session_drops_closed_terminal_from_snapshot() {
    // Closing a terminal tab must evict its session from the live state so it
    // stops appearing in the current-session aggregate (otherwise stale cards
    // linger after the tab is gone).
    let store = AIRuntimeStateStore::default();
    let terminal = codex_bridge_terminal();
    let detected =
        std::collections::HashMap::from([(terminal.terminal_id.clone(), "codex".to_string())]);
    assert!(
        store
            .ensure_detected_sessions(&[terminal], &detected, 1000.0)
            .did_change
    );
    assert_eq!(store.snapshot().sessions.len(), 1);

    assert!(store.remove_session("terminal-1"));
    assert!(store.snapshot().sessions.is_empty());

    // Removing an unknown terminal is a no-op.
    assert!(!store.remove_session("terminal-1"));
}

#[test]
fn note_output_activity_only_sustains_a_responding_turn() {
    let store = AIRuntimeStateStore::default();
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", 1000.0)
    ));
    assert_eq!(core.sessions.get("terminal-1").unwrap().state, "responding");
    *store.core.lock().unwrap() = core;

    // Real output during a responding turn pulls updated_at forward.
    let now = now_seconds();
    assert!(store.note_output_activity("terminal-1", now));
    assert!(store.core.lock().unwrap().sessions["terminal-1"].updated_at >= now - 1.0);

    // Unknown terminals never create a session (service/shell output is inert).
    assert!(!store.note_output_activity("terminal-unknown", now));

    // Once the turn goes idle, output no longer fabricates activity.
    store
        .core
        .lock()
        .unwrap()
        .sessions
        .get_mut("terminal-1")
        .unwrap()
        .state = "idle".to_string();
    assert!(!store.note_output_activity("terminal-1", now + 10.0));
}

#[test]
fn responding_heartbeat_renews_within_ceiling_across_quiet_gap() {
    // A genuinely active turn that has only just gone quiet (well within the
    // ceiling) must still have its heartbeat renewed so it is not interrupted
    // mid-flight.
    let now = now_seconds();
    let mut core = AIRuntimeStateCore::default();
    assert!(apply_hook_unlocked(
        &mut core,
        test_hook("promptSubmitted", now - 60.0)
    ));
    assert_eq!(core.sessions.get("terminal-1").unwrap().state, "responding");

    apply_runtime_snapshot_unlocked(
        &mut core,
        "terminal-1",
        responding_probe_snapshot(now - 60.0),
    );

    let session = core.sessions.get("terminal-1").unwrap();
    assert_eq!(session.state, "responding");
    assert!(
        session.updated_at >= now - 5.0,
        "fresh responding turn should renew its heartbeat to ~now (updated_at={}, now={now})",
        session.updated_at
    );
}

#[test]
fn first_prompt_notifies_when_terminal_already_has_detected_session() {
    // Process detection creates an idle session first; the first prompt event
    // then flips it to responding and notifies.
    let store = AIRuntimeStateStore::default();
    let terminal = codex_bridge_terminal();
    let detected =
        std::collections::HashMap::from([(terminal.terminal_id.clone(), "codex".to_string())]);
    assert!(
        store
            .ensure_detected_sessions(&[terminal], &detected, 1000.0)
            .did_change
    );
    assert_eq!(store.snapshot().sessions[0].state, "idle");

    let prompt = test_hook("promptSubmitted", 1001.0);
    let mutation = store.apply_hook(prompt);

    assert!(mutation.did_change);
    assert!(mutation.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.sessions[0].state, "responding");
}

#[test]
fn prompt_submitted_uses_wrapper_project_even_when_hook_cwd_differs() {
    let store = AIRuntimeStateStore::default();
    let mut prompt = test_hook("promptSubmitted", 1000.0);
    prompt.project_path = Some("F:\\codux-gpui".to_string());
    prompt.metadata = Some(AIHookEventMetadata {
        cwd: Some("C:\\Users\\dux".to_string()),
        ..empty_metadata()
    });

    let mutation = store.apply_hook(prompt);

    assert!(mutation.did_change);
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.sessions[0].state, "responding");
}

#[test]
fn multiple_same_tool_sessions_are_isolated_by_terminal_id() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook_for(
                "codex",
                "codex-term-1",
                "codex-session-1",
                1000.0
            ))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook_for(
                "codex",
                "codex-term-2",
                "codex-session-2",
                1001.0
            ))
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.global_totals.running, 2);
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "codex-term-1"
                && session.ai_session_id.as_deref() == Some("codex-session-1")
                && session.state == "responding")
    );
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "codex-term-2"
                && session.ai_session_id.as_deref() == Some("codex-session-2")
                && session.state == "responding")
    );

    assert!(
        store
            .apply_hook(AIHookEventPayload {
                kind: "turnCompleted".to_string(),
                updated_at: 1010.0,
                metadata: Some(AIHookEventMetadata {
                    has_completed_turn: Some(true),
                    ..empty_metadata()
                }),
                ..test_hook_for("codex", "codex-term-1", "codex-session-1", 1010.0)
            })
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.global_totals.running, 1);
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "codex-term-1" && session.state == "idle")
    );
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "codex-term-2"
                && session.ai_session_id.as_deref() == Some("codex-session-2")
                && session.state == "responding")
    );
}

#[test]
fn multiple_claude_sessions_are_isolated_by_terminal_id_and_external_session_id() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook_for(
                "claude",
                "claude-term-1",
                "claude-external-1",
                1000.0
            ))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook_for(
                "claude",
                "claude-term-2",
                "claude-external-2",
                1001.0
            ))
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.global_totals.running, 2);
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "claude-term-1"
                && session.tool == "claude"
                && session.ai_session_id.as_deref() == Some("claude-external-1"))
    );
    assert!(
        snapshot
            .sessions
            .iter()
            .any(|session| session.terminal_id == "claude-term-2"
                && session.tool == "claude"
                && session.ai_session_id.as_deref() == Some("claude-external-2"))
    );
}

#[test]
fn stale_runtime_completion_snapshot_after_prompt_stays_running() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1020.0))
            .did_change
    );

    let mutation = store.apply_runtime_snapshot(
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 150,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1010.0,
            started_at: Some(1000.0),
            completed_at: Some(1010.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
    );

    assert!(mutation.did_change);
    assert!(mutation.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.completion_count, 0);
    assert_eq!(snapshot.sessions[0].state, "responding");
    assert!(!snapshot.sessions[0].has_completed_turn);
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));
}

#[test]
fn codex_prelaunch_interrupted_resume_clears_renewed_running_session() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1_767_225_610.0))
            .did_change
    );
    assert!(
        store
            .apply_runtime_snapshot("terminal-1", responding_probe_snapshot(1_767_225_610.0))
            .did_change
    );
    assert!(store.note_output_activity("terminal-1", 1_767_225_640.0));
    assert!(
        store
            .apply_runtime_snapshot("terminal-1", responding_probe_snapshot(1_767_225_640.0))
            .did_change
    );
    assert_eq!(
        store.snapshot().sessions[0].active_turn_started_at,
        Some(1_767_225_640.0)
    );

    let snapshot = store.apply_runtime_snapshot(
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 150,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1_767_225_640.0,
            started_at: Some(1_767_225_000.0),
            completed_at: Some(1_767_225_640.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "live".to_string(),
            source: CODEX_STALE_PRELAUNCH_OPEN_TURN_SOURCE.to_string(),
            plan: None,
        },
    );

    assert!(snapshot.did_change);
    assert!(snapshot.completion.is_none());
    let state = store.snapshot();
    assert_eq!(state.running_count, 0);
    assert_eq!(state.completion_count, 0);
    assert_eq!(state.sessions[0].state, "idle");
    assert!(!state.sessions[0].was_interrupted);
    assert!(!state.sessions[0].has_completed_turn);
    assert!(matches!(
        state.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));
}

#[test]
fn codex_prelaunch_stale_snapshot_does_not_mark_idle_binding_unfinished() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1_767_225_610.0))
            .did_change
    );

    let mutation = store.apply_runtime_snapshot(
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 150,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1_767_225_610.0,
            started_at: Some(1_767_225_000.0),
            completed_at: Some(1_767_225_610.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: false,
            session_origin: "live".to_string(),
            source: CODEX_STALE_PRELAUNCH_OPEN_TURN_SOURCE.to_string(),
            plan: None,
        },
    );

    assert!(mutation.did_change);
    assert!(mutation.completion.is_none());
    let state = store.snapshot();
    assert_eq!(state.running_count, 0);
    assert_eq!(state.completion_count, 0);
    assert_eq!(state.sessions[0].state, "idle");
    assert!(!state.sessions[0].was_interrupted);
    assert!(!state.sessions[0].has_completed_turn);
    assert!(state.sessions[0].active_turn_started_at.is_none());
    assert!(state.sessions[0].runtime_turn_started_at.is_none());
    assert!(matches!(
        state.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));

    assert!(
        store
            .apply_hook(test_hook_for(
                "codex",
                "terminal-b",
                "session-b",
                1_767_225_620.0
            ))
            .did_change
    );
    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-b", "session-b", 1_767_225_630.0)
    });

    assert!(complete.completion.is_some());
}

#[test]
fn same_second_completion_snapshot_after_prompt_completes() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1020.178))
            .did_change
    );

    let mutation = store.apply_runtime_snapshot(
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: None,
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 150,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1020.743,
            started_at: Some(1000.0),
            completed_at: Some(1020.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
    );

    assert!(mutation.did_change);
    assert!(mutation.completion.is_some());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.completion_count, 1);
    assert_eq!(snapshot.sessions[0].state, "idle");
    assert!(snapshot.sessions[0].has_completed_turn);
}

#[test]
fn later_probe_for_same_completed_turn_does_not_notify_twice() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1020.0))
            .did_change
    );

    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        updated_at: 1030.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook("turnCompleted", 1030.0)
    });
    assert!(complete.did_change);
    assert!(complete.completion.is_some());

    let probe = store.apply_runtime_snapshot(
        "terminal-1",
        AIRuntimeContextSnapshot {
            tool: "codex".to_string(),
            external_session_id: Some("session-1".to_string()),
            transcript_path: Some("/tmp/codex.jsonl".to_string()),
            model: Some("gpt-5.4".to_string()),
            assistant_preview: Some("done".to_string()),
            input_tokens: 0,
            output_tokens: 0,
            cached_input_tokens: 0,
            total_tokens: 200,
            usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            updated_at: 1036.0,
            started_at: Some(1020.0),
            completed_at: Some(1030.0),
            response_state: Some("idle".to_string()),
            was_interrupted: false,
            has_completed_turn: true,
            session_origin: "live".to_string(),
            source: "probe".to_string(),
            plan: None,
        },
    );

    assert!(probe.did_change);
    assert!(probe.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.completion_count, 1);
    assert_eq!(snapshot.sessions[0].total_tokens, 200);
}

#[test]
fn same_session_next_prompt_completion_notifies_again() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook("sessionStarted", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1020.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(AIHookEventPayload {
                kind: "turnCompleted".to_string(),
                updated_at: 1030.0,
                metadata: Some(AIHookEventMetadata {
                    has_completed_turn: Some(true),
                    ..empty_metadata()
                }),
                ..test_hook("turnCompleted", 1030.0)
            })
            .completion
            .is_some()
    );

    assert!(
        store
            .apply_hook(test_hook("promptSubmitted", 1040.0))
            .did_change
    );
    let second = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        updated_at: 1050.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook("turnCompleted", 1050.0)
    });

    assert!(second.did_change);
    assert!(second.completion.is_some());
}

#[test]
fn running_session_suppresses_project_completion_badge() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-a", "session-a", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-b", "session-b", 1001.0))
            .did_change
    );
    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        updated_at: 1010.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-a", "session-a", 1010.0)
    });

    assert!(complete.did_change);
    assert!(complete.completion.is_none());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.completion_count, 0);
    assert!(snapshot.latest_completion.is_none());
    assert!(matches!(
        snapshot.projects[0].project_phase,
        AIProjectPhase::Running { .. }
    ));
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));
}

#[test]
fn dismissed_completion_does_not_reappear_while_another_session_runs() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-a", "session-a", 1000.0))
            .did_change
    );
    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        updated_at: 1010.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-a", "session-a", 1010.0)
    });
    assert!(complete.completion.is_some());
    assert!(store.dismiss_completion("project-1"));
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-b", "session-b", 1020.0))
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 1);
    assert_eq!(snapshot.completion_count, 0);
    assert!(snapshot.latest_completion.is_none());
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));
}

#[test]
fn detected_idle_session_does_not_suppress_sibling_completion() {
    let store = AIRuntimeStateStore::default();
    let idle_terminal = AIRuntimeTerminalState {
        terminal_id: "terminal-b".to_string(),
        terminal_instance_id: Some("terminal-b-instance".to_string()),
        project_id: "project-1".to_string(),
        slot_id: "slot-b".to_string(),
        title: "Claude".to_string(),
        cwd: "/tmp/codex-project".to_string(),
        tool: None,
        is_active: false,
        session_key: None,
    };
    let detected =
        std::collections::HashMap::from([("terminal-b".to_string(), "claude".to_string())]);
    assert!(
        store
            .ensure_detected_sessions(&[idle_terminal.clone()], &detected, 1000.0)
            .did_change
    );
    assert!(
        store
            .ensure_detected_sessions(&[idle_terminal], &detected, 1015.0)
            .did_change
    );
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-a", "session-a", 1020.0))
            .did_change
    );
    let complete = store.apply_hook(AIHookEventPayload {
        kind: "turnCompleted".to_string(),
        updated_at: 1030.0,
        metadata: Some(AIHookEventMetadata {
            has_completed_turn: Some(true),
            ..empty_metadata()
        }),
        ..test_hook_for("codex", "terminal-a", "session-a", 1030.0)
    });

    assert!(complete.did_change);
    assert!(complete.completion.is_some());
    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.completion_count, 1);
    assert!(snapshot.latest_completion.is_some());
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Completed { .. }
    ));
}

#[test]
fn timed_out_unfinished_session_still_suppresses_old_completion() {
    let store = AIRuntimeStateStore::default();
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-a", "session-a", 1000.0))
            .did_change
    );
    assert!(
        store
            .apply_hook(AIHookEventPayload {
                kind: "turnCompleted".to_string(),
                updated_at: 1010.0,
                metadata: Some(AIHookEventMetadata {
                    has_completed_turn: Some(true),
                    ..empty_metadata()
                }),
                ..test_hook_for("codex", "terminal-a", "session-a", 1010.0)
            })
            .completion
            .is_some()
    );
    assert!(
        store
            .apply_hook(test_hook_for("codex", "terminal-b", "session-b", 1020.0))
            .did_change
    );
    assert!(
        store
            .reconcile_bridge_snapshot(&[AIRuntimeTerminalState {
                terminal_id: "terminal-b".to_string(),
                terminal_instance_id: Some("terminal-b-instance".to_string()),
                project_id: "project-1".to_string(),
                slot_id: "slot-b".to_string(),
                title: "Codex".to_string(),
                cwd: "/tmp/codex-project".to_string(),
                tool: None,
                is_active: false,
                session_key: None,
            }])
            .did_change
    );

    let snapshot = store.snapshot();
    assert_eq!(snapshot.running_count, 0);
    assert_eq!(snapshot.completion_count, 0);
    assert!(snapshot.latest_completion.is_none());
    assert!(matches!(
        snapshot.projects[0].completed_phase,
        AIProjectPhase::Idle
    ));
}

fn test_hook(kind: &str, updated_at: f64) -> AIHookEventPayload {
    AIHookEventPayload {
        kind: kind.to_string(),
        terminal_id: "terminal-1".to_string(),
        terminal_instance_id: Some("instance-1".to_string()),
        project_id: "project-1".to_string(),
        project_name: "Project".to_string(),
        project_path: Some("/tmp/codex-project".to_string()),
        session_title: "Codex".to_string(),
        tool: "codex".to_string(),
        ai_session_id: Some("session-1".to_string()),
        model: Some("gpt-5.4".to_string()),
        input_tokens: None,
        output_tokens: None,
        cached_input_tokens: None,
        total_tokens: None,
        updated_at,
        metadata: None,
    }
}

fn test_hook_for(
    tool: &str,
    terminal_id: &str,
    ai_session_id: &str,
    updated_at: f64,
) -> AIHookEventPayload {
    AIHookEventPayload {
        tool: tool.to_string(),
        terminal_id: terminal_id.to_string(),
        terminal_instance_id: Some(format!("{terminal_id}-instance")),
        ai_session_id: Some(ai_session_id.to_string()),
        session_title: format!("{tool} session"),
        updated_at,
        ..test_hook("promptSubmitted", updated_at)
    }
}

fn empty_metadata() -> AIHookEventMetadata {
    AIHookEventMetadata {
        transcript_path: None,
        notification_type: None,
        source: None,
        reason: None,
        cwd: None,
        target_tool_name: None,
        message: None,
        was_interrupted: None,
        has_completed_turn: None,
    }
}

fn test_binding(
    binding_id: &str,
    terminal_id: &str,
    terminal_instance_id: &str,
    started_at: f64,
) -> AIRuntimeBinding {
    AIRuntimeBinding {
        runtime_binding_id: binding_id.to_string(),
        terminal_id: terminal_id.to_string(),
        terminal_instance_id: Some(terminal_instance_id.to_string()),
        tool: "codex".to_string(),
        project_id: "project-1".to_string(),
        project_name: "Project".to_string(),
        project_path: Some("/tmp/codex-project".to_string()),
        session_title: "Codex".to_string(),
        launch_started_at: started_at,
        external_session_id: None,
        transcript_path: None,
        model: None,
        updated_at: started_at,
    }
}
