use super::*;
use wecode_runtime::ai_runtime_state::{
    AIRuntimeProjectPhaseSummary, AIRuntimeProjectStateSummary,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::app) enum AgentLifecycleState {
    Idle,
    Working,
    Waiting,
    Completed,
    Error,
    Warning,
}

pub(in crate::app) fn selected_worktree_info(state: &RuntimeState) -> Option<WorktreeInfo> {
    let selected_id = state.worktrees.selected_worktree_id.as_deref()?;
    state
        .worktrees
        .worktrees
        .iter()
        .find(|worktree| worktree.id == selected_id)
        .filter(|worktree| worktree.is_default || worktree.exists)
        .cloned()
}

pub(in crate::app) fn terminal_layout_owner_id(state: &RuntimeState) -> Option<String> {
    selected_worktree_info(state)
        .map(|worktree| worktree.id)
        .or_else(|| {
            state
                .selected_project
                .as_ref()
                .map(|project| project.id.clone())
        })
}

pub(in crate::app) fn terminal_layout_storage_key(project_id: &str, worktree_id: &str) -> String {
    wecode_runtime::terminal_layout::terminal_layout_storage_key(project_id, worktree_id)
}

pub(in crate::app) fn current_terminal_layout_storage_key(state: &RuntimeState) -> Option<String> {
    let project_id = state.selected_project.as_ref()?.id.as_str();
    let worktree_id = terminal_layout_owner_id(state)?;
    Some(terminal_layout_storage_key(project_id, &worktree_id))
}

pub(in crate::app) fn ai_activity_project_states_changed(
    previous: &[AIRuntimeProjectStateSummary],
    next: &[AIRuntimeProjectStateSummary],
) -> bool {
    previous.len() != next.len()
        || previous
            .iter()
            .zip(next)
            .any(|(previous, next)| ai_activity_project_state_changed(previous, next))
}

fn ai_activity_project_state_changed(
    previous: &AIRuntimeProjectStateSummary,
    next: &AIRuntimeProjectStateSummary,
) -> bool {
    previous.project_id != next.project_id
        || ai_activity_phase_changed(&previous.project_phase, &next.project_phase)
        || ai_activity_phase_changed(&previous.completed_phase, &next.completed_phase)
        || previous.totals != next.totals
}

fn ai_activity_phase_changed(
    previous: &AIRuntimeProjectPhaseSummary,
    next: &AIRuntimeProjectPhaseSummary,
) -> bool {
    previous.kind != next.kind
        || previous.tool != next.tool
        || previous.was_interrupted != next.was_interrupted
        || ((previous.kind == "completed" || next.kind == "completed")
            && previous.updated_at != next.updated_at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wecode_runtime::ai_runtime_state::{
        AIRuntimeProjectPhaseSummary, AIRuntimeProjectTotalsSummary,
    };

    fn project_state(project_id: &str, kind: &str) -> AIRuntimeProjectStateSummary {
        AIRuntimeProjectStateSummary {
            project_id: project_id.to_string(),
            project_phase: AIRuntimeProjectPhaseSummary {
                kind: kind.to_string(),
                updated_at: 1.0,
                ..Default::default()
            },
            completed_phase: AIRuntimeProjectPhaseSummary::default(),
            totals: AIRuntimeProjectTotalsSummary {
                project_id: project_id.to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn ai_activity_project_states_changed_tracks_phase_changes() {
        let previous = vec![project_state("project-a", "idle")];
        let next = vec![project_state("project-a", "running")];

        assert!(ai_activity_project_states_changed(&previous, &next));
    }

    #[test]
    fn ai_activity_project_states_changed_ignores_equal_project_states() {
        let previous = vec![project_state("project-a", "running")];
        let next = previous.clone();

        assert!(!ai_activity_project_states_changed(&previous, &next));
    }

    #[test]
    fn ai_activity_project_states_changed_ignores_timestamp_heartbeats() {
        let previous = vec![project_state("project-a", "running")];
        let mut next = previous.clone();
        next[0].project_phase.updated_at = 20.0;

        assert!(!ai_activity_project_states_changed(&previous, &next));
    }

    #[test]
    fn ai_activity_project_states_changed_tracks_completed_timestamp() {
        let previous = vec![project_state("project-a", "idle")];
        let mut next = previous.clone();
        next[0].completed_phase = AIRuntimeProjectPhaseSummary {
            kind: "completed".to_string(),
            updated_at: 20.0,
            ..Default::default()
        };

        assert!(ai_activity_project_states_changed(&previous, &next));
    }
}
