use crate::ai_history_normalized::{
    AIGlobalHistorySnapshot, AIHistoryProjectRequest, AIHistorySnapshot,
    load_indexed_global_history, load_indexed_project_history,
};
use crate::runtime_trace::runtime_trace_elapsed;
use std::sync::mpsc::Receiver;
use std::time::Instant;

pub(super) fn indexed_project_snapshot(
    project: AIHistoryProjectRequest,
) -> Result<Option<AIHistorySnapshot>, String> {
    let started_at = Instant::now();
    let project_id = project.id.clone();
    load_indexed_project_history(project)
        .map_err(|error| error.to_string())
        .map(|snapshot| {
            runtime_trace_elapsed(
                "ai-history",
                "load_project_cache",
                started_at,
                &format!(
                    "project={} hit={} sessions={}",
                    project_id,
                    snapshot.is_some(),
                    snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.sessions.len())
                        .unwrap_or(0)
                ),
            );
            snapshot
        })
}

pub(super) fn indexed_global_snapshot(
    projects: Vec<AIHistoryProjectRequest>,
) -> Result<Option<AIGlobalHistorySnapshot>, String> {
    let started_at = Instant::now();
    let project_count = projects.len();
    load_indexed_global_history(projects)
        .map_err(|error| error.to_string())
        .map(|snapshot| {
            runtime_trace_elapsed(
                "ai-history",
                "load_global_cache",
                started_at,
                &format!(
                    "projects={} hit={} sessions={}",
                    project_count,
                    snapshot.is_some(),
                    snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.sessions.len())
                        .unwrap_or(0)
                ),
            );
            snapshot
        })
}

pub(super) fn receive_reply<T>(rx: Receiver<Result<T, String>>) -> Result<T, String> {
    rx.recv()
        .map_err(|_| "AI history indexer reply dropped.".to_string())?
}
