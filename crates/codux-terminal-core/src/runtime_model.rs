use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRuntimeProject {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRuntimeTerminal {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(default = "default_terminal_layout_kind")]
    pub layout_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_characters: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRuntimePlan {
    #[serde(default)]
    pub state_changed: bool,
    #[serde(default)]
    pub clear_terminal: bool,
    #[serde(default)]
    pub reset_terminal_input: bool,
    #[serde(default)]
    pub reset_terminal_buffer: bool,
    #[serde(default)]
    pub request_terminal_list: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_project_select_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_session_id: Option<String>,
    #[serde(default)]
    pub bind_full_buffer: bool,
    #[serde(default)]
    pub flush_terminal_input: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed_session_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRuntimeStateSnapshot {
    pub projects: Vec<RemoteRuntimeProject>,
    pub terminals: Vec<RemoteRuntimeTerminal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_project_select_id: Option<String>,
    #[serde(default)]
    pub pending_project_select_sent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_select_acknowledged_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creating_terminal_project_id: Option<String>,
    #[serde(default)]
    pub last_terminal_id_by_project: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct RemoteRuntimeModel {
    projects: Vec<RemoteRuntimeProject>,
    terminals: Vec<RemoteRuntimeTerminal>,
    selected_project_id: Option<String>,
    active_session_id: Option<String>,
    pending_project_select_id: Option<String>,
    pending_project_select_sent: bool,
    project_select_acknowledged_id: Option<String>,
    creating_terminal_project_id: Option<String>,
    last_terminal_id_by_project: HashMap<String, String>,
}

impl RemoteRuntimeModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> RemoteRuntimeStateSnapshot {
        RemoteRuntimeStateSnapshot {
            projects: self.projects.clone(),
            terminals: self.terminals.clone(),
            selected_project_id: self.selected_project_id.clone(),
            active_session_id: self.active_session_id.clone(),
            pending_project_select_id: self.pending_project_select_id.clone(),
            pending_project_select_sent: self.pending_project_select_sent,
            project_select_acknowledged_id: self.project_select_acknowledged_id.clone(),
            creating_terminal_project_id: self.creating_terminal_project_id.clone(),
            last_terminal_id_by_project: self.last_terminal_id_by_project.clone(),
        }
    }

    pub fn reset(&mut self, keep_projects: bool) {
        let projects = if keep_projects {
            self.projects.clone()
        } else {
            Vec::new()
        };
        let selected = self
            .selected_project_id
            .as_ref()
            .filter(|selected| keep_projects && projects.iter().any(|item| item.id == **selected))
            .cloned();
        *self = Self {
            projects,
            selected_project_id: selected,
            ..Self::default()
        };
    }

    pub fn restore_cached_projects(&mut self, projects: Vec<RemoteRuntimeProject>) {
        if projects.is_empty() || !self.projects.is_empty() {
            return;
        }
        self.selected_project_id = projects.first().map(|item| item.id.clone());
        self.projects = projects;
    }

    pub fn apply_project_list(
        &mut self,
        projects: Vec<RemoteRuntimeProject>,
        remote_selected_project_id: Option<String>,
        terminal_visible: bool,
        terminal_list_loaded: bool,
    ) -> RemoteRuntimePlan {
        let previous_selected = self.selected_project_id.clone();
        let remote_selected_project_id = clean_optional_string(remote_selected_project_id);
        let confirms_pending_project_select = self
            .pending_project_select_id
            .as_deref()
            .is_some_and(|pending| remote_selected_project_id.as_deref() == Some(pending));
        let selected = selected_project_from_list(
            &projects,
            self.pending_project_select_id.as_deref(),
            remote_selected_project_id.as_deref(),
            previous_selected.as_deref(),
            self.active_session_id.is_some(),
        );
        let project_changed = selected != previous_selected;
        self.projects = projects;
        self.selected_project_id = selected;
        if project_changed {
            self.active_session_id = None;
        }
        if confirms_pending_project_select {
            self.project_select_acknowledged_id = self.pending_project_select_id.clone();
            self.pending_project_select_id = None;
            self.pending_project_select_sent = false;
        }
        let bind =
            self.ensure_terminal_for_selected_project(terminal_visible, terminal_list_loaded);
        RemoteRuntimePlan {
            state_changed: true,
            clear_terminal: project_changed && terminal_visible,
            reset_terminal_input: project_changed && terminal_visible,
            reset_terminal_buffer: (project_changed && terminal_visible)
                || bind.reset_terminal_buffer,
            request_terminal_list: bind.request_terminal_list,
            request_project_select_id: bind.request_project_select_id,
            bind_session_id: bind.bind_session_id,
            bind_full_buffer: bind.bind_full_buffer,
            flush_terminal_input: bind.flush_terminal_input,
            removed_session_id: None,
        }
    }

    pub fn apply_terminal_list(
        &mut self,
        terminals: Vec<RemoteRuntimeTerminal>,
        terminal_visible: bool,
        terminal_list_loaded: bool,
    ) -> RemoteRuntimePlan {
        let active_missing = self.active_session_id.as_ref().is_some_and(|active_id| {
            !terminals
                .iter()
                .any(|item| item.id == *active_id && is_accessible_terminal(item))
        });
        let removed_session_id = if active_missing {
            self.active_session_id.clone()
        } else {
            None
        };
        if let Some(removed) = removed_session_id.as_ref() {
            self.last_terminal_id_by_project
                .retain(|_, terminal_id| terminal_id != removed);
            self.active_session_id = None;
        }
        self.terminals = terminals;
        let bind =
            self.ensure_terminal_for_selected_project(terminal_visible, terminal_list_loaded);
        RemoteRuntimePlan {
            state_changed: true,
            reset_terminal_input: active_missing,
            reset_terminal_buffer: active_missing || bind.reset_terminal_buffer,
            removed_session_id,
            request_terminal_list: bind.request_terminal_list,
            request_project_select_id: bind.request_project_select_id,
            bind_session_id: bind.bind_session_id,
            bind_full_buffer: bind.bind_full_buffer,
            flush_terminal_input: bind.flush_terminal_input,
            clear_terminal: false,
        }
    }

    pub fn user_select_project(
        &mut self,
        project: RemoteRuntimeProject,
        terminal_visible: bool,
    ) -> RemoteRuntimePlan {
        let project_changed = self.selected_project_id.as_deref() != Some(project.id.as_str());
        let previous_project_id = self.selected_project_id.clone();
        if project_changed
            && let (Some(previous_project_id), Some(active_session_id)) = (
                previous_project_id.as_ref(),
                self.active_session_id.as_ref(),
            )
            && self.terminals.iter().any(|item| {
                item.id == *active_session_id
                    && item.project_id == *previous_project_id
                    && is_accessible_terminal(item)
            })
        {
            self.last_terminal_id_by_project
                .insert(previous_project_id.clone(), active_session_id.clone());
        }
        let existing = if terminal_visible {
            accessible_terminals_for_project(&self.terminals, &project.id)
        } else {
            Vec::new()
        };
        let terminal = preferred_terminal_for_project(
            &self.last_terminal_id_by_project,
            &project.id,
            &existing,
        )
        .cloned();
        if let Some(terminal) = terminal.as_ref() {
            self.last_terminal_id_by_project
                .insert(project.id.clone(), terminal.id.clone());
        }
        self.selected_project_id = Some(project.id.clone());
        self.active_session_id = terminal.as_ref().map(|item| item.id.clone()).or_else(|| {
            if project_changed && terminal_visible {
                None
            } else {
                self.active_session_id.clone()
            }
        });
        self.pending_project_select_id = Some(project.id.clone());
        self.pending_project_select_sent = false;
        self.project_select_acknowledged_id = None;
        RemoteRuntimePlan {
            state_changed: true,
            clear_terminal: project_changed && terminal_visible,
            reset_terminal_input: project_changed && terminal_visible,
            reset_terminal_buffer: project_changed && terminal_visible,
            request_terminal_list: terminal_visible && terminal.is_none(),
            request_project_select_id: Some(project.id),
            bind_session_id: terminal.as_ref().map(|item| item.id.clone()),
            bind_full_buffer: terminal.is_some(),
            flush_terminal_input: terminal.is_some(),
            removed_session_id: None,
        }
    }

    pub fn project_selected(&mut self, project_id: Option<String>) -> RemoteRuntimePlan {
        let Some(selected) = clean_optional_string(project_id) else {
            return RemoteRuntimePlan::default();
        };
        if let Some(pending) = self.pending_project_select_id.as_deref()
            && pending != selected
        {
            return RemoteRuntimePlan::default();
        }
        if self.selected_project_id.as_deref() != Some(selected.as_str())
            && !self.projects.iter().any(|item| item.id == selected)
        {
            return RemoteRuntimePlan::default();
        }
        let project_changed = self.selected_project_id.as_deref() != Some(selected.as_str());
        self.selected_project_id = Some(selected.clone());
        if project_changed {
            self.active_session_id = None;
        }
        self.pending_project_select_id = None;
        self.pending_project_select_sent = false;
        self.project_select_acknowledged_id = Some(selected);
        RemoteRuntimePlan {
            state_changed: true,
            reset_terminal_input: true,
            reset_terminal_buffer: true,
            request_terminal_list: true,
            ..RemoteRuntimePlan::default()
        }
    }

    pub fn ensure_terminal_for_selected_project(
        &mut self,
        terminal_visible: bool,
        terminal_list_loaded: bool,
    ) -> RemoteRuntimePlan {
        if !terminal_visible {
            return RemoteRuntimePlan::default();
        }
        let Some(project_id) = self.selected_project_id.clone() else {
            return RemoteRuntimePlan::default();
        };
        if !terminal_list_loaded {
            return RemoteRuntimePlan {
                request_terminal_list: true,
                ..RemoteRuntimePlan::default()
            };
        }
        if let Some(active_id) = self.active_session_id.as_ref()
            && self.terminals.iter().any(|item| {
                item.id == *active_id
                    && item.project_id == project_id
                    && is_accessible_terminal(item)
            })
        {
            return RemoteRuntimePlan::default();
        }
        let existing = accessible_terminals_for_project(&self.terminals, &project_id);
        if existing.is_empty() {
            if self.pending_project_select_id.as_deref() == Some(project_id.as_str()) {
                if self.pending_project_select_sent {
                    return RemoteRuntimePlan::default();
                }
                return RemoteRuntimePlan {
                    request_project_select_id: Some(project_id),
                    ..RemoteRuntimePlan::default()
                };
            }
            if self.project_select_acknowledged_id.as_deref() == Some(project_id.as_str()) {
                return RemoteRuntimePlan::default();
            }
            self.pending_project_select_id = Some(project_id.clone());
            self.pending_project_select_sent = false;
            self.project_select_acknowledged_id = None;
            return RemoteRuntimePlan {
                request_terminal_list: true,
                request_project_select_id: Some(project_id),
                ..RemoteRuntimePlan::default()
            };
        }
        let terminal = preferred_terminal_for_project(
            &self.last_terminal_id_by_project,
            &project_id,
            &existing,
        )
        .expect("existing terminals are not empty");
        self.active_session_id = Some(terminal.id.clone());
        if self.pending_project_select_id.as_deref() != Some(project_id.as_str()) {
            self.pending_project_select_id = None;
            self.pending_project_select_sent = false;
            self.project_select_acknowledged_id = None;
        }
        self.creating_terminal_project_id = None;
        self.last_terminal_id_by_project
            .insert(project_id, terminal.id.clone());
        RemoteRuntimePlan {
            state_changed: true,
            reset_terminal_buffer: true,
            bind_session_id: Some(terminal.id.clone()),
            bind_full_buffer: true,
            flush_terminal_input: true,
            ..RemoteRuntimePlan::default()
        }
    }

    pub fn select_terminal(&mut self, terminal: RemoteRuntimeTerminal) -> RemoteRuntimePlan {
        if !is_accessible_terminal(&terminal) {
            return RemoteRuntimePlan::default();
        }
        self.last_terminal_id_by_project
            .insert(terminal.project_id.clone(), terminal.id.clone());
        self.selected_project_id = Some(terminal.project_id.clone());
        self.active_session_id = Some(terminal.id.clone());
        self.pending_project_select_id = None;
        self.pending_project_select_sent = false;
        self.project_select_acknowledged_id = None;
        self.creating_terminal_project_id = None;
        RemoteRuntimePlan {
            state_changed: true,
            reset_terminal_input: true,
            reset_terminal_buffer: true,
            bind_session_id: Some(terminal.id),
            bind_full_buffer: true,
            ..RemoteRuntimePlan::default()
        }
    }

    pub fn remove_terminal(&mut self, terminal_id: &str) -> RemoteRuntimePlan {
        let closing_active = self.active_session_id.as_deref() == Some(terminal_id);
        self.terminals.retain(|item| item.id != terminal_id);
        self.last_terminal_id_by_project
            .retain(|_, id| id != terminal_id);
        if closing_active {
            self.active_session_id = None;
        }
        RemoteRuntimePlan {
            state_changed: true,
            clear_terminal: closing_active,
            reset_terminal_input: closing_active,
            reset_terminal_buffer: closing_active,
            removed_session_id: Some(terminal_id.to_string()),
            ..RemoteRuntimePlan::default()
        }
    }

    pub fn set_terminal_creating_project(&mut self, project_id: Option<String>) {
        self.creating_terminal_project_id = clean_optional_string(project_id);
    }

    pub fn terminal_created(&mut self, terminal: RemoteRuntimeTerminal) -> RemoteRuntimePlan {
        if !is_accessible_terminal(&terminal) {
            return RemoteRuntimePlan::default();
        }
        self.terminals.retain(|item| item.id != terminal.id);
        self.terminals.insert(0, terminal.clone());
        self.last_terminal_id_by_project
            .insert(terminal.project_id.clone(), terminal.id.clone());
        self.selected_project_id = Some(terminal.project_id);
        self.active_session_id = Some(terminal.id.clone());
        self.pending_project_select_id = None;
        self.pending_project_select_sent = false;
        self.project_select_acknowledged_id = None;
        self.creating_terminal_project_id = None;
        RemoteRuntimePlan {
            state_changed: true,
            clear_terminal: true,
            reset_terminal_buffer: true,
            bind_session_id: Some(terminal.id),
            bind_full_buffer: true,
            flush_terminal_input: true,
            ..RemoteRuntimePlan::default()
        }
    }

    pub fn mark_project_select_sent(&mut self, project_id: &str) {
        if self.pending_project_select_id.as_deref() == Some(project_id) {
            self.pending_project_select_sent = true;
        }
    }

    pub fn clear_project_select_sent(&mut self, project_id: &str) {
        if self.pending_project_select_id.as_deref() == Some(project_id) {
            self.pending_project_select_sent = false;
        }
    }

    pub fn pending_project_select(&self, include_sent: bool) -> Option<String> {
        let project_id = self.pending_project_select_id.as_ref()?;
        if project_id.is_empty() || (!include_sent && self.pending_project_select_sent) {
            return None;
        }
        Some(project_id.clone())
    }

    pub fn current_project_terminals(&self) -> Vec<RemoteRuntimeTerminal> {
        let Some(project_id) = self.selected_project_id.as_ref() else {
            return Vec::new();
        };
        let mut terminals = accessible_terminals_for_project(&self.terminals, project_id);
        terminals.sort_by(|left, right| compare_remote_terminals(left, right));
        terminals.into_iter().cloned().collect()
    }
}

fn default_terminal_layout_kind() -> String {
    "split".to_string()
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn selected_project_from_list(
    projects: &[RemoteRuntimeProject],
    pending_project_select_id: Option<&str>,
    remote_selected_project_id: Option<&str>,
    current_selected_project_id: Option<&str>,
    prefer_current_project: bool,
) -> Option<String> {
    let pending = pending_project_select_id.and_then(|value| {
        let value = value.trim();
        if !value.is_empty() && projects.iter().any(|item| item.id == value) {
            Some(value.to_string())
        } else {
            None
        }
    });
    if pending.is_some() {
        return pending;
    }
    if prefer_current_project
        && let Some(current) = current_selected_project_id
        && projects.iter().any(|item| item.id == current)
    {
        return Some(current.to_string());
    }
    let remote = remote_selected_project_id.and_then(|value| {
        let value = value.trim();
        if !value.is_empty() && projects.iter().any(|item| item.id == value) {
            Some(value.to_string())
        } else {
            None
        }
    });
    if remote.is_some() {
        return remote;
    }
    if let Some(current) = current_selected_project_id
        && projects.iter().any(|item| item.id == current)
    {
        return Some(current.to_string());
    }
    projects.first().map(|item| item.id.clone())
}

fn is_accessible_terminal(terminal: &RemoteRuntimeTerminal) -> bool {
    !terminal.id.is_empty() && !terminal.project_id.is_empty()
}

fn accessible_terminals_for_project<'a>(
    terminals: &'a [RemoteRuntimeTerminal],
    project_id: &str,
) -> Vec<&'a RemoteRuntimeTerminal> {
    terminals
        .iter()
        .filter(|item| item.project_id == project_id && is_accessible_terminal(item))
        .collect()
}

fn preferred_terminal_for_project<'a>(
    last_terminal_id_by_project: &HashMap<String, String>,
    project_id: &str,
    terminals: &'a [&'a RemoteRuntimeTerminal],
) -> Option<&'a RemoteRuntimeTerminal> {
    let mut list = terminals.to_vec();
    list.sort_by(|left, right| compare_remote_terminals(left, right));
    if let Some(remembered_id) = last_terminal_id_by_project.get(project_id)
        && let Some(terminal) = list.iter().find(|terminal| terminal.id == *remembered_id)
    {
        return Some(*terminal);
    }
    if let Some(terminal) = list
        .iter()
        .find(|terminal| terminal_layout_kind(terminal) == "split")
    {
        return Some(*terminal);
    }
    list.first().copied()
}

fn compare_remote_terminals(
    left: &RemoteRuntimeTerminal,
    right: &RemoteRuntimeTerminal,
) -> std::cmp::Ordering {
    left.created_at
        .as_deref()
        .unwrap_or_default()
        .cmp(right.created_at.as_deref().unwrap_or_default())
        .then_with(|| left.id.cmp(&right.id))
}

fn terminal_layout_kind(terminal: &RemoteRuntimeTerminal) -> &str {
    if terminal.layout_kind.trim().eq_ignore_ascii_case("tab") {
        "tab"
    } else {
        "split"
    }
}
