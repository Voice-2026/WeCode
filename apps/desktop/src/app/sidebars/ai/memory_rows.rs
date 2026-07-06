use super::memory_labels::*;
use super::memory_window::ai_memory_card;
use super::*;

pub(super) fn ai_memory_manager_queue_content(
    manager: &MemoryManagerSnapshot,
    language: &str,
    _window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let queued = manager.extraction.queued.max(0);
    let running = manager.extraction.running.max(0);
    let failed = manager.extraction.failed.max(0);
    let has_queue = !manager.queued_extractions.is_empty();
    let empty_label = ai_sidebar_text(
        language,
        "memory.manager.queue.empty",
        "No queued memory tasks",
    );
    let queued_label = ai_sidebar_text(language, "memory.status.short_queued", "Queued");
    let running_label = ai_sidebar_text(language, "memory.status.short_remembering", "Remembering");
    let failed_label = ai_sidebar_text(language, "memory.status.short_failed", "Failed");

    div()
        .size_full()
        .flex()
        .flex_col()
        .when(!has_queue, |this| {
            this.child(ai_memory_manager_empty_row(empty_label, cx))
        })
        .when(has_queue, |this| {
            this.child(
                ai_memory_card(cx)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Spinner::new().xsmall().color(color(theme::ORANGE)))
                            .child(
                                div()
                                    .text_size(rems(0.875))
                                    .line_height(rems(1.125))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(cx.theme().foreground)
                                    .child(ai_sidebar_text(
                                        language,
                                        "memory.status.processing",
                                        "Remembering",
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(12.0))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(ai_memory_queue_count_badge(queued_label, queued, cx))
                            .child(ai_memory_queue_count_badge(running_label, running, cx))
                            .child(ai_memory_queue_count_badge(failed_label, failed, cx)),
                    )
                    .when_some(manager.extraction.last_error.clone(), |this, error| {
                        this.child(
                            div()
                                .mt(px(10.0))
                                .text_size(rems(0.75))
                                .line_height(rems(1.25))
                                .text_color(cx.theme().danger)
                                .child(error),
                        )
                    }),
            )
            .children(
                manager.queued_extractions.iter().cloned().map(|task| {
                    ai_memory_queued_extraction_row(task, language, cx).into_any_element()
                }),
            )
        })
}

pub(super) fn ai_memory_queue_count_badge(
    label: String,
    count: i64,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .rounded_full()
        .px(px(8.0))
        .py(px(3.0))
        .text_size(rems(0.75))
        .line_height(rems(1.0))
        .text_color(cx.theme().muted_foreground)
        .bg(cx.theme().muted)
        .child(format!("{count} {label}"))
}

pub(super) fn ai_memory_queued_extraction_row(
    task: codux_runtime::memory::MemoryExtractionTask,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let clear_id = task.id.clone();
    let title = if task.session_id.trim().is_empty() {
        task.tool.clone()
    } else {
        format!("{} · {}", task.tool, task.session_id)
    };
    let subtitle = task
        .workspace_path
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| task.project_id.clone());
    let status_label = memory_extraction_status_label(&task.status, language);
    let status_color = if task.status == "running" {
        color(theme::ORANGE)
    } else {
        color(theme::ACCENT)
    };

    ai_memory_card(cx)
        .mt(px(8.0))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .truncate()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .text_color(cx.theme().foreground)
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(4.0))
                                .truncate()
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(subtitle),
                        ),
                )
                .child(
                    div()
                        .rounded_full()
                        .px(px(8.0))
                        .py(px(3.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(status_color)
                        .bg(status_color.opacity(0.14))
                        .child(status_label),
                )
                .when(task.status != "running", |this| {
                    this.child(ai_memory_row_icon_button(
                        format!("ai-memory-manager-clear-pending-{clear_id}"),
                        HeroIconName::Trash,
                        ai_sidebar_text(language, "common.delete", "Delete"),
                        cx,
                        move |app, _event, window, cx| {
                            app.clear_pending_memory_extraction(clear_id.clone(), window, cx)
                        },
                    ))
                }),
        )
        .child(
            div()
                .mt(px(7.0))
                .text_size(rems(0.6875))
                .line_height(rems(1.0))
                .text_color(cx.theme().muted_foreground)
                .child(memory_date_label(task.enqueued_at)),
        )
}

pub(super) fn ai_memory_manager_target_row(
    target: codux_runtime::memory::MemoryManagerTargetRow,
    selected_scope: &str,
    selected_project_id: Option<&str>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let scope = target.scope.clone();
    let project_id = target.project_id.clone();
    let title = if scope == "user" {
        ai_sidebar_text(language, "memory.manager.user_memory", "User Memory")
    } else {
        target.title.clone()
    };
    let subtitle = if scope == "user" {
        ai_sidebar_text(
            language,
            "memory.manager.user_memory.subtitle",
            "Cross-project preferences",
        )
    } else {
        target.subtitle.clone()
    };
    let active = scope == selected_scope
        && (scope != "project" || project_id.as_deref() == selected_project_id);
    let foreground = if active {
        cx.theme().foreground
    } else {
        cx.theme().muted_foreground
    };
    div()
        .id(SharedString::from(format!(
            "memory-manager-target-{}",
            target.id
        )))
        .mb(px(2.0))
        .min_h(px(48.0))
        .w_full()
        .rounded(px(8.0))
        .px(px(10.0))
        .py(px(7.0))
        .flex()
        .items_center()
        .gap(px(9.0))
        .cursor_pointer()
        .text_color(foreground)
        .bg(if active {
            cx.theme().sidebar_accent
        } else {
            cx.theme().transparent
        })
        .hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.select_memory_manager_target(scope.clone(), project_id.clone(), cx)
        }))
        .child(
            Icon::new(HeroIconName::Folder)
                .size_4()
                .flex_shrink_0()
                .text_color(foreground),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .truncate()
                        .text_size(rems(0.8125))
                        .line_height(rems(1.125))
                        .child(title),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(rems(0.6875))
                        .line_height(rems(1.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .flex_none()
                .rounded_full()
                .px(px(7.0))
                .py(px(2.0))
                .text_size(rems(0.6875))
                .line_height(rems(1.0))
                .text_color(if active {
                    cx.theme().foreground
                } else {
                    cx.theme().muted_foreground
                })
                .bg(if active {
                    cx.theme().primary.opacity(0.16)
                } else {
                    cx.theme().muted
                })
                .child(target.count.to_string()),
        )
}

pub(super) fn ai_memory_manager_tab_button(
    label: impl Into<String>,
    tab: MemoryManagerTab,
    active_tab: MemoryManagerTab,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let label = label.into();
    let active = tab == active_tab;
    let hover_bg = cx.theme().secondary_hover;
    div()
        .id(SharedString::from(format!(
            "ai-memory-manager-tab-{}",
            tab.as_str()
        )))
        .mr(px(6.0))
        .h(px(30.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .cursor_pointer()
        .text_size(rems(0.8125))
        .line_height(rems(1.0))
        .font_weight(if active {
            gpui::FontWeight::MEDIUM
        } else {
            gpui::FontWeight::NORMAL
        })
        .text_color(if active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        })
        .bg(if active {
            cx.theme().secondary
        } else {
            cx.theme().transparent
        })
        .hover(move |style| style.bg(hover_bg))
        .on_click(cx.listener(move |app, _event, _window, cx| app.set_memory_manager_tab(tab, cx)))
        .child(label)
}

pub(super) fn ai_memory_header_icon_button(
    id: &'static str,
    icon: HeroIconName,
    tooltip: impl Into<String>,
    loading: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    with_codux_tooltip(
        cx.entity(),
        format!("ai-memory-header-tooltip-{id}"),
        Button::new(id)
            .compact()
            .ghost()
            .loading(loading)
            .text_color(cx.theme().secondary_foreground)
            .icon(
                Icon::new(icon)
                    .size_3p5()
                    .text_color(cx.theme().secondary_foreground),
            )
            .on_click(cx.listener(on_click)),
        tooltip.into(),
    )
}

pub(super) fn ai_memory_section_label(
    label: String,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .mt(px(12.0))
        .mb(px(6.0))
        .text_size(rems(0.75))
        .line_height(rems(1.0))
        .text_color(cx.theme().muted_foreground)
        .child(label)
}

pub(super) fn ai_memory_migrate_project_button(
    manager: &MemoryManagerSnapshot,
    selected_project_id: Option<&str>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let tooltip = ai_sidebar_text(
        language,
        "memory.manager.migrate_project",
        "Rebind Project Memory",
    );
    let empty_label = ai_sidebar_text(
        language,
        "memory.manager.migrate_project.no_targets",
        "No migration targets",
    );
    let targets = manager
        .target_rows
        .iter()
        .filter(|target| {
            target.scope == "project"
                && target.is_open_project
                && target.project_id.is_some()
                && target.project_id.as_deref() != selected_project_id
        })
        .cloned()
        .collect::<Vec<_>>();
    let app_entity = cx.entity();

    with_codux_tooltip(
        cx.entity(),
        "ai-memory-migrate-project-memory-tooltip",
        Button::new("ai-memory-migrate-project-memory")
            .compact()
            .ghost()
            .text_color(cx.theme().secondary_foreground)
            .icon(
                Icon::new(HeroIconName::ArrowsRightLeft)
                    .size_3p5()
                    .text_color(cx.theme().secondary_foreground),
            )
            .dropdown_menu_with_anchor(gpui::Anchor::TopRight, move |menu, _window, _cx| {
                if targets.is_empty() {
                    return menu.item(
                        PopupMenuItem::new(empty_label.clone())
                            .icon(HeroIconName::Folder)
                            .disabled(true),
                    );
                }

                targets.iter().take(12).fold(menu, |menu, target| {
                    let Some(to_project_id) = target.project_id.clone() else {
                        return menu;
                    };
                    let title = target.title.clone();
                    let entity = app_entity.clone();
                    menu.item(
                        PopupMenuItem::new(title)
                            .icon(HeroIconName::Folder)
                            .on_click(move |_, window, cx| {
                                cx.update_entity(&entity, |app, cx| {
                                    app.migrate_selected_memory_project_to(
                                        to_project_id.clone(),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                })
            }),
        tooltip,
    )
}

pub(super) fn ai_memory_project_profile_row(
    profile: codux_runtime::memory::MemoryProjectProfileSummary,
    refreshing: bool,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let label = ai_sidebar_text(
        language,
        "memory.manager.project_profile",
        "Project Profile",
    );
    ai_memory_card(cx)
        .mt(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(
                            Icon::new(HeroIconName::DocumentText)
                                .size_4()
                                .flex_shrink_0()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(cx.theme().foreground)
                                .child(label),
                        ),
                )
                .child(if refreshing {
                    ai_memory_refreshing_label(language, cx).into_any_element()
                } else {
                    ai_memory_row_icon_button(
                        "ai-memory-refresh-project-profile",
                        HeroIconName::ArrowPath,
                        ai_sidebar_text(
                            language,
                            "memory.manager.project_profile.refresh",
                            "Regenerate Project Profile",
                        ),
                        cx,
                        |app, _event, window, cx| {
                            app.refresh_selected_memory_project_profile(window, cx)
                        },
                    )
                    .into_any_element()
                })
                .child(ai_memory_row_icon_button(
                    "ai-memory-delete-project-profile",
                    HeroIconName::Trash,
                    ai_sidebar_text(
                        language,
                        "memory.manager.project_profile.delete",
                        "Delete Project Profile",
                    ),
                    cx,
                    |app, _event, window, cx| {
                        app.delete_selected_memory_project_profile(window, cx)
                    },
                )),
        )
        .child(
            div()
                .mt(px(9.0))
                .text_size(rems(0.8125))
                .line_height(rems(1.375))
                .text_color(cx.theme().foreground)
                .w_full()
                .child(profile.content),
        )
}

pub(super) fn ai_memory_project_profile_empty_row(
    refreshing: bool,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let label = ai_sidebar_text(
        language,
        "memory.manager.project_profile",
        "Project Profile",
    );
    let empty_label = ai_sidebar_text(
        language,
        "memory.manager.project_profile.empty",
        "No project profile exists",
    );
    ai_memory_card(cx)
        .mt(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(
                            Icon::new(HeroIconName::DocumentText)
                                .size_4()
                                .flex_shrink_0()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(cx.theme().foreground)
                                .child(label),
                        ),
                )
                .child(if refreshing {
                    ai_memory_refreshing_label(language, cx).into_any_element()
                } else {
                    ai_memory_row_icon_button(
                        "ai-memory-create-project-profile",
                        HeroIconName::ArrowPath,
                        ai_sidebar_text(
                            language,
                            "memory.manager.project_profile.refresh",
                            "Regenerate Project Profile",
                        ),
                        cx,
                        |app, _event, window, cx| {
                            app.refresh_selected_memory_project_profile(window, cx)
                        },
                    )
                    .into_any_element()
                }),
        )
        .child(
            div()
                .mt(px(8.0))
                .text_size(rems(0.75))
                .line_height(rems(1.25))
                .text_color(cx.theme().muted_foreground)
                .child(empty_label),
        )
}

pub(super) fn ai_memory_refreshing_label(
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .px(px(7.0))
        .text_size(rems(0.75))
        .line_height(rems(1.0))
        .text_color(cx.theme().muted_foreground)
        .child(ai_sidebar_text(language, "common.processing", "Processing"))
}

pub(super) fn ai_memory_manager_summary_row(
    summary: &codux_runtime::memory::MemorySummaryRow,
    selected_memory_summary_id: Option<&str>,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let summary_placeholder =
        ai_sidebar_text(language, "memory.manager.edit_summary.title", "Summary");
    let version_label = ai_sidebar_text(language, "memory.manager.summary.version_format", "v%lld")
        .replacen("%lld", &summary.version.to_string(), 1);
    let tokens_label = ai_sidebar_text(
        language,
        "memory.manager.summary.tokens_format",
        "%lld tokens",
    )
    .replacen("%lld", &summary.token_estimate.to_string(), 1);
    let summary_id = summary.id.clone();
    let save_id = summary.id.clone();
    let delete_id = summary.id.clone();
    let input_value = summary.content.clone();
    let input_state = window.use_keyed_state(
        SharedString::from(format!("ai-memory-summary-content-{}", summary.id)),
        cx,
        {
            let value = input_value.clone();
            move |window, cx| {
                InputState::new(window, cx)
                    .default_value(value.clone())
                    .placeholder(summary_placeholder.clone())
            }
        },
    );
    input_state.update(cx, |state, cx| {
        if state.value().as_ref() != input_value {
            state.set_value(input_value.clone(), window, cx);
        }
    });
    let save_state = input_state.clone();
    let active = selected_memory_summary_id
        .map(|id| id == summary.id.as_str())
        .unwrap_or(false);

    ai_memory_card(cx)
        .id(SharedString::from(format!(
            "ai-memory-manager-summary-{}",
            summary.id
        )))
        .mb(px(8.0))
        .cursor_pointer()
        .when(active, |this| {
            this.border_color(cx.theme().primary.opacity(0.55))
                .bg(cx.theme().secondary_hover)
        })
        .hover(|style| style.border_color(cx.theme().primary.opacity(0.35)))
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.selected_memory_summary_id = Some(summary_id.clone());
            app.status_message = format!("selected memory summary: {summary_id}");
            app.invalidate_memory_panel(cx);
        }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(rems(0.875))
                        .line_height(rems(1.125))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(cx.theme().foreground)
                        .child(format!("{} {}", summary.scope, version_label)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .mr(px(4.0))
                                .text_size(rems(0.6875))
                                .line_height(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(tokens_label),
                        )
                        .child(ai_memory_row_icon_button(
                            format!("ai-memory-save-summary-{save_id}"),
                            HeroIconName::Check,
                            ai_sidebar_text(language, "common.save", "Save"),
                            cx,
                            move |app, _event, window, cx| {
                                let content = save_state.read(cx).value().to_string();
                                app.update_memory_summary_content(
                                    save_id.clone(),
                                    content,
                                    window,
                                    cx,
                                );
                            },
                        ))
                        .child(ai_memory_row_icon_button(
                            format!("ai-memory-delete-summary-{delete_id}"),
                            HeroIconName::Trash,
                            ai_sidebar_text(language, "common.delete", "Delete"),
                            cx,
                            move |app, _event, window, cx| {
                                app.selected_memory_summary_id = Some(delete_id.clone());
                                app.delete_selected_memory_summary(window, cx);
                            },
                        )),
                ),
        )
        .child(if active {
            div()
                .mt(px(8.0))
                .child(Input::new(&input_state).with_size(gpui_component::Size::Small))
                .into_any_element()
        } else {
            div()
                .mt(px(9.0))
                .text_size(rems(0.8125))
                .line_height(rems(1.375))
                .text_color(cx.theme().foreground)
                .w_full()
                .child(summary.content.clone())
                .into_any_element()
        })
}

pub(super) fn ai_memory_manager_entry_groups(
    entries: &[MemoryEntrySummary],
    selected_memory_entry_id: Option<&str>,
    active_tab: MemoryManagerTab,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> Vec<AnyElement> {
    let mut groups: BTreeMap<String, Vec<MemoryEntrySummary>> = BTreeMap::new();
    for entry in entries {
        groups
            .entry(memory_module_key(entry))
            .or_default()
            .push(entry.clone());
    }

    groups
        .into_iter()
        .map(|(module_key, group_entries)| {
            div()
                .mb(px(16.0))
                .child(
                    div()
                        .mb(px(8.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().size(px(7.0)).rounded_full().bg(cx.theme().primary))
                        .child(
                            div()
                                .text_size(rems(0.6875))
                                .line_height(rems(1.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(cx.theme().muted_foreground)
                                .child(memory_module_title(&module_key, language)),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .px(px(7.0))
                                .py(px(1.0))
                                .text_size(rems(0.6875))
                                .line_height(rems(1.0))
                                .text_color(cx.theme().primary)
                                .bg(cx.theme().primary.opacity(0.12))
                                .child(group_entries.len().to_string()),
                        ),
                )
                .child(div().flex().flex_col().gap(px(8.0)).children(
                    group_entries.into_iter().map(|entry| {
                        let active = selected_memory_entry_id
                            .map(|id| id == entry.id.as_str())
                            .unwrap_or(false);
                        ai_memory_manager_entry_row(entry, active, active_tab, language, cx)
                            .into_any_element()
                    }),
                ))
                .into_any_element()
        })
        .collect()
}

pub(super) fn ai_memory_manager_entry_row(
    entry: MemoryEntrySummary,
    active: bool,
    active_tab: MemoryManagerTab,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let select_id = entry.id.clone();
    let archive_id = entry.id.clone();
    let delete_id = entry.id.clone();
    let can_archive = active_tab == MemoryManagerTab::Active && entry.status == "active";

    ai_memory_card(cx)
        .id(SharedString::from(format!(
            "ai-memory-manager-entry-{}",
            entry.id
        )))
        .cursor_pointer()
        .when(active, |this| {
            this.border_color(cx.theme().primary.opacity(0.55))
                .bg(cx.theme().secondary_hover)
        })
        .hover(|style| style.border_color(cx.theme().primary.opacity(0.35)))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_memory_entry(select_id.clone(), window, cx)
        }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(ai_memory_badge(
                            memory_kind_title(&entry.kind, language),
                            memory_kind_color(&entry.kind),
                        ))
                        .child(ai_memory_status_pill(&entry.status, language, cx)),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .mr(px(4.0))
                                .text_size(rems(0.6875))
                                .line_height(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(memory_date_label(entry.updated_at)),
                        )
                        .when(can_archive, |this| {
                            this.child(ai_memory_row_icon_button(
                                format!("ai-memory-manager-archive-{archive_id}"),
                                HeroIconName::ArchiveBox,
                                ai_sidebar_text(language, "memory.manager.archive", "Archive"),
                                cx,
                                move |app, _event, window, cx| {
                                    app.selected_memory_entry_id = Some(archive_id.clone());
                                    app.archive_selected_memory_entry(window, cx);
                                },
                            ))
                        })
                        .child(ai_memory_row_icon_button(
                            format!("ai-memory-manager-delete-{delete_id}"),
                            HeroIconName::Trash,
                            ai_sidebar_text(language, "common.delete", "Delete"),
                            cx,
                            move |app, _event, window, cx| {
                                app.selected_memory_entry_id = Some(delete_id.clone());
                                app.delete_selected_memory_entry(window, cx);
                            },
                        )),
                ),
        )
        .child(
            div()
                .mt(px(9.0))
                .w_full()
                .text_size(rems(0.875))
                .line_height(rems(1.375))
                .text_color(cx.theme().foreground)
                .child(entry.content.clone()),
        )
        .child(ai_memory_entry_meta(&entry, language, cx))
        .when_some(entry.rationale.clone(), |this, rationale| {
            this.child(
                div()
                    .mt(px(6.0))
                    .w_full()
                    .text_size(rems(0.75))
                    .line_height(rems(1.25))
                    .text_color(cx.theme().muted_foreground)
                    .child(rationale),
            )
        })
        .when_some(entry.last_decision.clone(), |this, decision| {
            this.child(ai_memory_decision_row(decision, language, cx))
        })
}

pub(super) fn ai_memory_failed_extraction_row(
    task: codux_runtime::memory::MemoryExtractionTask,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let retry_id = task.id.clone();
    let clear_id = task.id.clone();
    let title = if task.session_id.trim().is_empty() {
        task.tool.clone()
    } else {
        format!("{} · {}", task.tool, task.session_id)
    };
    let subtitle = task
        .workspace_path
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| task.project_id.clone());
    let error = task
        .error
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            ai_sidebar_text(language, "memory.manager.failed.unknown", "Unknown error")
        });

    ai_memory_card(cx)
        .id(SharedString::from(format!(
            "ai-memory-manager-failed-{}",
            task.id
        )))
        .mb(px(8.0))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .truncate()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .text_color(cx.theme().foreground)
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(4.0))
                                .truncate()
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(subtitle),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(ai_memory_row_icon_button(
                            format!("ai-memory-manager-retry-{retry_id}"),
                            HeroIconName::ArrowPath,
                            ai_sidebar_text(language, "memory.manager.failed.retry", "Retry"),
                            cx,
                            move |app, _event, window, cx| {
                                app.retry_failed_memory_extraction(retry_id.clone(), window, cx)
                            },
                        ))
                        .child(ai_memory_row_icon_button(
                            format!("ai-memory-manager-clear-failed-{clear_id}"),
                            HeroIconName::Trash,
                            ai_sidebar_text(language, "common.delete", "Delete"),
                            cx,
                            move |app, _event, window, cx| {
                                app.clear_failed_memory_extraction(clear_id.clone(), window, cx)
                            },
                        )),
                ),
        )
        .child(
            div()
                .mt(px(9.0))
                .w_full()
                .text_size(rems(0.75))
                .line_height(rems(1.25))
                .text_color(cx.theme().danger)
                .child(error),
        )
        .child(
            div()
                .mt(px(6.0))
                .text_size(rems(0.6875))
                .line_height(rems(1.0))
                .text_color(cx.theme().muted_foreground)
                .child(memory_date_label(task.enqueued_at)),
        )
}

pub(super) fn ai_memory_decision_row(
    decision: codux_runtime::memory::MemoryEntryDecisionSummary,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .mt(px(8.0))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(8.0))
        .px(px(10.0))
        .py(px(7.0))
        .bg(cx.theme().secondary_hover)
        .child(ai_memory_badge(
            memory_decision_title(&decision.kind, language),
            memory_decision_color(&decision.kind),
        ))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .text_color(cx.theme().muted_foreground)
                .child(decision.reason),
        )
}

/// Secondary status indicator (dot + muted label) for an entry. Kept low-key so
/// the kind badge stays the primary identifier.
pub(super) fn ai_memory_status_pill(
    status: &str,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let status_color = memory_status_color(status);
    div()
        .flex()
        .items_center()
        .gap(px(5.0))
        .child(div().size(px(6.0)).rounded_full().bg(status_color))
        .child(
            div()
                .text_size(rems(0.6875))
                .line_height(rems(1.0))
                .text_color(cx.theme().muted_foreground)
                .child(memory_status_title(status, language)),
        )
}

/// Demoted meta line for an entry: module · tier · source rendered as plain
/// muted text instead of a row of coloured badges.
pub(super) fn ai_memory_entry_meta(
    entry: &MemoryEntrySummary,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let mut parts = vec![
        memory_module_title(&memory_module_key(entry), language),
        memory_tier_title(&entry.tier, language),
    ];
    if let Some(source_tool) = entry
        .source_tool
        .clone()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(source_tool);
    }
    div()
        .mt(px(7.0))
        .text_size(rems(0.6875))
        .line_height(rems(1.0))
        .text_color(cx.theme().muted_foreground)
        .child(parts.join(" · "))
}

pub(super) fn ai_memory_badge(label: String, badge_color: Hsla) -> impl IntoElement {
    div()
        .rounded_full()
        .px(px(9.0))
        .py(px(3.0))
        .text_size(rems(0.75))
        .line_height(rems(0.9375))
        .text_color(badge_color)
        .bg(badge_color.opacity(0.14))
        .child(label)
}

pub(super) fn ai_memory_manager_empty_row(
    message: impl Into<String>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    centered_empty_state(HeroIconName::Inbox, message, cx)
}

pub(super) fn ai_memory_row_icon_button(
    id: impl Into<SharedString>,
    icon: HeroIconName,
    tooltip: impl Into<String>,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    let tooltip = tooltip.into();
    let id = id.into();
    with_codux_tooltip(
        cx.entity(),
        SharedString::from(format!("ai-memory-row-tooltip-{id}")),
        Button::new(id)
            .compact()
            .ghost()
            .text_color(cx.theme().secondary_foreground)
            .icon(
                Icon::new(icon)
                    .size_3p5()
                    .text_color(cx.theme().secondary_foreground),
            )
            .on_click(cx.listener(on_click)),
        tooltip,
    )
}
