//! Memory serving for the headless host. The controller routes a remote-hosted
//! project's memory reads here; the agent runs the shared `codux-memory` engine
//! against its own memory store (`<agent_data_dir>/memory.sqlite3`) so the
//! project's memory lives where its AI sessions run.
//!
//! This is the read path (`memory.read` → summary/manager/management/status).
//! Extraction (`memory.extract`, the LLM write path driven by a
//! controller-forwarded provider config) is the follow-up.

use codux_memory::{MemoryManagementRequest, MemoryProjectInfo, MemoryService};
use serde_json::{Value, json};

use crate::projects::{AgentProjectStore, agent_data_dir};

fn service() -> MemoryService {
    MemoryService::new(agent_data_dir())
}

/// The host's projects mapped into the engine's project shape (the manager view
/// labels rows per project).
fn memory_projects() -> Vec<MemoryProjectInfo> {
    AgentProjectStore::new()
        .list()
        .into_iter()
        .map(|project| MemoryProjectInfo {
            id: project.id,
            name: project.name,
            path: project.path,
        })
        .collect()
}

/// Resolve the host's own project id for a controller-supplied path. The
/// host's memory store is keyed by the host's project ids, but the controller
/// only knows its own ids; like `ai.state`, it sends the project *path* and the
/// host maps it to its local project (falling back to the supplied id).
fn host_project_id(payload: &Value) -> Option<String> {
    let project_path = payload.get("projectPath").and_then(Value::as_str);
    if let Some(path) = project_path.filter(|value| !value.is_empty()) {
        if let Some(project) = AgentProjectStore::new()
            .list()
            .into_iter()
            .find(|project| project.path == path)
        {
            return Some(project.id);
        }
    }
    payload
        .get("projectId")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Serve a `memory.read` query. Returns `{op, result}` where `result` is the
/// op's JSON snapshot (or null on error, mirroring the engine's own fallbacks).
pub fn memory_read_payload(payload: &Value) -> Value {
    let op = payload.get("op").and_then(Value::as_str).unwrap_or("");
    let resolved_project_id = host_project_id(payload);
    let project_id = resolved_project_id.as_deref();
    let result = match op {
        "summary" => serde_json::to_value(service().summary(project_id)).unwrap_or(Value::Null),
        "status" => service()
            .extraction_status_snapshot()
            .ok()
            .and_then(|status| serde_json::to_value(status).ok())
            .unwrap_or(Value::Null),
        "management" => match serde_json::from_value::<MemoryManagementRequest>(payload.clone()) {
            Ok(mut request) => {
                request.project_id = project_id.map(str::to_string);
                service()
                    .management_snapshot(request)
                    .ok()
                    .and_then(|snapshot| serde_json::to_value(snapshot).ok())
                    .unwrap_or(Value::Null)
            }
            Err(_) => Value::Null,
        },
        "manager" => {
            let scope = payload.get("scope").and_then(Value::as_str).unwrap_or("project");
            let tab = payload.get("tab").and_then(Value::as_str).unwrap_or("active");
            let limit = payload.get("limit").and_then(Value::as_i64).unwrap_or(500);
            serde_json::to_value(service().manager_snapshot(
                &memory_projects(),
                scope,
                project_id,
                tab,
                limit,
            ))
            .unwrap_or(Value::Null)
        }
        _ => Value::Null,
    };
    json!({ "op": op, "result": result })
}
