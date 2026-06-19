//! Lightweight trace hook. The engine emits diagnostics through an optional
//! process-wide sink: the desktop forwards them into its rotating runtime log,
//! while the headless agent can leave it unset (no-op) or install its own.

use std::sync::OnceLock;
use std::time::Instant;

type TraceSink = fn(&str, &str);

static SINK: OnceLock<TraceSink> = OnceLock::new();

/// Install the process-wide trace sink. Only the first call wins.
pub fn set_trace_sink(sink: TraceSink) {
    let _ = SINK.set(sink);
}

pub fn runtime_trace(category: &str, message: &str) {
    if let Some(sink) = SINK.get() {
        sink(category, message);
    }
}

pub fn runtime_trace_elapsed(category: &str, action: &str, started_at: Instant, details: &str) {
    let elapsed_ms = started_at.elapsed().as_millis();
    if details.trim().is_empty() {
        runtime_trace(category, &format!("{action} elapsed_ms={elapsed_ms}"));
    } else {
        runtime_trace(category, &format!("{action} elapsed_ms={elapsed_ms} {details}"));
    }
}
