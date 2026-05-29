use gpui_component::InteractiveElementExt as _;

use super::*;

impl CoduxApp {
    pub(super) fn task_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project_name = self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "未选择项目".to_string());

        div()
            .flex()
            .flex_col()
            .w(px(286.0))
            .h_full()
            .bg(color(theme::BG_PANEL))
            .border_r_1()
            .border_color(color(theme::BORDER_SOFT))
            .child(column_header(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color(theme::TEXT))
                            .truncate()
                            .child(project_name),
                    )
                    .child(header_icon_button(
                        "task-refresh",
                        IconName::Redo2,
                        cx,
                        |app, _event, window, cx| {
                            app.reload_worktrees(window, cx);
                            app.reload_ai_history(window, cx);
                            app.reload_project_git(window, cx);
                        },
                    )),
            ))
            .child(
                v_resizable("task-column-resizable")
                    .child(
                        resizable_panel()
                            .size(px(320.0))
                            .size_range(px(180.0)..px(560.0))
                            .child(self.task_list_area(cx)),
                    )
                    .child(
                        resizable_panel()
                            .size_range(px(180.0)..px(640.0))
                            .child(self.recent_session_area(cx)),
                    ),
            )
    }

    fn task_list_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = &self.state.worktrees.tasks;
        let selected_worktree_id = self.state.worktrees.selected_worktree_id.as_deref();
        div().flex().flex_col().size_full().min_h_0().child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .gap_1()
                .p_3()
                .overflow_y_scrollbar()
                .children(tasks.iter().take(12).map(|task| {
                    let active = selected_worktree_id
                        .map(|id| id == task.worktree_id)
                        .unwrap_or(false);
                    let git_summary = self
                        .state
                        .worktrees
                        .worktrees
                        .iter()
                        .find(|worktree| worktree.id == task.worktree_id)
                        .map(|worktree| worktree.git_summary.clone())
                        .unwrap_or_else(|| {
                            worktree_git_summary_from_git(&self.state.worktrees.active_git)
                        });
                    task_row(task.clone(), active, git_summary, cx).into_any_element()
                }))
                .children(
                    self.state
                        .worktrees
                        .worktrees
                        .iter()
                        .take(4)
                        .cloned()
                        .map(|worktree| {
                            let active = self
                                .state
                                .worktrees
                                .selected_worktree_id
                                .as_ref()
                                .map(|id| id == &worktree.id)
                                .unwrap_or(false);
                            worktree_compact_row(worktree, active, cx).into_any_element()
                        }),
                ),
        )
    }

    fn recent_session_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(session_section_heading(
                "会话记录",
                self.state.ai_history.sessions.len(),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .p_2()
                    .overflow_y_scrollbar()
                    .children(self.state.ai_history.sessions.iter().take(16).cloned().map(
                        |session| {
                            let active = self
                                .selected_ai_session_id
                                .as_deref()
                                .map(|id| id == session.id)
                                .unwrap_or(false);
                            ai_session_compact_row(session, active, cx).into_any_element()
                        },
                    )),
            )
    }
}

fn session_section_heading(
    title: &'static str,
    count: usize,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .h(px(40.0))
        .px_3()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .border_t_1()
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(cx.theme().secondary)
        .child(
            div()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child(title),
        )
        .child(Tag::secondary().rounded_full().child(count.to_string()))
}

fn task_row(
    task: WorktreeTaskInfo,
    active: bool,
    git: ProjectWorktreeGitSummary,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let worktree_id = task.worktree_id.clone();
    let branch_label = if task.base_branch.trim().is_empty() {
        task.title
    } else {
        task.base_branch
    };
    div()
        .id(SharedString::from(format!("task-{}", task.worktree_id)))
        .rounded(px(8.0))
        .px_4()
        .py_1()
        .flex()
        .items_center()
        .gap_4()
        .bg(color(if active {
            theme::BG_ROW_HOVER
        } else {
            theme::BG_COLUMN
        }))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_worktree(worktree_id.clone(), window, cx)
        }))
        .child(
            div()
                .w(px(10.0))
                .h(px(10.0))
                .rounded_full()
                .flex_shrink_0()
                .bg(color(theme::ACCENT)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .overflow_hidden()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(branch_label),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(format!("{} 个变更", git.changes)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(
                                    div()
                                        .text_color(color(0x3EE66B))
                                        .child(format!("+{}", git.additions.max(0))),
                                )
                                .child(
                                    div()
                                        .text_color(color(0xFF5C68))
                                        .child(format!("-{}", git.deletions.max(0))),
                                ),
                        ),
                ),
        )
}

fn worktree_compact_row(
    worktree: WorktreeInfo,
    active: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let worktree_id = worktree.id.clone();
    let git = worktree.git_summary.clone();
    div()
        .id(SharedString::from(format!(
            "compact-worktree-{}",
            worktree.id
        )))
        .rounded(px(8.0))
        .px_4()
        .py_1()
        .flex()
        .items_center()
        .gap_4()
        .bg(color(if active {
            theme::BG_ROW_HOVER
        } else {
            theme::BG_COLUMN
        }))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_worktree(worktree_id.clone(), window, cx)
        }))
        .child(
            div()
                .w(px(10.0))
                .h(px(10.0))
                .rounded_full()
                .flex_shrink_0()
                .bg(color(theme::ACCENT)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .overflow_hidden()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(worktree.branch),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(format!("{} 个变更", git.changes)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(
                                    div()
                                        .text_color(color(0x3EE66B))
                                        .child(format!("+{}", git.additions.max(0))),
                                )
                                .child(
                                    div()
                                        .text_color(color(0xFF5C68))
                                        .child(format!("-{}", git.deletions.max(0))),
                                ),
                        ),
                ),
        )
}

fn worktree_git_summary_from_git(git: &GitSummary) -> ProjectWorktreeGitSummary {
    ProjectWorktreeGitSummary {
        changes: git.staged + git.unstaged + git.untracked,
        incoming: git.behind,
        outgoing: git.ahead,
        additions: 0,
        deletions: 0,
    }
}

fn ai_session_compact_row(
    session: AISessionSummary,
    active: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let session_id = session.id.clone();
    let restore_session_id = session.id.clone();
    let last_seen = relative_time_label(session.last_seen_at);
    div()
        .id(SharedString::from(format!(
            "compact-session-{}",
            session.id
        )))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .rounded(px(8.0))
        .px_2()
        .py_2()
        .bg(color(if active {
            theme::BG_ROW_HOVER
        } else {
            theme::BG_PANEL
        }))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_ai_session(session_id.clone(), window, cx)
        }))
        .on_double_click(cx.listener(move |app, _event, window, cx| {
            app.selected_ai_session_id = Some(restore_session_id.clone());
            app.restore_selected_ai_session(window, cx);
        }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(session.title),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(color(theme::TEXT_DIM))
                        .child(last_seen),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .text_xs()
                .text_color(color(theme::TEXT_DIM))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(session.source),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .child(compact_number(session.total_tokens)),
                ),
        )
}
