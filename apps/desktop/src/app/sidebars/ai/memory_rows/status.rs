use super::*;

pub(in crate::app::sidebars::ai) fn ai_memory_failed_extraction_row(
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

pub(in crate::app::sidebars::ai) fn ai_memory_decision_row(
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
pub(in crate::app::sidebars::ai) fn ai_memory_status_pill(
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
pub(in crate::app::sidebars::ai) fn ai_memory_entry_meta(
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

pub(in crate::app::sidebars::ai) fn ai_memory_badge(
    label: String,
    badge_color: Hsla,
) -> impl IntoElement {
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

pub(in crate::app::sidebars::ai) fn ai_memory_manager_empty_row(
    message: impl Into<String>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    centered_empty_state(HeroIconName::Inbox, message, cx)
}

pub(in crate::app::sidebars::ai) fn ai_memory_row_icon_button(
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
