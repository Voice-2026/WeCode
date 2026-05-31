use super::*;

pub(in crate::app) fn child_window_shell<T>(
    title: impl Into<SharedString>,
    cx: &mut Context<T>,
) -> gpui::Div {
    let title = title.into();

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
        .child(
            div()
                .h(px(48.0))
                .flex_shrink_0()
                .pl(px(86.0))
                .pr(px(20.0))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().title_bar)
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(14.0))
                        .line_height(px(14.0))
                        .child(title),
                ),
        )
}
