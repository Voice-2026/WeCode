use super::super::*;
use super::fixtures::*;

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
