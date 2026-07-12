use super::*;
use crate::app::workspace_shared::workspace_i18n;

#[derive(Clone, Copy)]
enum AttentionSectionKind {
    Actionable,
    Completed,
    Active,
}

impl WeCodeApp {
    pub(in crate::app) fn attention_workspace_body(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let language = self.state.settings.language.clone();
        let items = self.attention_feed.recent_items(ATTENTION_FEED_PAGE_LIMIT);
        let unread_count = self.attention_feed.unread_count();
        let actionable = items
            .iter()
            .filter(|item| {
                item.semantic == AttentionSemantic::Actionable
                    && item.read_state != AttentionReadState::Resolved
            })
            .cloned()
            .collect::<Vec<_>>();
        let completed = items
            .iter()
            .filter(|item| {
                item.semantic == AttentionSemantic::Completed
                    || item.read_state == AttentionReadState::Resolved
            })
            .cloned()
            .collect::<Vec<_>>();
        let active = items
            .iter()
            .filter(|item| item.semantic == AttentionSemantic::Active)
            .cloned()
            .collect::<Vec<_>>();
        let is_empty = actionable.is_empty() && completed.is_empty() && active.is_empty();
        let app_entity = cx.entity();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(theme::vibrancy_panel(cx.theme().sidebar))
            .child(
                div()
                    .id("attention-workspace-scroll")
                    .size_full()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(20.0))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap_4()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_size(rems(1.125))
                                                    .line_height(rems(1.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(color(theme::TEXT))
                                                    .child(workspace_i18n(
                                                        &language,
                                                        "workspace.attention.page_title",
                                                        "Agent attention",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(rems(0.75))
                                                    .line_height(rems(1.1))
                                                    .text_color(color(theme::TEXT_DIM))
                                                    .child(workspace_i18n(
                                                        &language,
                                                        "workspace.attention.page_subtitle",
                                                        "Review tasks that need you and recent agent activity.",
                                                    )),
                                            ),
                                    )
                                    .when(unread_count > 0, |this| {
                                        let app_entity = app_entity.clone();
                                        this.child(
                                            Button::new("attention-mark-all-read")
                                                .secondary()
                                                .compact()
                                                .with_size(Size::Small)
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .child(
                                                            Icon::new(HeroIconName::CheckCircle)
                                                                .size_3p5(),
                                                        )
                                                        .child(workspace_i18n(
                                                            &language,
                                                            "workspace.attention.mark_all_read",
                                                            "Mark all as read",
                                                        )),
                                                )
                                                .on_click(move |_, _window, cx| {
                                                    cx.update_entity(&app_entity, |app, cx| {
                                                        app.mark_all_attention_read(cx);
                                                    });
                                                }),
                                        )
                                    }),
                            )
                            .child(attention_summary_row(
                                actionable.len(),
                                completed.len(),
                                active.len(),
                                &language,
                                cx,
                            ))
                            .when(is_empty, |this| {
                                this.child(attention_empty_state(&language, cx))
                            })
                            .when(!actionable.is_empty(), |this| {
                                this.child(attention_section(
                                    AttentionSectionKind::Actionable,
                                    actionable,
                                    &language,
                                    app_entity.clone(),
                                    cx,
                                ))
                            })
                            .when(!completed.is_empty(), |this| {
                                this.child(attention_section(
                                    AttentionSectionKind::Completed,
                                    completed,
                                    &language,
                                    app_entity.clone(),
                                    cx,
                                ))
                            })
                            .when(!active.is_empty(), |this| {
                                this.child(attention_section(
                                    AttentionSectionKind::Active,
                                    active,
                                    &language,
                                    app_entity,
                                    cx,
                                ))
                            }),
                    ),
            )
    }
}

const ATTENTION_FEED_PAGE_LIMIT: usize = 500;

fn attention_summary_row(
    actionable: usize,
    completed: usize,
    active: usize,
    language: &str,
    cx: &mut Context<WeCodeApp>,
) -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .gap(px(10.0))
        .child(attention_summary_card(
            workspace_i18n(
                language,
                "workspace.attention.section.actionable",
                "Needs attention",
            ),
            actionable,
            HeroIconName::ExclamationTriangle,
            AttentionSectionKind::Actionable,
            cx,
        ))
        .child(attention_summary_card(
            workspace_i18n(
                language,
                "workspace.attention.section.completed",
                "Completed",
            ),
            completed,
            HeroIconName::CheckCircle,
            AttentionSectionKind::Completed,
            cx,
        ))
        .child(attention_summary_card(
            workspace_i18n(
                language,
                "workspace.attention.section.active",
                "In progress",
            ),
            active,
            HeroIconName::Bolt,
            AttentionSectionKind::Active,
            cx,
        ))
}

fn attention_summary_card(
    label: String,
    count: usize,
    icon: HeroIconName,
    kind: AttentionSectionKind,
    cx: &mut Context<WeCodeApp>,
) -> impl IntoElement {
    let accent = attention_accent(kind, cx);
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_3()
        .rounded(px(10.0))
        .bg(theme::vibrancy_raised(cx.theme().sidebar))
        .px(px(14.0))
        .py(px(12.0))
        .child(
            div()
                .size(px(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.0))
                .bg(accent.opacity(0.14))
                .text_color(accent)
                .child(Icon::new(icon).size_3p5()),
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT))
                        .child(count.to_string()),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(rems(0.6875))
                        .text_color(color(theme::TEXT_DIM))
                        .child(label),
                ),
        )
}

fn attention_section(
    kind: AttentionSectionKind,
    items: Vec<AttentionItem>,
    language: &str,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> impl IntoElement {
    let (title, icon) = match kind {
        AttentionSectionKind::Actionable => (
            workspace_i18n(
                language,
                "workspace.attention.section.actionable",
                "Needs attention",
            ),
            HeroIconName::ExclamationTriangle,
        ),
        AttentionSectionKind::Completed => (
            workspace_i18n(
                language,
                "workspace.attention.section.completed",
                "Completed",
            ),
            HeroIconName::CheckCircle,
        ),
        AttentionSectionKind::Active => (
            workspace_i18n(
                language,
                "workspace.attention.section.active",
                "In progress",
            ),
            HeroIconName::Bolt,
        ),
    };
    let accent = attention_accent(kind, cx);
    let count = items.len();
    let language = language.to_string();

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(accent)
                .child(Icon::new(icon).size_3p5())
                .child(
                    div()
                        .text_size(rems(0.8125))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .min_w(px(18.0))
                        .h(px(18.0))
                        .px(px(5.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(accent.opacity(0.12))
                        .text_size(px(10.0))
                        .child(count.to_string()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .children(items.into_iter().map(|item| {
                    attention_item_card(item, &language, app_entity.clone(), cx).into_any_element()
                })),
        )
}

fn attention_item_card(
    item: AttentionItem,
    language: &str,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> impl IntoElement {
    let item_id = item.id.clone();
    let project_id = item.project_id.clone();
    let project_path = item.project_path.clone();
    let terminal_id = item.terminal_id.clone();
    let unread = item.read_state == AttentionReadState::Unread;
    let resolved = item.read_state == AttentionReadState::Resolved;
    let kind = if item.semantic == AttentionSemantic::Actionable && !resolved {
        AttentionSectionKind::Actionable
    } else if item.semantic == AttentionSemantic::Active {
        AttentionSectionKind::Active
    } else {
        AttentionSectionKind::Completed
    };
    let accent = attention_accent(kind, cx);
    let status = attention_item_status(&item, language);
    let title = if item.session_title.trim().is_empty() {
        status.clone()
    } else {
        item.session_title.clone()
    };
    let project = if item.project_name.trim().is_empty() {
        workspace_i18n(
            language,
            "workspace.attention.unknown_project",
            "Unknown project",
        )
    } else {
        item.project_name.clone()
    };
    let tool = if item.tool.trim().is_empty() {
        "AI CLI".to_string()
    } else {
        item.tool.clone()
    };
    let relative_time = attention_relative_time(item.updated_at, app_now_seconds(), language);

    div()
        .id(SharedString::from(format!("attention-item-{}", item.id)))
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .rounded(px(10.0))
        .border_1()
        .border_color(if unread {
            accent.opacity(0.5)
        } else {
            cx.theme().border.opacity(0.7)
        })
        .bg(theme::vibrancy_raised(cx.theme().sidebar))
        .px(px(14.0))
        .py(px(12.0))
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(cx.theme().secondary_hover)
                .border_color(accent.opacity(0.45))
        })
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.open_attention_item(
                    item_id.clone(),
                    project_id.clone(),
                    project_path.clone(),
                    terminal_id.clone(),
                    window,
                    cx,
                );
            });
        })
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(32.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.0))
                        .bg(accent.opacity(0.13))
                        .text_color(accent)
                        .child(
                            Icon::new(match kind {
                                AttentionSectionKind::Actionable => {
                                    HeroIconName::ExclamationTriangle
                                }
                                AttentionSectionKind::Completed => HeroIconName::CheckCircle,
                                AttentionSectionKind::Active => HeroIconName::Bolt,
                            })
                            .size_3p5(),
                        ),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(rems(0.8125))
                                        .font_weight(if unread {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(color(theme::TEXT))
                                        .child(title),
                                )
                                .when(unread, |this| {
                                    this.child(
                                        div().size(px(7.0)).flex_none().rounded_full().bg(accent),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(rems(0.6875))
                                .line_height(rems(1.0))
                                .text_color(color(theme::TEXT_DIM))
                                .child(format!("{project} · {tool}")),
                        ),
                ),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .rounded_full()
                        .bg(accent.opacity(0.12))
                        .px(px(8.0))
                        .py(px(3.0))
                        .text_size(px(10.0))
                        .text_color(accent)
                        .child(status),
                )
                .child(
                    div()
                        .text_size(rems(0.6875))
                        .text_color(color(theme::TEXT_DIM))
                        .child(relative_time),
                )
                .child(
                    Icon::new(HeroIconName::ChevronRight)
                        .size_3p5()
                        .text_color(color(theme::TEXT_DIM)),
                ),
        )
}

fn attention_item_status(item: &AttentionItem, language: &str) -> String {
    if item.read_state == AttentionReadState::Resolved {
        return workspace_i18n(language, "workspace.attention.resolved", "Resolved");
    }
    match item.semantic {
        AttentionSemantic::Actionable if item.interrupted => workspace_i18n(
            language,
            "workspace.attention.interrupted",
            "Task interrupted",
        ),
        AttentionSemantic::Actionable => workspace_i18n(
            language,
            "workspace.attention.needs_input",
            "Waiting for input",
        ),
        AttentionSemantic::Completed => {
            workspace_i18n(language, "workspace.attention.completed", "Task completed")
        }
        AttentionSemantic::Active => {
            workspace_i18n(language, "workspace.attention.active", "Agent working")
        }
    }
}

fn attention_empty_state(language: &str, cx: &mut Context<WeCodeApp>) -> impl IntoElement {
    div()
        .min_h(px(260.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .rounded(px(12.0))
        .bg(theme::vibrancy_raised(cx.theme().sidebar))
        .text_center()
        .child(
            div()
                .size(px(44.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(cx.theme().secondary)
                .text_color(color(theme::TEXT_DIM))
                .child(Icon::new(HeroIconName::Bell).size_5()),
        )
        .child(
            div()
                .text_size(rems(0.875))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child(workspace_i18n(
                    language,
                    "workspace.attention.empty",
                    "No agent updates",
                )),
        )
        .child(
            div()
                .max_w(px(360.0))
                .text_size(rems(0.75))
                .line_height(rems(1.1))
                .text_color(color(theme::TEXT_DIM))
                .child(workspace_i18n(
                    language,
                    "workspace.attention.empty_hint",
                    "Agent tasks that need input, finish, or keep running will appear here.",
                )),
        )
}

fn attention_accent(kind: AttentionSectionKind, cx: &mut Context<WeCodeApp>) -> gpui::Hsla {
    match kind {
        AttentionSectionKind::Actionable => cx.theme().danger,
        AttentionSectionKind::Completed => cx.theme().success,
        AttentionSectionKind::Active => cx.theme().primary,
    }
}
