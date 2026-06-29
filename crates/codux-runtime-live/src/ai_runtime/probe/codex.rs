mod parse;
mod preview;
mod types;

use crate::ai_runtime::{
    probe::common::now_seconds,
    probe::paths::{
        codex_session_id_from_rollout, find_codex_rollout_by_cwd_since, find_codex_rollout_path,
    },
    snapshot::{AIRuntimeContextSnapshot, AIRuntimeProbeRequest},
    state::normalized_string,
};

use self::parse::parse_codex_runtime_state;

pub(crate) fn probe_codex_runtime(
    request: &AIRuntimeProbeRequest,
) -> Option<AIRuntimeContextSnapshot> {
    let project_path = normalized_string(request.project_path.as_deref())?;
    let file_path = normalized_string(request.transcript_path.as_deref())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            // Known id → match by name; otherwise locate the live rollout by cwd.
            match normalized_string(request.external_session_id.as_deref()) {
                Some(external_id) => find_codex_rollout_path(&project_path, &external_id),
                None => find_codex_rollout_by_cwd_since(&project_path, request.started_at),
            }
        })?;
    let transcript_path = file_path.display().to_string();
    let parsed = parse_codex_runtime_state(
        &file_path,
        Some(&project_path),
        request.started_at,
        request.updated_at,
    )?;
    // Pure-file approval wait: a command/patch call is written with no result
    // yet, the approval policy can still prompt, and it has sat idle past the
    // gap. (codex sessions re-probe on a quiet-session interval, so for
    // non-`never` policies this can surface a few seconds late; `never` -- the
    // common headless setup -- gates it off entirely.)
    let mut response_state = parsed.response_state.clone();
    if parsed.needs_user_input(now_seconds()) {
        response_state = Some("needsInput".to_string());
    }
    let external_session_id = normalized_string(request.external_session_id.as_deref())
        .or_else(|| codex_session_id_from_rollout(&file_path));
    let mut plan = parsed.plan;
    if let (Some(plan), Some(session_id)) = (plan.as_mut(), external_session_id.as_ref()) {
        plan.session_id = session_id.clone();
    }
    Some(AIRuntimeContextSnapshot {
        tool: "codex".to_string(),
        external_session_id,
        transcript_path: Some(transcript_path),
        model: parsed.model,
        assistant_preview: parsed.assistant_preview,
        input_tokens: parsed.input_tokens.unwrap_or(0),
        output_tokens: parsed.output_tokens.unwrap_or(0),
        cached_input_tokens: parsed.cached_input_tokens.unwrap_or(0),
        total_tokens: parsed.total_tokens.unwrap_or(0),
        usage_amounts: Vec::new(),
        baseline_usage_amounts: Vec::new(),
        updated_at: parsed.updated_at.unwrap_or(request.updated_at),
        started_at: parsed.started_at,
        completed_at: parsed.completed_at,
        response_state,
        was_interrupted: parsed.was_interrupted,
        has_completed_turn: parsed.has_completed_turn,
        session_origin: parsed.origin,
        source: "probe".to_string(),
        plan,
    })
}
