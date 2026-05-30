use super::*;

pub(in crate::app) fn child_window_shell(title: impl Into<SharedString>) -> gpui::Div {
    let title = title.into();

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(color(theme::BG))
        .text_color(color(theme::TEXT))
        .child(
            div()
                .h(px(48.0))
                .flex_shrink_0()
                .pl(px(86.0))
                .pr(px(20.0))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT).opacity(0.45))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(14.0))
                        .line_height(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                ),
        )
}
