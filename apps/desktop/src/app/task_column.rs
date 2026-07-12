use chrono::{DateTime, Local};
use gpui_component::{
    InteractiveElementExt as _, Selectable as _, WindowExt as _, button::ButtonVariant,
    dialog::DialogButtonProps, menu::ContextMenuExt as _,
};

use super::agent_display::{agent_lifecycle_color, agent_lifecycle_status_dot, spin_icon};
use super::ai_runtime_status::AgentLifecycleState;
use super::scroll_compat::wecode_uniform_list_with_sizing;
use super::ui_helpers::{titlebar_drag_area, wecode_tooltip_container};
use super::{
    formatting::{relative_time_label_for_language, usage_amount_label},
    *,
};
use gpui::ListSizingBehavior;

pub(in crate::app) struct TaskColumnView {
    app_entity: gpui::Entity<WeCodeApp>,
    header_view: gpui::Entity<TaskColumnHeaderView>,
    worktree_list_view: gpui::Entity<TaskWorktreeListView>,
    branch_list_view: gpui::Entity<TaskBranchListView>,
    terminal_list_view: gpui::Entity<TaskTerminalListView>,
    session_list_view: gpui::Entity<TaskSessionListView>,
    active_tab: TaskColumnPrimaryTab,
    active_git_tab: TaskGitTab,
    worktree_count: usize,
    branch_count: usize,
    terminal_count: usize,
    session_count: usize,
    labels: TaskColumnLabels,
}

#[derive(Clone)]
pub(in crate::app) struct TaskSessionDrag {
    pub(in crate::app) session_id: String,
    pub(in crate::app) title: String,
}

impl Render for TaskSessionDrag {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py(px(6.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_sm()
            .text_color(color(theme::TEXT))
            .max_w(px(220.0))
            .truncate()
            .child(self.title.clone())
    }
}

#[derive(Clone, PartialEq)]
struct TaskWorktreeRow {
    id: String,
    project_id: String,
    title: String,
    path: String,
    is_default: bool,
    active: bool,
    git_changes: usize,
    git_additions: i64,
    git_deletions: i64,
    lifecycle: Option<AgentLifecycleState>,
}

#[derive(Clone, PartialEq)]
struct TaskSessionRow {
    id: String,
    session_key: String,
    external_session_id: Option<String>,
    title: String,
    source: String,
    last_model: Option<String>,
    first_seen_at: f64,
    last_seen_at: f64,
    total_tokens: i64,
    usage_amounts: Vec<wecode_runtime::ai_history::AIUsageAmount>,
    active: bool,
    pinned: bool,
    archived: bool,
}

impl Render for TaskColumnView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        task_column_content(
            self.app_entity.clone(),
            self.header_view.clone(),
            self.worktree_list_view.clone(),
            self.branch_list_view.clone(),
            self.terminal_list_view.clone(),
            self.session_list_view.clone(),
            self.active_tab,
            self.active_git_tab,
            self.worktree_count,
            self.branch_count,
            self.terminal_count,
            self.session_count,
            self.labels.clone(),
            cx,
        )
        .into_any_element()
    }
}

impl WeCodeApp {
    pub(in crate::app) fn task_column_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskColumnView> {
        let active_tab = self.task_column_primary_tab;
        let active_git_tab = self.task_git_tab;
        let worktree_count = self.state.worktrees.worktrees.len();
        let branch_count = self.state.git.branches.len();
        let terminal_count = self
            .main_terminal()
            .map(|tab| tab.panes.len())
            .unwrap_or_default()
            + self.collapsed_terminal_panes.len();
        let session_metadata = self.runtime_service.ai_session_metadata();
        let session_count = self
            .state
            .ai_history
            .sessions
            .iter()
            .filter(|session| {
                !session_metadata
                    .get(&session.id)
                    .is_some_and(|metadata| metadata.archived)
            })
            .count();
        let labels = task_column_labels(&self.state.settings.language);
        if let Some(view) = self.task_column_view.clone() {
            self.update_task_column_child_views(cx);
            view.update(cx, |view, cx| {
                if view.active_tab != active_tab
                    || view.worktree_count != worktree_count
                    || view.active_git_tab != active_git_tab
                    || view.branch_count != branch_count
                    || view.terminal_count != terminal_count
                    || view.session_count != session_count
                    || view.labels != labels
                {
                    view.active_tab = active_tab;
                    view.worktree_count = worktree_count;
                    view.active_git_tab = active_git_tab;
                    view.branch_count = branch_count;
                    view.terminal_count = terminal_count;
                    view.session_count = session_count;
                    view.labels = labels;
                    cx.notify();
                }
            });
            return view;
        }
        let header_view = self.task_column_header_view(cx);
        let worktree_list_view = self.task_worktree_list_view(cx);
        let branch_list_view = self.task_branch_list_view(cx);
        let terminal_list_view = self.task_terminal_list_view(cx);
        let session_list_view = self.task_session_list_view(cx);
        let app_entity = cx.entity();
        let view = cx.new(|_| TaskColumnView {
            app_entity,
            header_view,
            worktree_list_view,
            branch_list_view,
            terminal_list_view,
            session_list_view,
            active_tab,
            active_git_tab,
            worktree_count,
            branch_count,
            terminal_count,
            session_count,
            labels,
        });
        self.task_column_view = Some(view.clone());
        view
    }

    pub(in crate::app) fn update_task_column_child_views(&mut self, cx: &mut Context<Self>) {
        let _ = self.task_column_header_view(cx);
        let _ = self.task_worktree_list_view(cx);
        let _ = self.task_branch_list_view(cx);
        let _ = self.task_terminal_list_view(cx);
        let _ = self.task_session_list_view(cx);
    }
}

#[derive(Clone, PartialEq)]
struct TaskColumnLabels {
    language: String,
    no_project: String,
    no_worktrees_title: String,
    no_branches_title: String,
    no_sessions_title: String,
    no_branch: String,
    sessions: String,
    terminals: String,
    git: String,
    branches: String,
    worktrees: String,
    current: String,
    changed_format: String,
    create: String,
    new_branch: String,
    branch_name: String,
    refresh: String,
    open: String,
    rename: String,
    close: String,
    cancel: String,
    close_terminal_title: String,
    close_terminal_message_format: String,
    new_terminal: String,
    new_session: String,
    open_folder: String,
    merge: String,
    delete: String,
    bind_wechat: String,
    wechat_bound: String,
    archived: String,
    filter: String,
    all: String,
    pin: String,
    unpin: String,
    archive: String,
    unarchive: String,
    sort: String,
    sort_updated: String,
    sort_created: String,
}

fn task_column_labels(language: &str) -> TaskColumnLabels {
    let locale = locale_from_language_setting(language);
    let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
    TaskColumnLabels {
        language: language.to_string(),
        no_project: tr("files.panel.no_project", "No project selected"),
        no_worktrees_title: tr("worktree.sidebar.empty_title", "No worktrees yet"),
        no_branches_title: tr("git.branch.empty", "No branches"),
        no_sessions_title: tr("ai.sessions.empty", "No Sessions"),
        no_branch: tr("git.branch.none", "No Branch"),
        sessions: tr("ai.sessions.history", "Session History"),
        terminals: tr("terminal.title", "Terminal"),
        git: tr("titlebar.git", "Git"),
        branches: tr("git.branches.title", "Branches"),
        worktrees: tr("worktree.title", "Worktree"),
        current: tr("common.current", "Current"),
        changed_format: tr("worktree.sidebar.changed_format", "%@ changed"),
        create: tr("worktree.create.title", "New Worktree"),
        new_branch: tr("git.branch.new", "New Branch"),
        branch_name: tr("git.branch.name", "Branch name"),
        refresh: tr("common.refresh", "Refresh"),
        open: tr("common.open", "Open"),
        rename: tr("common.rename", "Rename"),
        close: tr("common.close", "Close"),
        cancel: tr("common.cancel", "Cancel"),
        close_terminal_title: tr("terminal.close.title", "Close Terminal"),
        close_terminal_message_format: tr("terminal.close.message_format", "Close %@?"),
        new_terminal: tr("terminal.new", "New Terminal"),
        new_session: tr("ai.sessions.new_session", "New Session"),
        open_folder: tr("worktree.menu.open_folder", "Open Folder"),
        merge: tr("worktree.menu.merge", "Merge to Mainline"),
        delete: tr("common.delete", "Delete"),
        bind_wechat: tr("terminal.wechat.bind", "Bind WeChat to this terminal"),
        wechat_bound: tr("terminal.wechat.bound", "WeChat bound"),
        archived: tr("common.archived", "Archived"),
        filter: tr("common.filter", "Filter"),
        all: tr("common.all", "All"),
        pin: tr("common.pin", "Pin"),
        unpin: tr("common.unpin", "Unpin"),
        archive: tr("common.archive", "Archive"),
        unarchive: tr("common.unarchive", "Unarchive"),
        sort: tr("common.sort", "Sort"),
        sort_updated: tr("ai.sessions.sort_updated", "Recently updated"),
        sort_created: tr("ai.sessions.sort_created", "Creation time"),
    }
}

fn task_session_row(
    session: &AISessionSummary,
    active_ai_session_id: Option<&str>,
    metadata: Option<&wecode_runtime::ai_session_metadata::AISessionMetadata>,
) -> TaskSessionRow {
    TaskSessionRow {
        id: session.id.clone(),
        session_key: session.session_key.clone(),
        external_session_id: session.external_session_id.clone(),
        title: session.title.clone(),
        source: session.source.clone(),
        last_model: session.last_model.clone(),
        first_seen_at: session.first_seen_at,
        last_seen_at: session.last_seen_at,
        total_tokens: session.total_tokens,
        usage_amounts: session.usage_amounts.clone(),
        active: active_ai_session_id
            .map(|id| {
                id == session.id
                    || id == session.session_key
                    || session.external_session_id.as_deref() == Some(id)
            })
            .unwrap_or(false),
        pinned: metadata.is_some_and(|metadata| metadata.pinned),
        archived: metadata.is_some_and(|metadata| metadata.archived),
    }
}

fn task_branch_rows(branches: &[GitBranchSummary]) -> Vec<TaskBranchRow> {
    let mut rows = branches
        .iter()
        .map(|branch| TaskBranchRow {
            name: branch.name.clone(),
            is_current: branch.is_current,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    rows
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TaskColumnHeaderSnapshot {
    project_name: String,
    refreshing: bool,
    create_label: String,
    create_branch: bool,
    refresh_label: String,
    language: String,
    local_branches: Vec<(String, bool)>,
    remote_branches: Vec<String>,
}

pub(in crate::app) struct TaskColumnHeaderView {
    app_entity: gpui::Entity<WeCodeApp>,
    snapshot: TaskColumnHeaderSnapshot,
}

impl TaskColumnHeaderView {
    fn set_snapshot(&mut self, snapshot: TaskColumnHeaderSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TaskColumnHeaderView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        task_column_header(
            self.snapshot.project_name.clone(),
            self.snapshot.refreshing,
            self.snapshot.create_label.clone(),
            self.snapshot.create_branch,
            self.snapshot.refresh_label.clone(),
            self.snapshot.language.clone(),
            self.snapshot.local_branches.clone(),
            self.snapshot.remote_branches.clone(),
            self.app_entity.clone(),
            cx,
        )
        .into_any_element()
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TaskWorktreeListSnapshot {
    labels: TaskColumnLabels,
    worktrees: Vec<TaskWorktreeRow>,
}

pub(in crate::app) struct TaskWorktreeListView {
    app_entity: gpui::Entity<WeCodeApp>,
    snapshot: TaskWorktreeListSnapshot,
    scroll_handle: UniformListScrollHandle,
}

impl TaskWorktreeListView {
    fn set_snapshot(&mut self, snapshot: TaskWorktreeListSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TaskWorktreeListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        task_list_area(
            self.snapshot.worktrees.clone(),
            self.snapshot.labels.clone(),
            self.scroll_handle.clone(),
            self.app_entity.clone(),
            cx,
        )
        .into_any_element()
    }
}

#[derive(Clone, PartialEq)]
struct TaskBranchRow {
    name: String,
    is_current: bool,
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TaskBranchListSnapshot {
    labels: TaskColumnLabels,
    branches: Vec<TaskBranchRow>,
}

pub(in crate::app) struct TaskBranchListView {
    app_entity: gpui::Entity<WeCodeApp>,
    snapshot: TaskBranchListSnapshot,
    scroll_handle: UniformListScrollHandle,
}

impl TaskBranchListView {
    fn set_snapshot(&mut self, snapshot: TaskBranchListSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TaskBranchListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        branch_list_area(
            self.snapshot.branches.clone(),
            self.snapshot.labels.clone(),
            self.scroll_handle.clone(),
            self.app_entity.clone(),
            cx,
        )
        .into_any_element()
    }
}

#[derive(Clone, PartialEq)]
struct TaskTerminalRow {
    terminal_id: Option<String>,
    pane_index: usize,
    title: String,
    subtitle: Option<String>,
    created_at: Option<f64>,
    lifecycle: Option<AgentLifecycleState>,
    running: bool,
    active: bool,
    wechat_bound: bool,
    collapsed: bool,
    collapsed_index: Option<usize>,
}

#[derive(Clone)]
enum TaskTerminalListItem {
    Terminal(TaskTerminalRow),
    Create,
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TaskTerminalListSnapshot {
    labels: TaskColumnLabels,
    terminals: Vec<TaskTerminalRow>,
}

pub(in crate::app) struct TaskTerminalListView {
    app_entity: gpui::Entity<WeCodeApp>,
    snapshot: TaskTerminalListSnapshot,
    scroll_handle: UniformListScrollHandle,
}

impl TaskTerminalListView {
    fn set_snapshot(&mut self, snapshot: TaskTerminalListSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        if self.snapshot.terminals.len() != snapshot.terminals.len() {
            wecode_runtime::runtime_trace::runtime_trace(
                "task-terminal-list",
                &format!("rows={}", snapshot.terminals.len()),
            );
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TaskTerminalListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        terminal_list_area(
            self.snapshot.terminals.clone(),
            self.snapshot.labels.clone(),
            self.scroll_handle.clone(),
            self.app_entity.clone(),
            cx,
        )
        .into_any_element()
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TaskSessionListSnapshot {
    labels: TaskColumnLabels,
    sessions: Vec<TaskSessionRow>,
    filter: TaskSessionFilter,
    sort: TaskSessionSort,
    source_filter: TaskSessionSourceFilter,
}

pub(in crate::app) struct TaskSessionListView {
    app_entity: gpui::Entity<WeCodeApp>,
    snapshot: TaskSessionListSnapshot,
    scroll_handle: UniformListScrollHandle,
}

impl TaskSessionListView {
    fn set_snapshot(&mut self, snapshot: TaskSessionListSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TaskSessionListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        recent_session_area(
            self.snapshot.sessions.clone(),
            self.snapshot.labels.clone(),
            self.snapshot.filter,
            self.snapshot.sort,
            self.snapshot.source_filter,
            self.scroll_handle.clone(),
            self.app_entity.clone(),
            cx,
        )
        .into_any_element()
    }
}

impl WeCodeApp {
    fn task_column_header_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskColumnHeaderView> {
        let snapshot = self.task_column_header_snapshot();
        if let Some(view) = self.task_column_header_view.clone() {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view;
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| TaskColumnHeaderView {
            app_entity,
            snapshot,
        });
        self.task_column_header_view = Some(view.clone());
        view
    }

    fn task_worktree_list_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskWorktreeListView> {
        let snapshot = self.task_worktree_list_snapshot();
        if let Some(view) = self.task_worktree_list_view.clone() {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view;
        }
        let app_entity = cx.entity();
        let scroll_handle = self.task_scroll_handle.clone();
        let view = cx.new(|_| TaskWorktreeListView {
            app_entity,
            snapshot,
            scroll_handle,
        });
        self.task_worktree_list_view = Some(view.clone());
        view
    }

    fn task_branch_list_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskBranchListView> {
        let snapshot = self.task_branch_list_snapshot();
        if let Some(view) = self.task_branch_list_view.clone() {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view;
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| TaskBranchListView {
            app_entity,
            snapshot,
            scroll_handle: UniformListScrollHandle::new(),
        });
        self.task_branch_list_view = Some(view.clone());
        view
    }

    fn task_terminal_list_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskTerminalListView> {
        let snapshot = self.task_terminal_list_snapshot();
        if let Some(view) = self.task_terminal_list_view.clone() {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view;
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| TaskTerminalListView {
            app_entity,
            snapshot,
            scroll_handle: UniformListScrollHandle::new(),
        });
        self.task_terminal_list_view = Some(view.clone());
        view
    }

    fn task_session_list_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TaskSessionListView> {
        let snapshot = self.task_session_list_snapshot();
        if let Some(view) = self.task_session_list_view.clone() {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view;
        }
        let app_entity = cx.entity();
        let scroll_handle = self.session_scroll_handle.clone();
        let view = cx.new(|_| TaskSessionListView {
            app_entity,
            snapshot,
            scroll_handle,
        });
        self.task_session_list_view = Some(view.clone());
        view
    }

    fn task_column_header_snapshot(&self) -> TaskColumnHeaderSnapshot {
        let labels = task_column_labels(&self.state.settings.language);
        let create_branch = self.task_column_primary_tab == TaskColumnPrimaryTab::Git
            && self.task_git_tab == TaskGitTab::Branches;
        TaskColumnHeaderSnapshot {
            project_name: self
                .state
                .selected_project
                .as_ref()
                .map(|project| project.name.clone())
                .unwrap_or(labels.no_project),
            refreshing: self.task_column_refreshing,
            create_label: if create_branch {
                labels.new_branch
            } else {
                labels.create
            },
            create_branch,
            refresh_label: labels.refresh,
            language: self.state.settings.language.clone(),
            local_branches: self
                .state
                .git
                .branches
                .iter()
                .map(|branch| (branch.name.clone(), branch.is_current))
                .collect(),
            remote_branches: self.state.git.remote_branches.clone(),
        }
    }

    fn task_worktree_list_snapshot(&self) -> TaskWorktreeListSnapshot {
        let labels = task_column_labels(&self.state.settings.language);
        let selected_worktree_id = self.state.worktrees.selected_worktree_id.clone();
        let worktrees = self
            .state
            .worktrees
            .worktrees
            .iter()
            .map(|worktree| {
                let active = selected_worktree_id
                    .as_deref()
                    .map(|id| id == worktree.id)
                    .unwrap_or(false);
                TaskWorktreeRow {
                    id: worktree.id.clone(),
                    project_id: worktree.project_id.clone(),
                    title: worktree_row_title(worktree, &labels.no_branch),
                    path: worktree.path.clone(),
                    is_default: worktree.is_default,
                    active,
                    git_changes: worktree.git_summary.changes,
                    git_additions: worktree.git_summary.additions,
                    git_deletions: worktree.git_summary.deletions,
                    lifecycle: self.worktree_agent_lifecycle(worktree),
                }
            })
            .collect();

        TaskWorktreeListSnapshot { labels, worktrees }
    }

    fn task_branch_list_snapshot(&self) -> TaskBranchListSnapshot {
        let labels = task_column_labels(&self.state.settings.language);
        let branches = task_branch_rows(&self.state.git.branches);
        TaskBranchListSnapshot { labels, branches }
    }

    fn task_terminal_list_snapshot(&self) -> TaskTerminalListSnapshot {
        let labels = task_column_labels(&self.state.settings.language);
        let ai_titles = terminal_ai_titles_by_terminal_id(&self.state.ai_runtime_state.sessions);
        let active_terminal_id = self.active_terminal_runtime_id();
        let wechat_bound_session_ids =
            wecode_runtime::wechat_bridge_service::wechat_bridge_bound_session_ids();
        let mut terminals = self
            .main_terminal()
            .map(|tab| {
                tab.panes
                    .iter()
                    .enumerate()
                    .map(|(index, slot)| {
                        let terminal_id = Self::terminal_slot_terminal_id(tab, index, slot);
                        let osc_title = terminal_id
                            .as_deref()
                            .and_then(|id| self.terminal_osc_titles.get(id));
                        let (title, subtitle) = terminal_pane_display_title(
                            slot,
                            &ai_titles,
                            osc_title.map(String::as_str),
                            &labels.language,
                        );
                        let lifecycle = terminal_id
                            .as_deref()
                            .and_then(|id| self.pane_agent_lifecycle.get(id))
                            .map(|lifecycle| lifecycle.state);
                        let active = !active_terminal_id.is_empty()
                            && terminal_id.as_deref() == Some(active_terminal_id.as_str());
                        let created_at = terminal_id.as_deref().and_then(|id| {
                            self.state
                                .terminal_runtime
                                .sessions
                                .iter()
                                .find(|session| session.terminal_id == id)
                                .map(|session| session.created_at)
                                .filter(|created_at| *created_at > 0.0)
                        });
                        let running = terminal_id.as_deref().is_some_and(|id| {
                            self.state.ai_runtime_state.sessions.iter().any(|session| {
                                session.terminal_id == id
                                    && matches!(
                                        session.state.as_str(),
                                        "responding" | "needsInput" | "working" | "running"
                                    )
                            })
                        });
                        TaskTerminalRow {
                            wechat_bound: terminal_id.as_deref().is_some_and(|id| {
                                wechat_bound_session_ids.iter().any(|bound| bound == id)
                            }),
                            terminal_id,
                            pane_index: index,
                            title,
                            subtitle,
                            created_at,
                            lifecycle,
                            running,
                            active,
                            collapsed: false,
                            collapsed_index: None,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for (collapsed_index, slot) in self.collapsed_terminal_panes.iter().enumerate() {
            let osc_title = slot
                .terminal_id
                .as_deref()
                .and_then(|id| self.terminal_osc_titles.get(id));
            let (title, subtitle) = terminal_pane_display_title(
                slot,
                &ai_titles,
                osc_title.map(String::as_str),
                &labels.language,
            );
            terminals.push(TaskTerminalRow {
                wechat_bound: slot
                    .terminal_id
                    .as_deref()
                    .is_some_and(|id| wechat_bound_session_ids.iter().any(|bound| bound == id)),
                terminal_id: slot.terminal_id.clone(),
                pane_index: 0,
                title,
                subtitle,
                created_at: slot.terminal_id.as_deref().and_then(|id| {
                    self.state
                        .terminal_runtime
                        .sessions
                        .iter()
                        .find(|session| session.terminal_id == id)
                        .map(|session| session.created_at)
                        .filter(|created_at| *created_at > 0.0)
                }),
                lifecycle: slot
                    .terminal_id
                    .as_deref()
                    .and_then(|id| self.pane_agent_lifecycle.get(id))
                    .map(|lifecycle| lifecycle.state),
                running: slot.terminal_id.as_deref().is_some_and(|id| {
                    self.state.ai_runtime_state.sessions.iter().any(|session| {
                        session.terminal_id == id
                            && matches!(
                                session.state.as_str(),
                                "responding" | "needsInput" | "working" | "running"
                            )
                    })
                }),
                active: false,
                collapsed: true,
                collapsed_index: Some(collapsed_index),
            });
        }

        TaskTerminalListSnapshot { labels, terminals }
    }

    fn task_session_list_snapshot(&self) -> TaskSessionListSnapshot {
        let labels = task_column_labels(&self.state.settings.language);
        let metadata = self.runtime_service.ai_session_metadata();
        let active_terminal_id = self.active_terminal_runtime_id();
        let active_ai_session_id = (!active_terminal_id.is_empty())
            .then(|| {
                self.state
                    .ai_runtime_state
                    .sessions
                    .iter()
                    .find(|session| session.terminal_id == active_terminal_id)
                    .and_then(|session| session.ai_session_id.as_deref())
            })
            .flatten();
        let mut sessions = self
            .state
            .ai_history
            .sessions
            .iter()
            .map(|session| {
                task_session_row(session, active_ai_session_id, metadata.get(&session.id))
            })
            .filter(|session| task_session_matches_filter(session, self.task_session_filter))
            .filter(|session| task_session_matches_source(session, self.task_session_source_filter))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            let time_order = match self.task_session_sort {
                TaskSessionSort::UpdatedAt => right.last_seen_at.total_cmp(&left.last_seen_at),
                TaskSessionSort::CreatedAt => right.first_seen_at.total_cmp(&left.first_seen_at),
            };
            right.pinned.cmp(&left.pinned).then(time_order)
        });

        TaskSessionListSnapshot {
            labels,
            sessions,
            filter: self.task_session_filter,
            sort: self.task_session_sort,
            source_filter: self.task_session_source_filter,
        }
    }

    fn set_ai_session_pinned(&mut self, session_id: String, pinned: bool, cx: &mut Context<Self>) {
        match self
            .runtime_service
            .set_ai_session_pinned(&session_id, pinned)
        {
            Ok(_) => {
                self.status_message = if pinned {
                    "session pinned".to_string()
                } else {
                    "session unpinned".to_string()
                };
                self.invalidate_task_column(cx);
            }
            Err(error) => self.status_message = error,
        }
    }

    fn set_ai_session_archived(
        &mut self,
        session_id: String,
        archived: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime_service
            .set_ai_session_archived(&session_id, archived)
        {
            Ok(_) => {
                self.status_message = if archived {
                    "session archived".to_string()
                } else {
                    "session restored".to_string()
                };
                self.invalidate_task_column(cx);
            }
            Err(error) => self.status_message = error,
        }
    }

    fn set_task_session_sort(&mut self, sort: TaskSessionSort, cx: &mut Context<Self>) {
        match self
            .runtime_service
            .set_ai_session_list_sort(sort.as_setting())
        {
            Ok(_) => {
                self.task_session_sort = sort;
                self.invalidate_task_column(cx);
            }
            Err(error) => self.status_message = error,
        }
    }
}

fn task_column_content(
    app_entity: gpui::Entity<WeCodeApp>,
    header_view: gpui::Entity<TaskColumnHeaderView>,
    worktree_list_view: gpui::Entity<TaskWorktreeListView>,
    branch_list_view: gpui::Entity<TaskBranchListView>,
    terminal_list_view: gpui::Entity<TaskTerminalListView>,
    session_list_view: gpui::Entity<TaskSessionListView>,
    active_tab: TaskColumnPrimaryTab,
    active_git_tab: TaskGitTab,
    worktree_count: usize,
    branch_count: usize,
    terminal_count: usize,
    session_count: usize,
    labels: TaskColumnLabels,
    cx: &mut Context<TaskColumnView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .h_full()
        .min_h_0()
        .child(gpui::AnyView::from(header_view))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .bg(theme::vibrancy_panel(color(theme::BG_COLUMN)))
                .flex()
                .flex_col()
                .child(task_primary_tabs(
                    app_entity.clone(),
                    active_tab,
                    terminal_count,
                    session_count,
                    labels.clone(),
                    cx,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .when(active_tab == TaskColumnPrimaryTab::Git, |this| {
                            this.child(task_git_area(
                                app_entity.clone(),
                                active_git_tab,
                                worktree_count,
                                branch_count,
                                labels.clone(),
                                worktree_list_view,
                                branch_list_view,
                                cx,
                            ))
                        })
                        .when(active_tab == TaskColumnPrimaryTab::Terminals, |this| {
                            this.child(gpui::AnyView::from(terminal_list_view))
                        })
                        .when(active_tab == TaskColumnPrimaryTab::Sessions, |this| {
                            this.child(gpui::AnyView::from(session_list_view))
                        }),
                ),
        )
}

fn task_git_area(
    app_entity: gpui::Entity<WeCodeApp>,
    active_tab: TaskGitTab,
    worktree_count: usize,
    branch_count: usize,
    labels: TaskColumnLabels,
    worktree_list_view: gpui::Entity<TaskWorktreeListView>,
    branch_list_view: gpui::Entity<TaskBranchListView>,
    cx: &mut Context<TaskColumnView>,
) -> impl IntoElement {
    let worktree_entity = app_entity.clone();
    let branch_entity = app_entity;
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .child(
            div()
                .h(px(34.0))
                .flex_none()
                .px_3()
                .flex()
                .items_center()
                .child(
                    div()
                        .h(px(30.0))
                        .w_full()
                        .flex()
                        .items_center()
                        .rounded(px(6.0))
                        .bg(cx.theme().tab_bar_segmented)
                        .p(px(2.0))
                        .child(task_git_tab_button(
                            "task-git-tab-worktrees",
                            labels.worktrees,
                            worktree_count,
                            active_tab == TaskGitTab::Worktrees,
                            cx,
                            move |cx| {
                                cx.update_entity(&worktree_entity, |app, cx| {
                                    app.task_git_tab = TaskGitTab::Worktrees;
                                    let _ = app.task_column_view(cx);
                                });
                            },
                        ))
                        .child(task_git_tab_button(
                            "task-git-tab-branches",
                            labels.branches,
                            branch_count,
                            active_tab == TaskGitTab::Branches,
                            cx,
                            move |cx| {
                                cx.update_entity(&branch_entity, |app, cx| {
                                    app.task_git_tab = TaskGitTab::Branches;
                                    app.refresh_git_panel_state_async_quiet(cx);
                                    let _ = app.task_column_view(cx);
                                });
                            },
                        )),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .when(active_tab == TaskGitTab::Worktrees, |this| {
                    this.child(gpui::AnyView::from(worktree_list_view))
                })
                .when(active_tab == TaskGitTab::Branches, |this| {
                    this.child(gpui::AnyView::from(branch_list_view))
                }),
        )
}

fn task_git_tab_button(
    id: &'static str,
    label: String,
    count: usize,
    active: bool,
    cx: &mut Context<TaskColumnView>,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(26.0))
        .flex_1()
        .flex_basis(px(0.0))
        .min_w_0()
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .rounded(px(5.0))
        .text_size(rems(0.6875))
        .font_weight(if active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        })
        .map(|this| {
            if active {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            } else {
                this.text_color(cx.theme().tab_foreground)
                    .hover(|style| style.bg(cx.theme().secondary_hover))
            }
        })
        .cursor_pointer()
        .on_click(move |_, _window, cx| on_click(cx))
        .child(div().min_w_0().truncate().child(label))
        .child(div().flex_none().child(count.to_string()))
}

fn task_primary_tabs(
    app_entity: gpui::Entity<WeCodeApp>,
    active_tab: TaskColumnPrimaryTab,
    terminal_count: usize,
    session_count: usize,
    labels: TaskColumnLabels,
    cx: &mut Context<TaskColumnView>,
) -> impl IntoElement {
    let git_entity = app_entity.clone();
    let terminal_entity = app_entity.clone();
    let session_entity = app_entity;
    div().h(px(42.0)).flex_none().px_3().py(px(5.0)).child(
        div()
            .size_full()
            .flex()
            .items_center()
            .rounded(px(6.0))
            .bg(cx.theme().tab_bar_segmented)
            .p(px(3.0))
            .child(task_primary_tab_button(
                "task-primary-tab-git",
                labels.git,
                None,
                active_tab == TaskColumnPrimaryTab::Git,
                cx,
                move |cx| {
                    cx.update_entity(&git_entity, |app, cx| {
                        app.task_column_primary_tab = TaskColumnPrimaryTab::Git;
                        let _ = app.task_column_view(cx);
                    });
                },
            ))
            .child(task_primary_tab_button(
                "task-primary-tab-terminals",
                labels.terminals,
                Some(terminal_count),
                active_tab == TaskColumnPrimaryTab::Terminals,
                cx,
                move |cx| {
                    cx.update_entity(&terminal_entity, |app, cx| {
                        app.task_column_primary_tab = TaskColumnPrimaryTab::Terminals;
                        let _ = app.task_column_view(cx);
                    });
                },
            ))
            .child(task_primary_tab_button(
                "task-primary-tab-sessions",
                labels.sessions,
                Some(session_count),
                active_tab == TaskColumnPrimaryTab::Sessions,
                cx,
                move |cx| {
                    cx.update_entity(&session_entity, |app, cx| {
                        app.task_column_primary_tab = TaskColumnPrimaryTab::Sessions;
                        let _ = app.task_column_view(cx);
                    });
                },
            )),
    )
}

fn task_primary_tab_button(
    id: &'static str,
    label: String,
    count: Option<usize>,
    active: bool,
    cx: &mut Context<TaskColumnView>,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h_full()
        .flex_1()
        .flex_basis(px(0.0))
        .min_w_0()
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .rounded(px(5.0))
        .text_size(rems(0.75))
        .font_weight(if active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        })
        .map(|this| {
            if active {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            } else {
                this.text_color(cx.theme().tab_foreground)
                    .hover(|style| style.bg(cx.theme().secondary_hover))
            }
        })
        .cursor_pointer()
        .on_click(move |_, _window, cx| on_click(cx))
        .child(div().min_w_0().truncate().child(label))
        .when_some(count, |this, count| {
            this.child(div().flex_none().child(count.to_string()))
        })
}

fn task_column_header(
    project_name: String,
    refreshing: bool,
    create_label: String,
    create_branch: bool,
    refresh_label: String,
    language: String,
    local_branches: Vec<(String, bool)>,
    remote_branches: Vec<String>,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskColumnHeaderView>,
) -> impl IntoElement {
    let create_entity = app_entity.clone();
    let refresh_entity = app_entity.clone();
    div()
        .h(px(52.0))
        .w_full()
        .px(px(10.0))
        .flex_shrink_0()
        .flex()
        // No `items_center` on the outer div: the content row below stretches to
        // full header height so its drag area covers the whole title bar.
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(theme::vibrancy(cx.theme().title_bar))
        .child(
            // No `items_center`: children stretch to full header height so the
            // drag area fills it; the title text and buttons center themselves.
            div()
                .flex()
                .justify_between()
                .w_full()
                .h_full()
                .child(titlebar_drag_area(
                    "task-column-titlebar-drag",
                    div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_center()
                        .text_sm()
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(project_name),
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            wecode_tooltip_container(
                                app_entity.clone(),
                                "task-create-tooltip",
                                create_label.clone(),
                            )
                            .child(
                                Button::new("task-create")
                                    .ghost()
                                    .compact()
                                    .text_color(cx.theme().secondary_foreground)
                                    .icon(
                                        Icon::new(HeroIconName::Plus)
                                            .size_3p5()
                                            .text_color(cx.theme().secondary_foreground),
                                    )
                                    .on_click(move |_, window, cx| {
                                        if create_branch {
                                            super::git_actions::show_create_git_branch_dialog(
                                                create_entity.clone(),
                                                &language,
                                                local_branches.clone(),
                                                remote_branches.clone(),
                                                window,
                                                cx,
                                            );
                                        } else {
                                            cx.update_entity(
                                                &create_entity,
                                                |app: &mut WeCodeApp, cx| {
                                                    app.open_worktree_creator_window(window, cx);
                                                },
                                            );
                                        }
                                    }),
                            ),
                        )
                        .child(
                            wecode_tooltip_container(
                                app_entity,
                                "task-refresh-tooltip",
                                refresh_label,
                            )
                            .child(
                                Button::new("task-refresh")
                                    .ghost()
                                    .compact()
                                    .loading(refreshing)
                                    .disabled(refreshing)
                                    .text_color(cx.theme().secondary_foreground)
                                    .icon(
                                        Icon::new(HeroIconName::ArrowPath)
                                            .size_3p5()
                                            .text_color(cx.theme().secondary_foreground),
                                    )
                                    .on_click(move |_, _window, cx| {
                                        cx.update_entity(
                                            &refresh_entity,
                                            |app: &mut WeCodeApp, cx| {
                                                app.refresh_task_column_async(cx);
                                                app.refresh_git_panel_state_async_quiet(cx);
                                            },
                                        );
                                    }),
                            ),
                        ),
                ),
        )
}

fn task_list_area(
    rows: Vec<TaskWorktreeRow>,
    labels: TaskColumnLabels,
    scroll_handle: UniformListScrollHandle,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskWorktreeListView>,
) -> impl IntoElement {
    if rows.is_empty() {
        return div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .p_4()
            .child(task_empty_state(
                labels.no_worktrees_title,
                HeroIconName::Square3Stack3d,
                cx,
            ))
            .into_any_element();
    }
    let rows = Rc::new(rows);
    let row_labels = labels.clone();
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .p_3()
                .overflow_hidden()
                .child(wecode_uniform_list(
                    "task-column-worktrees",
                    rows,
                    scroll_handle,
                    None,
                    cx,
                    move |row, _index, _window, cx| {
                        div()
                            .w_full()
                            .pb(px(4.0))
                            .child(worktree_compact_row(
                                row,
                                row_labels.clone(),
                                app_entity.clone(),
                                cx,
                            ))
                            .into_any_element()
                    },
                )),
        )
        .into_any_element()
}

fn branch_list_area(
    rows: Vec<TaskBranchRow>,
    labels: TaskColumnLabels,
    scroll_handle: UniformListScrollHandle,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskBranchListView>,
) -> impl IntoElement {
    if rows.is_empty() {
        return div()
            .size_full()
            .min_h_0()
            .p_4()
            .child(task_empty_state(
                labels.no_branches_title,
                HeroIconName::ArrowPathRoundedSquare,
                cx,
            ))
            .into_any_element();
    }
    let rows = Rc::new(rows);
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .p_3()
        .overflow_hidden()
        .child(wecode_uniform_list(
            "task-column-branches",
            rows,
            scroll_handle,
            None,
            cx,
            move |row, _index, _window, cx| {
                div()
                    .w_full()
                    .pb(px(4.0))
                    .child(branch_compact_row(
                        row,
                        labels.clone(),
                        app_entity.clone(),
                        cx,
                    ))
                    .into_any_element()
            },
        ))
        .into_any_element()
}

fn branch_compact_row(
    branch: TaskBranchRow,
    labels: TaskColumnLabels,
    app_entity: gpui::Entity<WeCodeApp>,
    _cx: &mut Context<TaskBranchListView>,
) -> impl IntoElement {
    let branch_name = branch.name.clone();
    let is_current = branch.is_current;
    div()
        .id(SharedString::from(format!("task-branch-{branch_name}")))
        .w_full()
        .min_w_0()
        .rounded(px(8.0))
        .px_3()
        .py(px(8.0))
        .flex()
        .items_center()
        .gap_2()
        .when(is_current, |this| {
            this.bg(theme::elevate(color(theme::BG_COLUMN), 0.11))
        })
        .hover(|style| style.bg(theme::elevate(color(theme::BG_COLUMN), 0.07)))
        .cursor_pointer()
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.select_git_branch(branch_name.clone(), window, cx);
                if !is_current {
                    app.checkout_selected_git_branch(window, cx);
                }
                app.invalidate_task_column(cx);
            });
        })
        .child(
            div()
                .size(px(7.0))
                .flex_none()
                .rounded_full()
                .bg(if is_current {
                    color(theme::GREEN)
                } else {
                    color(theme::TEXT_DIM).opacity(0.45)
                }),
        )
        .child(
            Icon::new(HeroIconName::ArrowPathRoundedSquare)
                .size_3p5()
                .flex_none()
                .text_color(if is_current {
                    color(theme::GREEN)
                } else {
                    color(theme::TEXT_DIM)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_sm()
                .font_weight(if is_current {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(color(theme::TEXT))
                .child(branch.name),
        )
        .when(is_current, |this| {
            this.child(
                div()
                    .flex_none()
                    .text_size(rems(0.6875))
                    .text_color(color(theme::GREEN))
                    .child(labels.current),
            )
        })
}

fn task_empty_state(
    title: String,
    icon: HeroIconName,
    cx: &mut Context<impl Render>,
) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .max_w(px(220.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .text_center()
                .child(
                    div()
                        .size(px(34.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(cx.theme().secondary)
                        .child(
                            Icon::new(icon)
                                .size_4()
                                .text_color(cx.theme().muted_foreground),
                        ),
                )
                .child(
                    div()
                        .text_size(rems(0.875))
                        .line_height(rems(1.125))
                        .text_color(cx.theme().foreground)
                        .child(title),
                ),
        )
        .into_any_element()
}

fn recent_session_area(
    sessions: Vec<TaskSessionRow>,
    labels: TaskColumnLabels,
    filter: TaskSessionFilter,
    sort: TaskSessionSort,
    source_filter: TaskSessionSourceFilter,
    scroll_handle: UniformListScrollHandle,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskSessionListView>,
) -> impl IntoElement {
    let session_count = sessions.len();
    let sessions = Rc::new(sessions);
    let row_labels = labels.clone();
    let row_app_entity = app_entity.clone();

    div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .child(session_filter_tabs(
            app_entity,
            filter,
            sort,
            source_filter,
            labels.clone(),
            cx,
        ))
        .child(
            div()
                .relative()
                .flex_1()
                .w_full()
                .min_w_0()
                .min_h_0()
                .p_2()
                .overflow_hidden()
                .child(if session_count == 0 {
                    task_empty_state(labels.no_sessions_title, HeroIconName::CommandLine, cx)
                } else {
                    wecode_uniform_list_with_sizing(
                        "task-column-recent-sessions",
                        sessions,
                        scroll_handle,
                        None,
                        ListSizingBehavior::Auto,
                        cx,
                        move |session, _index, _window, cx| {
                            div()
                                .w_full()
                                .min_w_0()
                                .pb(px(4.0))
                                .child(ai_session_compact_row(
                                    session,
                                    row_labels.clone(),
                                    row_app_entity.clone(),
                                    cx,
                                ))
                                .into_any_element()
                        },
                    )
                    .into_any_element()
                }),
        )
        .into_any_element()
}

fn session_filter_tabs(
    app_entity: gpui::Entity<WeCodeApp>,
    filter: TaskSessionFilter,
    sort: TaskSessionSort,
    source_filter: TaskSessionSourceFilter,
    labels: TaskColumnLabels,
    cx: &mut Context<TaskSessionListView>,
) -> impl IntoElement {
    let items = [
        (TaskSessionFilter::All, labels.all.clone()),
        (TaskSessionFilter::Archived, labels.archived.clone()),
    ];
    div()
        .h(px(34.0))
        .flex_none()
        .px_2()
        .flex()
        .items_center()
        .child(
            div()
                .h(px(30.0))
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .rounded(px(6.0))
                .bg(cx.theme().tab_bar_segmented)
                .p(px(2.0))
                .children(items.into_iter().map(|(item_filter, label)| {
                    let app_entity = app_entity.clone();
                    let active = filter == item_filter;
                    div()
                        .id(SharedString::from(format!(
                            "task-session-filter-{item_filter:?}"
                        )))
                        .h(px(26.0))
                        .flex_1()
                        .flex_basis(px(0.0))
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px(px(2.0))
                        .rounded(px(5.0))
                        .text_size(rems(0.6875))
                        .font_weight(if active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .map(|this| {
                            if active {
                                this.bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground)
                            } else {
                                this.text_color(cx.theme().tab_foreground)
                                    .hover(|style| style.bg(cx.theme().secondary_hover))
                            }
                        })
                        .cursor_pointer()
                        .on_click(move |_, _window, cx| {
                            cx.update_entity(&app_entity, |app, cx| {
                                app.task_session_filter = item_filter;
                                let _ = app.task_column_view(cx);
                            });
                        })
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .text_center()
                                .truncate()
                                .child(label),
                        )
                }))
                .child(session_sort_button(
                    app_entity.clone(),
                    sort,
                    labels.clone(),
                ))
                .child(session_source_filter_button(
                    app_entity,
                    source_filter,
                    labels,
                    cx,
                )),
        )
}

fn session_sort_button(
    app_entity: gpui::Entity<WeCodeApp>,
    sort: TaskSessionSort,
    labels: TaskColumnLabels,
) -> impl IntoElement {
    let selected_label = match sort {
        TaskSessionSort::UpdatedAt => labels.sort_updated.clone(),
        TaskSessionSort::CreatedAt => labels.sort_created.clone(),
    };
    let options = [
        (TaskSessionSort::UpdatedAt, labels.sort_updated),
        (TaskSessionSort::CreatedAt, labels.sort_created),
    ];
    Button::new("task-session-sort")
        .ghost()
        .compact()
        .with_size(Size::Small)
        .size(px(26.0))
        .tooltip(format!("{}: {selected_label}", labels.sort))
        .icon(
            Icon::new(HeroIconName::BarsArrowDown)
                .size_3()
                .text_color(color(theme::TEXT_DIM)),
        )
        .dropdown_menu_with_anchor(gpui::Anchor::TopRight, move |menu, _window, _cx| {
            options
                .iter()
                .fold(menu.min_w(160.), |menu, (item_sort, label)| {
                    let app_entity = app_entity.clone();
                    let item_sort = *item_sort;
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(item_sort == sort)
                            .on_click(move |_, _window, cx| {
                                cx.update_entity(&app_entity, |app, cx| {
                                    app.set_task_session_sort(item_sort, cx);
                                });
                            }),
                    )
                })
        })
}

fn session_source_filter_button(
    app_entity: gpui::Entity<WeCodeApp>,
    source_filter: TaskSessionSourceFilter,
    labels: TaskColumnLabels,
    _cx: &mut Context<TaskSessionListView>,
) -> impl IntoElement {
    let source_label = match source_filter {
        TaskSessionSourceFilter::All => labels.all.clone(),
        TaskSessionSourceFilter::Claude => "Claude".to_string(),
        TaskSessionSourceFilter::Codex => "Codex".to_string(),
    };
    let icon_color = match source_filter {
        TaskSessionSourceFilter::All => color(theme::TEXT_DIM),
        TaskSessionSourceFilter::Claude => color(theme::ORANGE),
        TaskSessionSourceFilter::Codex => color(theme::ACCENT),
    };
    let tooltip = format!("{}: {source_label}", labels.filter);
    let options = [
        (TaskSessionSourceFilter::All, labels.all),
        (TaskSessionSourceFilter::Claude, "Claude".to_string()),
        (TaskSessionSourceFilter::Codex, "Codex".to_string()),
    ];

    Button::new("task-session-source-filter")
        .ghost()
        .compact()
        .with_size(Size::Small)
        .size(px(26.0))
        .selected(source_filter != TaskSessionSourceFilter::All)
        .tooltip(tooltip)
        .icon(
            Icon::new(HeroIconName::Funnel)
                .size_3()
                .text_color(icon_color),
        )
        .dropdown_menu_with_anchor(gpui::Anchor::TopRight, move |menu, _window, _cx| {
            options
                .iter()
                .fold(menu.min_w(140.), |menu, (item_filter, label)| {
                    let app_entity = app_entity.clone();
                    let item_filter = *item_filter;
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(item_filter == source_filter)
                            .on_click(move |_, _window, cx| {
                                cx.update_entity(&app_entity, |app, cx| {
                                    app.task_session_source_filter = item_filter;
                                    let _ = app.task_column_view(cx);
                                });
                            }),
                    )
                })
        })
}

const TERMINAL_LIST_ROW_HEIGHT: f32 = 58.0;

fn terminal_list_area(
    terminals: Vec<TaskTerminalRow>,
    labels: TaskColumnLabels,
    scroll_handle: UniformListScrollHandle,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskTerminalListView>,
) -> impl IntoElement {
    let mut items = terminals
        .into_iter()
        .map(TaskTerminalListItem::Terminal)
        .collect::<Vec<_>>();
    items.push(TaskTerminalListItem::Create);
    let list_height = TERMINAL_LIST_ROW_HEIGHT * items.len() as f32 + 16.0;
    let items = Rc::new(items);
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .child(
            div()
                .w_full()
                .min_w_0()
                .h(px(list_height))
                .flex_shrink()
                .min_h_0()
                .p_2()
                .overflow_hidden()
                .child(wecode_uniform_list(
                    "task-column-terminals",
                    items,
                    scroll_handle,
                    None,
                    cx,
                    move |item, _index, _window, cx| {
                        div()
                            .w_full()
                            .min_w_0()
                            .h(px(TERMINAL_LIST_ROW_HEIGHT))
                            .pb(px(4.0))
                            .child(match item {
                                TaskTerminalListItem::Terminal(terminal) => terminal_compact_row(
                                    terminal,
                                    labels.clone(),
                                    app_entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                                TaskTerminalListItem::Create => terminal_create_card(
                                    labels.new_terminal.clone(),
                                    app_entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                            })
                            .into_any_element()
                    },
                )),
        )
        .into_any_element()
}

fn terminal_create_card(
    label: String,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskTerminalListView>,
) -> impl IntoElement {
    let gateway_status = GatewayService::global_status();
    let gateway_ready = gateway_status.addr.is_some() && gateway_status.error.is_none();
    let gateway_hint = if let Some(error) = gateway_status.error {
        format!("Gateway failed: {error}")
    } else if gateway_status.enabled {
        "Gateway starting".to_string()
    } else {
        "Gateway disabled".to_string()
    };
    Button::new("task-terminal-create")
        .custom(
            ButtonCustomVariant::new(cx)
                .color(theme::elevate(color(theme::BG_COLUMN), 0.035))
                .hover(theme::elevate(color(theme::BG_COLUMN), 0.07))
                .foreground(cx.theme().foreground),
        )
        .w_full()
        .min_w_0()
        .h(px(36.0))
        .rounded(px(8.0))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(
            div()
                .text_center()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(theme::TEXT))
                .child(format!("＋ {label}")),
        )
        .dropdown_menu_with_anchor(gpui::Anchor::TopLeft, move |menu, _window, _cx| {
            let shell_entity = app_entity.clone();
            let mut menu = menu
                .min_w(260.)
                .item(
                    PopupMenuItem::new("纯终端")
                        .icon(HeroIconName::CommandLine)
                        .on_click(move |_, window, cx| {
                            cx.update_entity(&shell_entity, |app, cx| {
                                app.split_terminal(window, cx);
                            });
                        }),
                )
                .separator()
                .item(new_terminal_agent_menu_item(
                    app_entity.clone(),
                    "Claude Code",
                    HeroIconName::CommandLine,
                    "claude",
                ))
                .item(new_terminal_agent_menu_item(
                    app_entity.clone(),
                    "Codex",
                    HeroIconName::CommandLine,
                    "codex",
                ))
                .item(new_terminal_agent_menu_item(
                    app_entity.clone(),
                    "Kiro",
                    HeroIconName::Sparkles,
                    "kiro",
                ))
                .separator();
            if gateway_ready {
                for (label, target) in [
                    ("Kiro Gateway · Claude · Opus 4.8", "kiro-gateway-claude"),
                    ("Gateway · Haiku 4.5", "kiro-gateway-claude-haiku-4-5"),
                    ("Gateway · Sonnet 4.6", "kiro-gateway-claude-sonnet-4-6"),
                    ("Gateway · Opus 4.6", "kiro-gateway-claude-opus-4-6"),
                    ("Gateway · Opus 4.7", "kiro-gateway-claude-opus-4-7"),
                    ("Gateway · Opus 4.8", "kiro-gateway-claude-opus-4-8"),
                    ("Gateway · DeepSeek 3.2", "kiro-gateway-claude-deepseek-3-2"),
                    ("Gateway · GLM 5", "kiro-gateway-claude-glm-5"),
                    ("Gateway · MiniMax M2.5", "kiro-gateway-claude-minimax-m2-5"),
                    (
                        "Gateway · Qwen3 Coder",
                        "kiro-gateway-claude-qwen3-coder-next",
                    ),
                ] {
                    menu = menu.item(new_terminal_agent_menu_item(
                        app_entity.clone(),
                        label,
                        HeroIconName::ServerStack,
                        target,
                    ));
                }
            } else {
                menu = menu.item(
                    PopupMenuItem::new(gateway_hint.clone())
                        .icon(HeroIconName::ServerStack)
                        .disabled(true),
                );
            }
            menu
        })
}

fn new_terminal_agent_menu_item(
    app_entity: gpui::Entity<WeCodeApp>,
    label: &'static str,
    icon: HeroIconName,
    target: &'static str,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(icon)
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.create_terminal_with_quick_agent(target, window, cx);
            });
        })
}

fn terminal_compact_row(
    terminal: TaskTerminalRow,
    labels: TaskColumnLabels,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskTerminalListView>,
) -> impl IntoElement {
    let collapsed = terminal.collapsed;
    let collapsed_index = terminal.collapsed_index;
    let pane_index = terminal.pane_index;
    let terminal_id = terminal.terminal_id.clone();
    let terminal_id_for_click = terminal_id.clone();
    let terminal_id_for_wechat = terminal_id.clone();
    let terminal_id_for_rename = terminal_id.clone();
    let terminal_id_for_close = terminal_id.clone();
    let lifecycle = terminal.lifecycle;
    let running = terminal.running;
    let title_for_rename = terminal.title.clone();
    let title_for_menu = terminal.title.clone();
    let rename_label = labels.rename.clone();
    let menu_labels = labels.clone();
    let app_entity_for_row = app_entity.clone();
    let row_id = if collapsed {
        SharedString::from(format!(
            "compact-terminal-collapsed-{}",
            collapsed_index.unwrap_or(0)
        ))
    } else {
        SharedString::from(format!("compact-terminal-{pane_index}"))
    };
    let icon_color = if collapsed {
        color(theme::TEXT_DIM)
    } else {
        cx.theme().muted_foreground
    };
    let terminal_icon_color = match terminal.lifecycle {
        Some(state) if state != AgentLifecycleState::Idle => agent_lifecycle_color(state),
        _ => icon_color,
    };
    let title_color = if collapsed {
        color(theme::TEXT_DIM)
    } else {
        color(theme::TEXT)
    };
    let terminal_subtitle = terminal.subtitle.clone();
    let terminal_created_at = terminal.created_at.and_then(terminal_created_at_text);
    let terminal_wechat_bound = terminal.wechat_bound;
    let terminal_lifecycle = terminal.lifecycle;
    div()
        .id(row_id)
        .w_full()
        .min_w_0()
        .h_full()
        .rounded(px(8.0))
        .px_2()
        .py(px(5.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .when(terminal.active, |this| {
            this.bg(theme::elevate(color(theme::BG_COLUMN), 0.07))
        })
        .cursor_pointer()
        .hover(|style| style.bg(theme::elevate(color(theme::BG_COLUMN), 0.07)))
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity_for_row, |app, cx| {
                if lifecycle == Some(AgentLifecycleState::Completed)
                    && terminal_id_for_click
                        .as_deref()
                        .is_some_and(|id| app.dismiss_pane_agent_lifecycle_completion(id))
                {
                    app.invalidate_task_column(cx);
                }
                if app.workspace_view != WorkspaceView::Terminal {
                    app.set_workspace_view(WorkspaceView::Terminal, window, cx);
                }
                if let Some(idx) = collapsed_index {
                    app.restore_collapsed_terminal(idx, window, cx);
                } else {
                    app.select_terminal_pane(pane_index, window, cx);
                }
            });
        })
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    Icon::new(HeroIconName::CommandLine)
                        .size_3p5()
                        .flex_none()
                        .text_color(terminal_icon_color),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(title_color)
                        .truncate()
                        .child(terminal.title.clone()),
                )
                .when_some(
                    terminal_lifecycle.filter(|state| *state != AgentLifecycleState::Idle),
                    |this, state| this.child(agent_lifecycle_status_dot(state)),
                )
                .when(
                    collapsed
                        && terminal_lifecycle
                            .is_none_or(|state| state == AgentLifecycleState::Idle),
                    |this| {
                        this.child(
                            div()
                                .flex_none()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(color(theme::GREEN).opacity(0.85)),
                        )
                    },
                ),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .when_some(terminal_subtitle, |this, subtitle| {
                    this.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_right()
                            .text_size(rems(0.6875))
                            .line_height(rems(1.0))
                            .text_color(color(theme::TEXT_DIM))
                            .truncate()
                            .child(subtitle),
                    )
                })
                .when_some(terminal_created_at, |this, created_at| {
                    this.child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(3.0))
                            .text_size(rems(0.6875))
                            .text_color(color(theme::TEXT_DIM))
                            .child(Icon::new(HeroIconName::Clock).size(px(10.0)))
                            .child(created_at),
                    )
                })
                .when_some(terminal_id_for_wechat, |this, terminal_id| {
                    let app_entity_for_tooltip = app_entity.clone();
                    let app_entity_for_click = app_entity.clone();
                    let tooltip = if terminal_wechat_bound {
                        labels.wechat_bound.clone()
                    } else {
                        labels.bind_wechat.clone()
                    };
                    let icon_color = if terminal_wechat_bound {
                        color(theme::GREEN)
                    } else {
                        color(theme::TEXT_DIM)
                    };
                    this.child(
                        wecode_tooltip_container(
                            app_entity_for_tooltip,
                            format!("task-terminal-wechat-{terminal_id}"),
                            tooltip,
                        )
                        .size(px(18.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .text_color(icon_color)
                        .hover(|style| style.bg(theme::elevate(color(theme::BG_COLUMN), 0.11)))
                        .on_click(move |_event, window, cx| {
                            cx.stop_propagation();
                            window.prevent_default();
                            cx.update_entity(&app_entity_for_click, |app, cx| {
                                app.wechat_bind_terminal_session(&terminal_id, cx);
                            });
                        })
                        .child(
                            Icon::new(HeroIconName::ChatBubbleLeftRight)
                                .size(px(10.0))
                                .text_color(icon_color),
                        ),
                    )
                }),
        )
        .context_menu(move |menu, _window, _cx| {
            let rename_entity = app_entity.clone();
            let rename_terminal_id = terminal_id_for_rename.clone();
            let rename_title = title_for_rename.clone();
            let rename_action_label = rename_label.clone();
            let close_entity = app_entity.clone();
            let close_title = title_for_menu.clone();
            let close_labels = menu_labels.clone();
            let close_terminal_id = terminal_id_for_close.clone();
            menu.item(
                PopupMenuItem::new(rename_label.clone())
                    .icon(HeroIconName::PencilSquare)
                    .on_click(move |_, window, cx| {
                        let app_entity = rename_entity.clone();
                        let terminal_id = rename_terminal_id.clone();
                        super::quick_input::show_quick_input(
                            rename_action_label.clone(),
                            rename_action_label.clone(),
                            rename_title.clone(),
                            false,
                            move |title, _window, cx| {
                                app_entity.update(cx, |app, cx| {
                                    app.rename_terminal_pane(
                                        terminal_id.clone(),
                                        pane_index,
                                        collapsed_index,
                                        title,
                                        cx,
                                    );
                                });
                            },
                            window,
                            cx,
                        );
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(close_labels.close.clone())
                    .icon(HeroIconName::XMark)
                    .on_click(move |_, window, cx| {
                        request_close_task_terminal(
                            close_entity.clone(),
                            close_terminal_id.clone(),
                            pane_index,
                            collapsed_index,
                            close_title.clone(),
                            running,
                            close_labels.clone(),
                            window,
                            cx,
                        );
                    }),
            )
        })
}

fn terminal_created_at_text(created_at: f64) -> Option<String> {
    if !created_at.is_finite() || created_at <= 0.0 {
        return None;
    }
    let seconds = created_at.floor() as i64;
    let nanos = ((created_at - seconds as f64) * 1_000_000_000.0) as u32;
    DateTime::from_timestamp(seconds, nanos)
        .map(|time| time.with_timezone(&Local).format("%m-%d %H:%M").to_string())
}

#[allow(clippy::too_many_arguments)]
fn request_close_task_terminal(
    app_entity: gpui::Entity<WeCodeApp>,
    terminal_id: Option<String>,
    pane_index: usize,
    collapsed_index: Option<usize>,
    title: String,
    running: bool,
    labels: TaskColumnLabels,
    window: &mut Window,
    cx: &mut App,
) {
    if !running {
        close_task_terminal_now(
            &app_entity,
            terminal_id,
            pane_index,
            collapsed_index,
            window,
            cx,
        );
        return;
    }

    let message = labels.close_terminal_message_format.replace("%@", &title);
    window.open_dialog(cx, move |dialog, _window, _cx| {
        let close_entity = app_entity.clone();
        let close_terminal_id = terminal_id.clone();
        dialog
            .title(labels.close_terminal_title.clone())
            .button_props(
                DialogButtonProps::default()
                    .ok_text(labels.close.clone())
                    .ok_variant(ButtonVariant::Danger)
                    .cancel_text(labels.cancel.clone())
                    .show_cancel(true),
            )
            .on_ok(move |_, window, cx| {
                close_task_terminal_now(
                    &close_entity,
                    close_terminal_id.clone(),
                    pane_index,
                    collapsed_index,
                    window,
                    cx,
                );
                true
            })
            .child(
                div()
                    .px_4()
                    .py_3()
                    .text_sm()
                    .text_color(color(theme::TEXT))
                    .child(message.clone()),
            )
    });
}

fn close_task_terminal_now(
    app_entity: &gpui::Entity<WeCodeApp>,
    terminal_id: Option<String>,
    pane_index: usize,
    collapsed_index: Option<usize>,
    window: &mut Window,
    cx: &mut App,
) {
    cx.update_entity(app_entity, |app, cx| {
        if let Some(collapsed_index) = collapsed_index {
            app.close_collapsed_terminal_pane(collapsed_index, cx);
        } else {
            app.close_terminal_target(terminal_id.as_deref(), pane_index, window, cx);
        }
    });
}

fn worktree_compact_row(
    worktree: TaskWorktreeRow,
    labels: TaskColumnLabels,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskWorktreeListView>,
) -> impl IntoElement {
    let worktree_id = worktree.id.clone();
    let menu_worktree_id = worktree.id.clone();
    let menu_worktree_path = worktree.path.clone();
    let is_default = worktree.is_default;
    let lifecycle_dismiss_id = if worktree.is_default {
        worktree.project_id.clone()
    } else {
        worktree.id.clone()
    };
    let lifecycle = worktree.lifecycle;
    let select_entity = app_entity.clone();
    div()
        .id(SharedString::from(format!(
            "compact-worktree-{}",
            worktree.id
        )))
        .w_full()
        .min_w_0()
        .rounded(px(8.0))
        .px_3()
        .py(px(8.0))
        .flex()
        .items_center()
        .gap_3()
        .when(worktree.active, |this| {
            this.bg(theme::elevate(color(theme::BG_COLUMN), 0.07))
        })
        .cursor_pointer()
        .hover(|style| style.bg(theme::elevate(color(theme::BG_COLUMN), 0.07)))
        .on_click(move |_, window, cx| {
            cx.update_entity(&select_entity, |app, cx| {
                if lifecycle == Some(AgentLifecycleState::Completed)
                    && app.dismiss_worktree_pane_agent_lifecycle_completion(&lifecycle_dismiss_id)
                {
                    app.invalidate_task_column(cx);
                }
                app.select_worktree(worktree_id.clone(), window, cx)
            });
        })
        .child(worktree_activity_dot(lifecycle))
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(rems(0.875))
                        .line_height(rems(1.125))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(worktree.title),
                )
                .child(
                    div()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(color(theme::TEXT_DIM))
                        .truncate()
                        .child(
                            labels
                                .changed_format
                                .replace("%@", &worktree.git_changes.to_string()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .child(
                    div()
                        .text_color(cx.theme().success)
                        .child(format!("+{}", worktree.git_additions.max(0))),
                )
                .child(
                    div()
                        .text_color(cx.theme().danger)
                        .child(format!("-{}", worktree.git_deletions.max(0))),
                ),
        )
        .context_menu(move |menu, _window, _cx| {
            let open_entity = app_entity.clone();
            let open_path = menu_worktree_path.clone();
            let merge_entity = app_entity.clone();
            let merge_worktree_id = menu_worktree_id.clone();
            let remove_entity = app_entity.clone();
            let remove_worktree_id = menu_worktree_id.clone();

            let menu = menu.item(
                PopupMenuItem::new(labels.open_folder.clone())
                    .icon(HeroIconName::Folder)
                    .on_click(move |_, _window, cx| {
                        cx.update_entity(&open_entity, |app, cx| {
                            app.open_worktree_folder(open_path.clone(), cx);
                        });
                    }),
            );

            if is_default {
                return menu;
            }

            menu.separator()
                .item(
                    PopupMenuItem::new(labels.merge.clone())
                        .icon(HeroIconName::ArrowDownTray)
                        .on_click(move |_, _window, cx| {
                            cx.update_entity(&merge_entity, |app, cx| {
                                app.merge_worktree_by_id(merge_worktree_id.clone(), cx);
                            });
                        }),
                )
                .separator()
                .item(
                    PopupMenuItem::new(labels.delete.clone())
                        .icon(HeroIconName::Trash)
                        .on_click(move |_, _window, cx| {
                            cx.update_entity(&remove_entity, |app, cx| {
                                app.request_remove_worktree_by_id(
                                    remove_worktree_id.clone(),
                                    false,
                                    cx,
                                );
                            });
                        }),
                )
        })
}

fn worktree_activity_dot(lifecycle: Option<AgentLifecycleState>) -> AnyElement {
    // Fixed-width slot centring the indicator, so the row text never shifts as
    // it swaps between a 10px static dot and the 12px ring spinner.
    let dot = div().size(px(10.0)).rounded_full();
    let inner = match lifecycle {
        Some(AgentLifecycleState::Working) => spin_icon(color(theme::ORANGE), 12.0),
        Some(AgentLifecycleState::Waiting) | Some(AgentLifecycleState::Warning) => {
            dot.bg(color(theme::ORANGE)).into_any_element()
        }
        Some(AgentLifecycleState::Completed) => dot.bg(color(theme::GREEN)).into_any_element(),
        Some(AgentLifecycleState::Error) => dot.bg(color(theme::RED)).into_any_element(),
        Some(AgentLifecycleState::Idle) | None => dot.bg(color(theme::ACCENT)).into_any_element(),
    };
    div()
        .flex_none()
        .size(px(12.0))
        .flex()
        .items_center()
        .justify_center()
        .child(inner)
        .into_any_element()
}

fn worktree_row_title(worktree: &WorktreeInfo, no_branch: &str) -> String {
    let branch = worktree.branch.trim();
    let name = worktree.name.trim();

    if branch.is_empty() || branch == "uninitialized" {
        return no_branch.to_string();
    }

    if worktree.is_default {
        return branch.to_string();
    }

    if !name.is_empty() {
        return name.to_string();
    }

    branch
        .split('/')
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or(branch)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worktree(name: &str, branch: &str, is_default: bool) -> WorktreeInfo {
        WorktreeInfo {
            id: "worktree-1".to_string(),
            project_id: "project-1".to_string(),
            name: name.to_string(),
            branch: branch.to_string(),
            path: "/workspace/project".to_string(),
            status: "active".to_string(),
            is_default,
            exists: true,
            git_summary: Default::default(),
        }
    }

    #[test]
    fn worktree_row_title_uses_worktree_fields_without_git_panel_state() {
        assert_eq!(
            worktree_row_title(&worktree("Task A", "feature/task-a", false), "No Branch"),
            "Task A"
        );
        assert_eq!(
            worktree_row_title(&worktree("", "feature/task-b", false), "No Branch"),
            "task-b"
        );
        assert_eq!(
            worktree_row_title(&worktree("Main", "main", true), "No Branch"),
            "main"
        );
        assert_eq!(
            worktree_row_title(&worktree("Draft", "uninitialized", false), "No Branch"),
            "No Branch"
        );
    }

    #[test]
    fn ai_session_source_display_names_are_distinct() {
        assert_eq!(ai_session_source_display_name("claude"), "Claude");
        assert_eq!(ai_session_source_display_name("claude-code"), "Claude");
        assert_eq!(ai_session_source_display_name("codex"), "Codex");
        assert_eq!(ai_session_source_display_name("kiro"), "kiro");
    }

    #[test]
    fn branch_rows_put_current_branch_first_then_sort_by_name() {
        let rows = task_branch_rows(&[
            GitBranchSummary {
                name: "feature/zeta".to_string(),
                is_current: false,
            },
            GitBranchSummary {
                name: "main".to_string(),
                is_current: true,
            },
            GitBranchSummary {
                name: "feature/alpha".to_string(),
                is_current: false,
            },
        ]);
        assert_eq!(
            rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
            vec!["main", "feature/alpha", "feature/zeta"]
        );
    }

    #[test]
    fn terminal_created_at_text_is_compact_and_rejects_invalid_values() {
        assert!(terminal_created_at_text(0.0).is_none());
        assert!(terminal_created_at_text(f64::NAN).is_none());
        let text = terminal_created_at_text(1_700_000_000.0).expect("formatted creation time");
        assert_eq!(text.len(), 11);
        assert_eq!(text.chars().nth(2), Some('-'));
        assert_eq!(text.chars().nth(5), Some(' '));
        assert_eq!(text.chars().nth(8), Some(':'));
    }
}

fn ai_session_source_display_name(source: &str) -> String {
    let normalized = source.trim().to_ascii_lowercase();
    if normalized.contains("claude") {
        "Claude".to_string()
    } else if normalized.contains("codex") {
        "Codex".to_string()
    } else {
        source.trim().to_string()
    }
}

fn ai_session_compact_row(
    session: TaskSessionRow,
    labels: TaskColumnLabels,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<TaskSessionListView>,
) -> impl IntoElement {
    let restore_session_id = session.id.clone();
    let menu_session_id = session.id.clone();
    let menu_session_title = session.title.clone();
    let session_pinned = session.pinned;
    let session_archived = session.archived;
    let source_label = ai_session_source_display_name(&session.source);
    let source_color = if source_label == "Claude" {
        color(theme::ORANGE)
    } else if source_label == "Codex" {
        color(theme::ACCENT)
    } else {
        color(theme::TEXT_MUTED)
    };
    let session_detail = session.last_model.clone().unwrap_or_default();
    let has_session_detail = !session_detail.is_empty();
    let last_seen = relative_time_label_for_language(session.last_seen_at, &labels.language);
    let restore_entity = app_entity.clone();
    let drag_payload = TaskSessionDrag {
        session_id: session.id.clone(),
        title: session.title.clone(),
    };
    let session_active = session.active;
    div()
        .id(SharedString::from(format!(
            "compact-session-{}",
            session.id
        )))
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(if session_active {
            color(theme::ACCENT)
        } else {
            cx.theme().transparent
        })
        .px_3()
        .py(px(8.0))
        .cursor_pointer()
        .when(session_active, |style| {
            style.bg(theme::elevate(color(theme::BG_COLUMN), 0.11))
        })
        .hover(move |style| {
            style.bg(theme::elevate(
                color(theme::BG_COLUMN),
                if session_active { 0.14 } else { 0.07 },
            ))
        })
        .on_drag(drag_payload, move |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
        })
        .on_double_click(move |_, window, cx| {
            cx.update_entity(&restore_entity, |app, cx| {
                app.selected_ai_session_id = Some(restore_session_id.clone());
                app.restore_selected_ai_session(window, cx);
            });
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(session.title.clone()),
                )
                .when(session.pinned, |this| {
                    this.child(
                        Icon::new(HeroIconName::Star)
                            .size_3()
                            .flex_none()
                            .text_color(color(theme::ACCENT)),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .min_w_0()
                .text_size(rems(0.75))
                .text_color(color(theme::TEXT_DIM))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(div().size(px(5.0)).rounded_full().bg(source_color))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(source_color)
                                .child(source_label),
                        ),
                )
                .when(has_session_detail, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .text_color(color(theme::TEXT_DIM))
                            .child("·"),
                    )
                    .child(div().min_w_0().flex_1().truncate().child(session_detail))
                })
                .when(!has_session_detail, |this| {
                    this.child(div().min_w_0().flex_1())
                })
                .child(div().flex_shrink_0().text_right().child(format!(
                    "{} · {}",
                    session_usage_label(&session),
                    last_seen
                ))),
        )
        .context_menu(move |menu, _window, _cx| {
            let open_entity = app_entity.clone();
            let open_session_id = menu_session_id.clone();
            let fork_entity = app_entity.clone();
            let fork_session_id = menu_session_id.clone();
            let fork_label = labels.new_session.clone();
            let rename_entity = app_entity.clone();
            let rename_session_id = menu_session_id.clone();
            let rename_session_title = menu_session_title.clone();
            let remove_entity = app_entity.clone();
            let remove_session_id = menu_session_id.clone();
            let pin_entity = app_entity.clone();
            let pin_session_id = menu_session_id.clone();
            let pin_label = if session_pinned {
                labels.unpin.clone()
            } else {
                labels.pin.clone()
            };
            let archive_entity = app_entity.clone();
            let archive_session_id = menu_session_id.clone();
            let archive_label = if session_archived {
                labels.unarchive.clone()
            } else {
                labels.archive.clone()
            };

            menu.item(
                PopupMenuItem::new(labels.open.clone())
                    .icon(HeroIconName::CommandLine)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&open_entity, |app, cx| {
                            app.selected_ai_session_id = Some(open_session_id.clone());
                            app.restore_selected_ai_session(window, cx);
                        });
                    }),
            )
            .submenu_with_icon(
                Some(Icon::new(HeroIconName::Plus)),
                fork_label,
                _window,
                _cx,
                move |menu, _window, _cx| {
                    AI_SESSION_FORK_TARGETS
                        .iter()
                        .copied()
                        .fold(menu, |menu, target| {
                            let target_entity = fork_entity.clone();
                            let target_session_id = fork_session_id.clone();
                            menu.item(
                                PopupMenuItem::new(target.display_name().to_string())
                                    .icon(HeroIconName::CommandLine)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&target_entity, |app, cx| {
                                            app.fork_ai_session_to_tool(
                                                target_session_id.clone(),
                                                target,
                                                window,
                                                cx,
                                            );
                                        });
                                    }),
                            )
                        })
                },
            )
            .item(
                PopupMenuItem::new(labels.rename.clone())
                    .icon(HeroIconName::PencilSquare)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&rename_entity, |app, cx| {
                            app.request_rename_ai_session(
                                rename_session_id.clone(),
                                rename_session_title.clone(),
                                window,
                                cx,
                            );
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(pin_label)
                    .icon(HeroIconName::Star)
                    .on_click(move |_, _window, cx| {
                        cx.update_entity(&pin_entity, |app, cx| {
                            app.set_ai_session_pinned(pin_session_id.clone(), !session_pinned, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(archive_label)
                    .icon(if session_archived {
                        HeroIconName::ArchiveBoxArrowDown
                    } else {
                        HeroIconName::ArchiveBox
                    })
                    .on_click(move |_, _window, cx| {
                        cx.update_entity(&archive_entity, |app, cx| {
                            app.set_ai_session_archived(
                                archive_session_id.clone(),
                                !session_archived,
                                cx,
                            );
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(labels.delete.clone())
                    .icon(HeroIconName::Trash)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&remove_entity, |app, cx| {
                            app.request_remove_ai_session(remove_session_id.clone(), window, cx);
                        });
                    }),
            )
        })
}

fn session_usage_label(session: &TaskSessionRow) -> String {
    if session.total_tokens > 0 {
        return compact_number(session.total_tokens);
    }
    usage_amount_label(&session.usage_amounts).unwrap_or_else(|| compact_number(0))
}

fn task_session_matches_filter(session: &TaskSessionRow, filter: TaskSessionFilter) -> bool {
    match filter {
        TaskSessionFilter::All => !session.archived,
        TaskSessionFilter::Archived => session.archived,
    }
}

fn task_session_matches_source(session: &TaskSessionRow, filter: TaskSessionSourceFilter) -> bool {
    let source = session.source.to_ascii_lowercase();
    match filter {
        TaskSessionSourceFilter::All => true,
        TaskSessionSourceFilter::Claude => source.contains("claude"),
        TaskSessionSourceFilter::Codex => source.contains("codex"),
    }
}

#[cfg(test)]
mod session_filter_tests {
    use super::*;

    fn row(source: &str, pinned: bool, archived: bool) -> TaskSessionRow {
        TaskSessionRow {
            id: "session-1".to_string(),
            session_key: "session-1".to_string(),
            external_session_id: None,
            title: "Session".to_string(),
            source: source.to_string(),
            last_model: Some("claude-opus-4.8".to_string()),
            first_seen_at: 0.5,
            last_seen_at: 1.0,
            total_tokens: 0,
            usage_amounts: Vec::new(),
            active: false,
            pinned,
            archived,
        }
    }

    #[test]
    fn session_filters_combine_pin_and_source() {
        let claude = row("claude", true, false);
        let codex = row("codex", false, true);
        assert!(task_session_matches_filter(&claude, TaskSessionFilter::All));
        assert!(!task_session_matches_filter(&codex, TaskSessionFilter::All));
        assert!(task_session_matches_filter(
            &codex,
            TaskSessionFilter::Archived
        ));
        assert!(task_session_matches_source(
            &claude,
            TaskSessionSourceFilter::Claude
        ));
        assert!(!task_session_matches_source(
            &claude,
            TaskSessionSourceFilter::Codex
        ));
        assert!(task_session_matches_source(
            &codex,
            TaskSessionSourceFilter::All
        ));
    }
}
