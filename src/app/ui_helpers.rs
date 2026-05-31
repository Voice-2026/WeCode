use super::*;

pub(in crate::app) fn column_header(
    content: impl IntoElement,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .h(px(44.0))
        .w_full()
        .px(px(10.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().title_bar)
        .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
            window.start_window_move();
        })
        .child(content)
}

pub(in crate::app) fn header_icon_button(
    id: &'static str,
    icon: IconName,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .icon(Icon::new(icon).text_color(cx.theme().secondary_foreground))
        .on_click(cx.listener(on_click))
}

pub(in crate::app) fn assistant_header_icon_button(
    id: &'static str,
    icon: IconName,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .compact()
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .icon(
            Icon::new(icon)
                .size_3p5()
                .text_color(cx.theme().secondary_foreground),
        )
        .on_click(cx.listener(on_click))
}

pub(in crate::app) fn section(title: &'static str, rows: Vec<String>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mx_3()
        .mt_3()
        .rounded_sm()
        .border_1()
        .border_color(color(theme::BORDER))
        .bg(color(theme::BG_ELEVATED))
        .child(
            div()
                .h(px(30.0))
                .px_2()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT_MUTED))
                .child(title),
        )
        .children(rows.into_iter().map(|row| {
            div()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(color(theme::TEXT_DIM))
                .child(row)
                .into_any_element()
        }))
}

#[cfg(test)]
pub(in crate::app) fn restored_terminal_preview_lines(output_tail: &str) -> Vec<String> {
    output_tail
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| line.chars().take(96).collect::<String>())
        .collect()
}

pub(in crate::app) fn empty_label(value: &str) -> String {
    if value.trim().is_empty() {
        "none".to_string()
    } else {
        value.to_string()
    }
}
