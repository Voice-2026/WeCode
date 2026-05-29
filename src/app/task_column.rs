use gpui_component::{InteractiveElementExt as _, menu::ContextMenuExt as _};

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
        let delete_confirm_session = self
            .ai_session_delete_confirm_id
            .as_deref()
            .and_then(|id| {
                self.state
                    .ai_history
                    .sessions
                    .iter()
                    .find(|session| session.id == id)
            })
            .cloned();

        div()
            .relative()
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
            .when_some(delete_confirm_session, |this, session| {
                this.child(ai_session_delete_confirm_overlay(session, cx))
            })
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
    let right_click_session_id = session.id.clone();
    let menu_session_id = session.id.clone();
    let app_entity = cx.entity();
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
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |app, _event, window, cx| {
                app.select_ai_session(right_click_session_id.clone(), window, cx)
            }),
        )
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
        .context_menu(move |menu, _window, _cx| {
            let open_entity = app_entity.clone();
            let open_session_id = menu_session_id.clone();
            let remove_entity = app_entity.clone();
            let remove_session_id = menu_session_id.clone();

            menu.item(
                PopupMenuItem::new("打开")
                    .icon(IconName::SquareTerminal)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&open_entity, |app, cx| {
                            app.selected_ai_session_id = Some(open_session_id.clone());
                            app.restore_selected_ai_session(window, cx);
                        });
                    }),
            )
            .item(PopupMenuItem::new("删除").icon(IconName::Delete).on_click(
                move |_, window, cx| {
                    cx.update_entity(&remove_entity, |app, cx| {
                        app.request_remove_ai_session(remove_session_id.clone(), window, cx);
                    });
                },
            ))
        })
}

fn ai_session_delete_confirm_overlay(
    session: AISessionSummary,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .left(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(color(0x000000).opacity(0.34))
        .p(px(16.0))
        .child(
            div()
                .w(px(250.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(color(theme::BORDER_SOFT))
                .bg(color(theme::BG_PANEL))
                .p(px(14.0))
                .shadow_lg()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(IconName::Delete)
                                .size_4()
                                .text_color(color(theme::ORANGE)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child("删除会话记录"),
                        ),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(format!("从索引中删除 {}？", session.title)),
                )
                .child(
                    div()
                        .mt(px(14.0))
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("ai-session-delete-cancel")
                                .compact()
                                .ghost()
                                .text_color(cx.theme().secondary_foreground)
                                .label("取消")
                                .on_click(cx.listener(|app, _event, _window, cx| {
                                    app.cancel_remove_ai_session(cx)
                                })),
                        )
                        .child(
                            Button::new("ai-session-delete-confirm")
                                .compact()
                                .primary()
                                .text_color(cx.theme().primary_foreground)
                                .label("删除")
                                .on_click(cx.listener(|app, _event, window, cx| {
                                    app.confirm_remove_ai_session(window, cx)
                                })),
                        ),
                ),
        )
}
