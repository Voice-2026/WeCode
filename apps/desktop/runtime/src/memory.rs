//! Desktop bridge to the shared `codux-memory` engine.
//!
//! The engine lives in `crates/codux-memory` so the headless host can run it
//! too. The desktop keeps its richer settings/project/session types; this module
//! re-exports the engine and converts the desktop types into the engine's narrow
//! config types at the call boundary (field shapes match, so the conversions are
//! a serde round-trip).

pub use codux_memory::*;

use crate::ai_runtime::AISessionSnapshot;
use crate::project_store::ProjectWorkspaceRecord;
use crate::runtime_state::ProjectInfo;
use crate::settings::{AIMemorySettings, AIProviderSettings, AISettings};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Convert between the desktop and engine types via a serde round-trip. Used for
/// the session snapshot, which derives Serialize and shares the engine's
/// camelCase shape. The settings types are Deserialize-only, so they are copied
/// field-for-field below.
fn convert<A: Serialize, B: DeserializeOwned + Default>(value: &A) -> B {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| serde_json::from_value(json).ok())
        .unwrap_or_default()
}

pub fn memory_config(settings: &AISettings) -> MemoryConfig {
    MemoryConfig {
        global_prompt: settings.global_prompt.clone(),
        memory: memory_settings(&settings.memory),
        providers: settings.providers.iter().map(memory_provider).collect(),
    }
}

pub fn memory_settings(settings: &AIMemorySettings) -> MemorySettings {
    MemorySettings {
        enabled: settings.enabled,
        automatic_injection_enabled: settings.automatic_injection_enabled,
        automatic_extraction_enabled: settings.automatic_extraction_enabled,
        allow_cross_project_user_recall: settings.allow_cross_project_user_recall,
        default_extractor_provider_id: settings.default_extractor_provider_id.clone(),
        max_injected_user_working_memories: settings.max_injected_user_working_memories,
        max_injected_project_working_memories: settings.max_injected_project_working_memories,
        max_active_working_entries: settings.max_active_working_entries,
        max_summary_versions: settings.max_summary_versions,
        summary_target_token_budget: settings.summary_target_token_budget,
        max_injected_summary_tokens: settings.max_injected_summary_tokens,
        extraction_idle_delay_seconds: settings.extraction_idle_delay_seconds,
        session_extraction_cooldown_seconds: settings.session_extraction_cooldown_seconds,
        max_index_sessions: settings.max_index_sessions,
        max_extraction_transcript_lines: settings.max_extraction_transcript_lines,
        max_extraction_transcript_tokens: settings.max_extraction_transcript_tokens,
    }
}

pub fn memory_provider(provider: &AIProviderSettings) -> MemoryProvider {
    MemoryProvider {
        id: provider.id.clone(),
        kind: provider.kind.clone(),
        display_name: provider.display_name.clone(),
        is_enabled: provider.is_enabled,
        model: provider.model.clone(),
        base_url: provider.base_url.clone(),
        api_key: provider.api_key.clone(),
        use_for_memory_extraction: provider.use_for_memory_extraction,
        priority: provider.priority,
    }
}

// ProjectInfo / ProjectWorkspaceRecord are not Serialize, so copy their fields
// directly (the engine models only a narrow subset).
pub fn memory_project_info(project: &ProjectInfo) -> MemoryProjectInfo {
    MemoryProjectInfo {
        id: project.id.clone(),
        name: project.name.clone(),
        path: project.path.clone(),
    }
}

pub fn memory_project_infos(projects: &[ProjectInfo]) -> Vec<MemoryProjectInfo> {
    projects.iter().map(memory_project_info).collect()
}

pub fn memory_project_record(record: &ProjectWorkspaceRecord) -> MemoryProjectRecord {
    MemoryProjectRecord {
        id: record.id.clone(),
        root_project_id: record.root_project_id.clone(),
        root_project_name: record.root_project_name.clone(),
        root_project_path: record.root_project_path.clone(),
        workspace_path: record.workspace_path.clone(),
        git_default_push_remote_name: record.git_default_push_remote_name.clone(),
    }
}

pub fn memory_project_records(records: &[ProjectWorkspaceRecord]) -> Vec<MemoryProjectRecord> {
    records.iter().map(memory_project_record).collect()
}

pub fn memory_session(session: &AISessionSnapshot) -> MemorySessionSnapshot {
    convert(session)
}

pub fn memory_sessions(sessions: &[AISessionSnapshot]) -> Vec<MemorySessionSnapshot> {
    sessions.iter().map(memory_session).collect()
}

/// Resolve the launch-artifact paths under the desktop's runtime root. Keeps the
/// 1-arg signature the GPUI launch path expects (shadows the engine's 2-arg
/// `launch_artifact_paths`, which the desktop reaches via the explicit base).
pub fn launch_artifact_paths(project_id: &str) -> MemoryLaunchArtifacts {
    codux_memory::launch_artifact_paths(&crate::runtime_paths::runtime_root_dir(), project_id)
}
