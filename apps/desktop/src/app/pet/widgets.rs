use super::*;

pub(super) fn pet_cancel_button(
    id: &'static str,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> Button {
    Button::new(id)
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .child(pet_button_label(
            pet_catalog_text(language, "common.cancel", "Cancel"),
            cx.theme().secondary_foreground,
        ))
        .on_click(|_, window, _| window.remove_window())
}

pub(super) fn pet_footer_bar(footer: impl IntoElement) -> impl IntoElement {
    div()
        .h(px(54.0))
        .flex_shrink_0()
        .border_t_1()
        .border_color(color(theme::BORDER_SOFT))
        .px(px(16.0))
        .flex()
        .items_center()
        .justify_end()
        .child(footer)
}

pub(super) fn pet_dialog_footer(children: Vec<AnyElement>) -> impl IntoElement {
    DialogFooter::new().children(children)
}

pub(super) fn pet_footer_button(
    id: &'static str,
    label: String,
    icon: HeroIconName,
    primary: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> Button {
    let button = Button::new(id)
        .text_color(if primary {
            cx.theme().primary_foreground
        } else {
            cx.theme().secondary_foreground
        })
        .icon(Icon::new(icon).size_3p5())
        .child(pet_button_label(
            label,
            if primary {
                cx.theme().primary_foreground
            } else {
                cx.theme().secondary_foreground
            },
        ))
        .on_click(cx.listener(on_click));
    if primary {
        button.primary()
    } else {
        button.secondary()
    }
}

pub(super) fn pet_button_label(
    label: impl Into<SharedString>,
    text_color: Hsla,
) -> impl IntoElement {
    div()
        .text_size(rems(0.875))
        .line_height(rems(1.125))
        .text_color(text_color)
        .child(label.into())
}

pub(super) fn pet_inline_button(
    id: &'static str,
    label: String,
    icon: HeroIconName,
    enabled: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> Button {
    Button::new(id)
        .compact()
        .primary()
        .disabled(!enabled)
        .text_color(cx.theme().primary_foreground)
        .w_full()
        .icon(Icon::new(icon).size_3p5())
        .child(
            div()
                .flex_none()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .child(SharedString::from(label)),
        )
        .on_click(cx.listener(on_click))
}

pub(super) fn pet_dex_sidebar_action(action: AnyElement) -> impl IntoElement {
    div()
        .w_full()
        .h(px(36.0))
        .flex()
        .items_center()
        .child(action)
}
