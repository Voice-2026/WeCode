use super::*;
use crate::ai_runtime::{
    AIHookEventPayload, AIRuntimeSupervisorEvent, TerminalStatusEvent, TerminalStatusState,
};
use std::time::{Duration, Instant};

mod capture;
mod environment;
mod manager;
mod osc;
mod viewport;

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

fn wait_for_terminal_status(
    bridge: &AIRuntimeBridge,
    terminal_id: &str,
    state: TerminalStatusState,
) -> TerminalStatusEvent {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        for event in bridge.drain_supervisor_events() {
            if let AIRuntimeSupervisorEvent::TerminalStatus { status } = event {
                if status.terminal_id == terminal_id && status.state == state {
                    return status;
                }
                seen.push(status);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("terminal {terminal_id} did not emit status {state:?}; seen={seen:?}");
}
