mod claude;
mod codex;
mod common;
mod gemini;
mod kiro;
mod opencode;
pub(crate) mod paths;
mod preview;
mod usage;

use crate::ai_runtime::{
    snapshot::{AIRuntimeContextSnapshot, AIRuntimeProbeRequest},
    state::canonical_tool_name,
};

pub fn probe_runtime(request: &AIRuntimeProbeRequest) -> Option<AIRuntimeContextSnapshot> {
    match canonical_tool_name(&request.tool).as_deref() {
        Some("codex") => codex::probe_codex_runtime(request),
        Some("claude") => claude::probe_claude_runtime(request),
        Some("gemini") => gemini::probe_gemini_runtime(request),
        Some("opencode") => opencode::probe_opencode_runtime(request),
        Some("kiro") => kiro::probe_kiro_runtime(request),
        _ => None,
    }
}
