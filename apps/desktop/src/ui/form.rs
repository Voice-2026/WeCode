use gpui::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size,
    form::{Field, Form, field, v_form},
    input::{Input, InputState},
};

use super::tokens::*;

pub(crate) fn form_page(
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
    content: impl IntoElement,
    cx: &App,
) -> AnyElement {
    div()
        .w_full()
        .px(FORM_PAGE_PADDING_X)
        .py(FORM_PAGE_PADDING_Y)
        .child(
            div()
                .w_full()
                .max_w(FORM_PAGE_MAX_WIDTH)
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .pb(gpui::px(16.0))
                        .flex()
                        .flex_col()
                        .gap(gpui::px(6.0))
                        .child(
                            div()
                                .text_size(FORM_PAGE_TITLE_SIZE)
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().foreground)
                                .child(title.into()),
                        )
                        .child(
                            div()
                                .text_size(FORM_PAGE_DESCRIPTION_SIZE)
                                .text_color(cx.theme().muted_foreground)
                                .child(description.into()),
                        ),
                )
                .child(content),
        )
        .into_any_element()
}

pub(crate) fn form_card(
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
    completed: bool,
    content: impl IntoElement,
    cx: &App,
) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .px(FORM_CARD_PADDING_X)
        .py(FORM_CARD_PADDING_Y)
        .rounded(gpui::px(10.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(gpui::px(12.0))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(gpui::px(4.0))
                        .child(
                            div()
                                .text_size(FORM_SECTION_TITLE_SIZE)
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().foreground)
                                .child(title.into()),
                        )
                        .child(
                            div()
                                .text_size(FORM_SECTION_DESCRIPTION_SIZE)
                                .text_color(cx.theme().muted_foreground)
                                .child(description.into()),
                        ),
                )
                .when(completed, |this| {
                    this.child(
                        Icon::new(IconName::CircleCheck)
                            .size(gpui::px(16.0))
                            .text_color(cx.theme().primary),
                    )
                }),
        )
        .child(div().mt(gpui::px(14.0)).min_w_0().child(content))
        .into_any_element()
}

pub(crate) fn form_control_field(
    label: impl Into<gpui::SharedString>,
    control: impl IntoElement,
    _cx: &App,
) -> Field {
    field().label(label.into()).child(control)
}

pub(crate) fn form_input_field(
    label: impl Into<gpui::SharedString>,
    state: &gpui::Entity<InputState>,
    multiline: bool,
    _cx: &App,
) -> Field {
    field().label(label.into()).child(
        Input::new(state)
            .with_size(Size::Medium)
            .w_full()
            .when(multiline, |input| input.h(FORM_MULTILINE_HEIGHT)),
    )
}

pub(crate) fn responsive_form(
    window: &Window,
    wide_columns: usize,
    compact_columns: usize,
) -> Form {
    let columns = if window.viewport_size().width.as_f32() < 1480.0 {
        compact_columns
    } else {
        wide_columns
    };
    v_form()
        .columns(columns.max(1))
        .label_text_size(FORM_FIELD_LABEL_SIZE)
}

pub(crate) fn form_field_label(label: impl Into<gpui::SharedString>, cx: &App) -> AnyElement {
    div()
        .text_size(FORM_FIELD_LABEL_SIZE)
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().foreground)
        .child(label.into())
        .into_any_element()
}
