use super::*;

pub(in crate::app::sidebars::ai) fn ai_memory_manager_summary_row(
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

pub(in crate::app::sidebars::ai) fn ai_memory_manager_entry_groups(
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

pub(in crate::app::sidebars::ai) fn ai_memory_manager_entry_row(
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
