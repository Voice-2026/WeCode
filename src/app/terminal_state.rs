use super::types::{
    TerminalPanePlan, TerminalPaneSlot, TerminalRestorePlan, TerminalTab, TerminalTabPlan,
};
use crate::terminal::{
    TerminalConfig, TerminalLaunchContext, TerminalPane, terminal_config_with_font_family,
};
use anyhow::Result;
use codux_runtime::{
    memory::{MemoryLaunchRequest, MemoryService, launch_artifact_paths},
    runtime_bridge::RuntimeInventory,
    runtime_state::RuntimeState,
    settings::{AppSettingsStore, SettingsSummary},
    terminal_layout::{TerminalLayoutSummary, TerminalPaneSummary, TerminalTabSummary},
    terminal_pty::TerminalManager,
    terminal_runtime::{TerminalRuntimeSessionSummary, TerminalRuntimeSummary},
    tool_permissions::ToolPermissionsSummary,
};
use gpui::px;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

pub(in crate::app) fn terminal_restore_plan(
    layout: &TerminalLayoutSummary,
    runtime: &TerminalRuntimeSummary,
) -> TerminalRestorePlan {
    let mut tabs = Vec::new();
    if !layout.top_panes.is_empty() {
        tabs.push(TerminalTabPlan {
            source_id: None,
            terminal_id: None,
            label: "主终端".to_string(),
            panes: layout
                .top_panes
                .iter()
                .enumerate()
                .map(|(index, pane)| {
                    let title = if pane.title.trim().is_empty() {
                        format!("分屏 {}", index + 1)
                    } else {
                        pane.title.clone()
                    };
                    TerminalPanePlan {
                        source_id: Some(pane.id.clone()).filter(|id| !id.trim().is_empty()),
                        terminal_id: Some(pane.terminal_id.clone())
                            .filter(|id| !id.trim().is_empty()),
                        title,
                        restored_output_bytes: restored_terminal_output_bytes(
                            runtime,
                            Some(&pane.terminal_id),
                            Some(&pane.id),
                        ),
                        restored_output_tail: restored_terminal_output_tail(
                            runtime,
                            Some(&pane.terminal_id),
                            Some(&pane.id),
                        ),
                    }
                })
                .collect(),
        });
    }
    tabs.extend(layout.tabs.iter().enumerate().map(|(index, tab)| {
        let label = if tab.label.trim().is_empty() {
            format!("标签页 {}", index + 1)
        } else {
            tab.label.clone()
        };
        TerminalTabPlan {
            source_id: Some(tab.id.clone()).filter(|id| !id.trim().is_empty()),
            terminal_id: Some(tab.terminal_id.clone()).filter(|id| !id.trim().is_empty()),
            panes: vec![TerminalPanePlan {
                source_id: Some(tab.id.clone()).filter(|id| !id.trim().is_empty()),
                terminal_id: Some(tab.terminal_id.clone()).filter(|id| !id.trim().is_empty()),
                title: label.clone(),
                restored_output_bytes: restored_terminal_output_bytes(
                    runtime,
                    Some(&tab.terminal_id),
                    Some(&tab.id),
                ),
                restored_output_tail: restored_terminal_output_tail(
                    runtime,
                    Some(&tab.terminal_id),
                    Some(&tab.id),
                ),
            }],
            label,
        }
    }));
    if tabs.is_empty() {
        tabs.push(TerminalTabPlan {
            source_id: None,
            terminal_id: None,
            label: "终端 1".to_string(),
            panes: vec![TerminalPanePlan {
                source_id: None,
                terminal_id: None,
                title: "终端 1".to_string(),
                restored_output_bytes: restored_terminal_output_bytes(runtime, None, None),
                restored_output_tail: restored_terminal_output_tail(runtime, None, None),
            }],
        });
    }
    for (index, tab) in tabs.iter_mut().enumerate() {
        if tab.panes.is_empty() {
            tab.panes.push(TerminalPanePlan {
                source_id: tab.source_id.clone(),
                terminal_id: tab.terminal_id.clone(),
                title: format!("分屏 {}", index + 1),
                restored_output_bytes: restored_terminal_output_bytes(
                    runtime,
                    tab.terminal_id.as_deref(),
                    tab.source_id.as_deref(),
                ),
                restored_output_tail: restored_terminal_output_tail(
                    runtime,
                    tab.terminal_id.as_deref(),
                    tab.source_id.as_deref(),
                ),
            });
        }
    }

    let active_index = layout
        .tabs
        .iter()
        .position(|tab| !layout.active_tab_id.is_empty() && tab.id == layout.active_tab_id)
        .map(|index| {
            if layout.top_panes.is_empty() {
                index
            } else {
                index + 1
            }
        })
        .or_else(|| {
            (!layout.top_panes.is_empty()
                && (layout.active_slot_id.is_empty()
                    || layout
                        .top_panes
                        .iter()
                        .any(|pane| pane.id == layout.active_slot_id)))
            .then_some(0)
        })
        .unwrap_or(0)
        .min(tabs.len().saturating_sub(1));

    TerminalRestorePlan { tabs, active_index }
}

fn restored_terminal_output_tail(
    runtime: &TerminalRuntimeSummary,
    terminal_id: Option<&str>,
    slot_id: Option<&str>,
) -> String {
    runtime
        .sessions
        .iter()
        .find(|session| terminal_session_matches(session, terminal_id, slot_id))
        .map(|session| session.output_tail.clone())
        .unwrap_or_default()
}

fn restored_terminal_output_bytes(
    runtime: &TerminalRuntimeSummary,
    terminal_id: Option<&str>,
    slot_id: Option<&str>,
) -> usize {
    runtime
        .sessions
        .iter()
        .find(|session| terminal_session_matches(session, terminal_id, slot_id))
        .map(|session| session.output_bytes)
        .unwrap_or_default()
}

fn terminal_session_matches(
    session: &TerminalRuntimeSessionSummary,
    terminal_id: Option<&str>,
    slot_id: Option<&str>,
) -> bool {
    let terminal_id = terminal_id.filter(|id| !id.trim().is_empty());
    let slot_id = slot_id.filter(|id| !id.trim().is_empty());
    let terminal_matches = terminal_id.is_some_and(|id| session.terminal_id == id);
    let slot_matches = slot_id.is_some_and(|id| session.slot_id == id || session.tab_id == id);
    match (terminal_id, slot_id) {
        (Some(_), Some(_)) => terminal_matches && slot_matches,
        (Some(_), None) => terminal_matches,
        (None, Some(_)) => slot_matches,
        (None, None) => session.is_running,
    }
}

pub(in crate::app) fn spawn_terminal_tabs<C>(
    plan: &TerminalRestorePlan,
    terminal_manager: Arc<TerminalManager>,
    launch_context: Option<&TerminalLaunchContext>,
    terminal_config: TerminalConfig,
    cx: &mut C,
) -> Result<(Vec<TerminalTab>, usize, usize)>
where
    C: gpui::AppContext,
{
    let mut next_id = 1;
    let mut tabs = Vec::new();
    for tab_plan in &plan.tabs {
        let tab_id = next_id;
        next_id += 1;
        let mut panes = Vec::new();
        for (pane_index, pane_plan) in tab_plan.panes.iter().enumerate() {
            let pane_context =
                terminal_pane_launch_context(launch_context, tab_id, pane_index, pane_plan);
            panes.push(TerminalPaneSlot {
                title: pane_plan.title.clone(),
                launch_context: pane_context.clone(),
                pane: TerminalPane::spawn_with_context_and_config(
                    cx,
                    terminal_manager.clone(),
                    pane_context.as_ref(),
                    terminal_config.clone(),
                )?,
                restored_output_bytes: pane_plan.restored_output_bytes,
                restored_output_tail: pane_plan.restored_output_tail.clone(),
            });
        }
        if panes.is_empty() {
            let pane_plan = TerminalPanePlan {
                source_id: tab_plan.source_id.clone(),
                terminal_id: tab_plan.terminal_id.clone(),
                title: tab_plan.label.clone(),
                restored_output_bytes: 0,
                restored_output_tail: String::new(),
            };
            let pane_context = terminal_pane_launch_context(launch_context, tab_id, 0, &pane_plan);
            panes.push(TerminalPaneSlot {
                title: tab_plan.label.clone(),
                launch_context: pane_context.clone(),
                pane: TerminalPane::spawn_with_context_and_config(
                    cx,
                    terminal_manager.clone(),
                    pane_context.as_ref(),
                    terminal_config.clone(),
                )?,
                restored_output_bytes: pane_plan.restored_output_bytes,
                restored_output_tail: pane_plan.restored_output_tail,
            });
        }
        tabs.push(TerminalTab {
            id: tab_id,
            label: tab_plan.label.clone(),
            source_id: tab_plan.source_id.clone(),
            terminal_id: tab_plan.terminal_id.clone(),
            panes,
        });
    }
    let active_terminal_id = tabs
        .get(plan.active_index)
        .or_else(|| tabs.first())
        .map(|tab| tab.id)
        .unwrap_or(1);
    Ok((tabs, active_terminal_id, next_id))
}

pub(in crate::app) fn terminal_launch_context(
    state: &RuntimeState,
    runtime: &RuntimeInventory,
    tool_permissions: &ToolPermissionsSummary,
) -> Option<TerminalLaunchContext> {
    let project = state.selected_project.as_ref()?;
    let memory_artifacts = (state.memory.available && state.settings.memory_enabled)
        .then(|| launch_artifact_paths(&project.id));
    Some(TerminalLaunchContext {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_path: PathBuf::from(&project.path),
        support_dir: state.support_dir.clone(),
        runtime_root: runtime.root.clone(),
        terminal_id: None,
        slot_id: None,
        session_key: None,
        session_title: None,
        session_cwd: None,
        session_instance_id: None,
        tool_permissions_file: tool_permissions
            .error
            .is_none()
            .then(|| PathBuf::from(&tool_permissions.path)),
        memory_workspace_root: memory_artifacts
            .as_ref()
            .map(|artifacts| artifacts.workspace_root.clone()),
        memory_prompt_file: memory_artifacts
            .as_ref()
            .map(|artifacts| artifacts.prompt_file.clone()),
        memory_index_file: memory_artifacts.map(|artifacts| artifacts.index_file),
    })
}

pub(in crate::app) fn terminal_config_for_settings(settings: &SettingsSummary) -> TerminalConfig {
    let mut config = terminal_config_with_font_family(&settings.terminal_font_family);
    let font_size = settings
        .terminal_font_size
        .parse::<f32>()
        .unwrap_or(14.0)
        .clamp(10.0, 28.0);
    config.font_size = px(font_size);
    config
}

pub(in crate::app) fn terminal_pane_launch_context(
    base: Option<&TerminalLaunchContext>,
    tab_id: usize,
    pane_index: usize,
    pane: &TerminalPanePlan,
) -> Option<TerminalLaunchContext> {
    let mut context = base.cloned()?;
    let terminal_id = pane
        .terminal_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| format!("gpui-term-{tab_id}"));
    let slot_id = pane
        .source_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| format!("gpui-pane-{tab_id}-{}", pane_index + 1));
    let session_key = format!("gpui:{}:{terminal_id}:{slot_id}", context.project_id);
    let session_instance_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, session_key.as_bytes());
    context.terminal_id = Some(terminal_id);
    context.slot_id = Some(slot_id);
    context.session_key = Some(session_key);
    context.session_title = Some(pane.title.clone());
    context.session_cwd = Some(context.project_path.clone());
    context.session_instance_id = Some(session_instance_id.to_string());
    Some(context)
}

pub(in crate::app) fn prepare_memory_launch_artifacts(state: &RuntimeState) {
    if !state.memory.available || !state.settings.memory_enabled {
        return;
    }
    let Some(project) = &state.selected_project else {
        return;
    };
    let app_settings = AppSettingsStore::from_support_dir(state.support_dir.clone()).snapshot();
    let _ = MemoryService::new(state.support_dir.clone()).prepare_launch_artifacts(
        MemoryLaunchRequest {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            workspace_path: Some(project.path.clone()),
            settings: app_settings.ai,
            extra_context: None,
        },
    );
}

pub(in crate::app) fn terminal_tab_summary(tab: &TerminalTab) -> TerminalTabSummary {
    TerminalTabSummary {
        id: tab
            .source_id
            .clone()
            .or_else(|| {
                tab.panes
                    .first()
                    .and_then(|slot| slot.launch_context.as_ref())
                    .and_then(|context| context.slot_id.clone())
            })
            .unwrap_or_else(|| format!("bottom-{}", tab.id)),
        label: tab.label.clone(),
        terminal_id: tab
            .terminal_id
            .clone()
            .or_else(|| {
                tab.panes
                    .first()
                    .and_then(|slot| slot.launch_context.as_ref())
                    .and_then(|context| context.terminal_id.clone())
            })
            .unwrap_or_else(|| format!("gpui-term-{}", tab.id)),
    }
}

pub(in crate::app) fn terminal_pane_summary(
    tab_id: usize,
    index: usize,
    slot: &TerminalPaneSlot,
) -> TerminalPaneSummary {
    TerminalPaneSummary {
        id: slot
            .launch_context
            .as_ref()
            .and_then(|context| context.slot_id.clone())
            .unwrap_or_else(|| format!("top-{}", index + 1)),
        title: slot.title.clone(),
        terminal_id: slot
            .launch_context
            .as_ref()
            .and_then(|context| context.terminal_id.clone())
            .unwrap_or_else(|| format!("gpui-pane-{}-{}", tab_id, index + 1)),
    }
}
