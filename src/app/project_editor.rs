use super::*;
use gpui_component::input::{Input, InputEvent, InputState};

impl CoduxApp {
    pub(in crate::app) fn project_editor_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(color(theme::BG))
            .child(column_header(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .line_height(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color(theme::TEXT))
                            .child("编辑项目"),
                    )
                    .child(header_icon_button(
                        "project-editor-close",
                        IconName::Close,
                        cx,
                        |_app, _event, window, _cx| window.remove_window(),
                    )),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p(px(18.0))
                    .child(project_editor_field(
                        "项目名称",
                        "project-editor-name",
                        &self.project_editor_name,
                        "Project",
                        window,
                        cx,
                        |app, value, window, cx| app.set_project_editor_name(value, window, cx),
                    ))
                    .child(project_editor_path_field(
                        &self.project_editor_path,
                        window,
                        cx,
                    ))
                    .child(
                        div()
                            .mt(px(4.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .line_height(px(16.0))
                                    .text_color(color(theme::TEXT_DIM))
                                    .truncate()
                                    .child(self.status_message.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Button::new("project-editor-cancel")
                                            .ghost()
                                            .text_color(cx.theme().secondary_foreground)
                                            .label("取消")
                                            .on_click(cx.listener(|_app, _event, window, _cx| {
                                                window.remove_window();
                                            })),
                                    )
                                    .child(
                                        Button::new("project-editor-save")
                                            .secondary()
                                            .text_color(cx.theme().secondary_foreground)
                                            .label("保存")
                                            .on_click(cx.listener(|app, _event, window, cx| {
                                                app.save_project_editor(window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
    }
}

fn project_editor_field(
    label: &'static str,
    id: &'static str,
    value: &str,
    placeholder: &'static str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    action: impl Fn(&mut CoduxApp, String, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child(label),
        )
        .child(project_editor_input(
            id,
            value,
            placeholder,
            window,
            cx,
            action,
        ))
        .into_any_element()
}

fn project_editor_input(
    id: &'static str,
    value: &str,
    placeholder: &'static str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    action: impl Fn(&mut CoduxApp, String, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let value = value.to_string();
    let input_state = window.use_keyed_state(SharedString::from(id), cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(value.clone())
            .placeholder(placeholder)
    });
    input_state.update(cx, |state, cx| {
        if state.value().as_ref() != value {
            state.set_value(value.clone(), window, cx);
        }
    });
    cx.subscribe_in(
        &input_state,
        window,
        move |app, state, event, window, cx| {
            if matches!(event, InputEvent::Change) {
                action(app, state.read(cx).value().to_string(), window, cx);
            }
        },
    )
    .detach();

    Input::new(&input_state)
        .with_size(gpui_component::Size::Medium)
        .into_any_element()
}

fn project_editor_path_field(
    path: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child("项目目录"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().min_w_0().child(project_editor_input(
                    "project-editor-path",
                    path,
                    "/path/to/project",
                    window,
                    cx,
                    |app, value, window, cx| app.set_project_editor_path(value, window, cx),
                )))
                .child(
                    Button::new("project-editor-choose-path")
                        .secondary()
                        .text_color(cx.theme().secondary_foreground)
                        .label("选择")
                        .on_click(cx.listener(|app, _event, window, cx| {
                            app.choose_project_editor_directory(window, cx);
                        })),
                ),
        )
        .into_any_element()
}
