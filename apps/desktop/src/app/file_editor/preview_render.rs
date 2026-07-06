use super::*;

pub(in crate::app) fn file_preview_window_workspace(
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: FilePreviewWindowSnapshot,
    markdown_state: Option<gpui::Entity<TextViewState>>,
    text_editor: Option<gpui::Entity<InputState>>,
    cx: &mut Context<FilePreviewWindowView>,
) -> impl IntoElement {
    let FilePreviewWindowSnapshot {
        relative_path,
        full_path,
        kind,
        content: _,
        error,
        language: _,
    } = snapshot;
    let title = relative_path
        .as_deref()
        .map(file_editor_label)
        .unwrap_or_else(|| {
            file_editor_i18n(app_entity.clone(), cx, "files.preview.title", "Preview")
        });
    let markdown_source_editor = text_editor.clone();
    let markdown_preview_state = markdown_state.clone();
    let text_preview_editor = text_editor.clone();

    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(color(theme::BG_TERMINAL))
        .text_color(cx.theme().foreground)
        .child(
            file_preview_window_header(app_entity.clone(), title, relative_path.clone(), cx)
                .flex_none(),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .size_full()
                .when_some(error, |this, error| {
                    this.child(file_preview_error(error, cx))
                })
                .when(
                    kind == FilePreviewKind::Image && full_path.is_some(),
                    |this| {
                        let path = full_path.clone().unwrap_or_default();
                        this.child(file_preview_image(
                            path,
                            file_editor_i18n(
                                app_entity.clone(),
                                cx,
                                "files.preview.loading",
                                "Loading preview...",
                            ),
                            file_editor_i18n(
                                app_entity.clone(),
                                cx,
                                "files.preview.read_error",
                                "Could not read this file.",
                            ),
                        ))
                    },
                )
                .when(kind == FilePreviewKind::Markdown, |this| {
                    if let (Some(editor), Some(markdown_state)) =
                        (markdown_source_editor, markdown_preview_state)
                    {
                        this.child(file_preview_markdown_split(editor, markdown_state, cx))
                    } else {
                        this
                    }
                })
                .when(kind == FilePreviewKind::Text, |this| {
                    this.when_some(text_preview_editor, |this, editor| {
                        this.child(file_preview_text(editor, true, cx))
                    })
                }),
        )
}

fn file_preview_window_header(
    app_entity: gpui::Entity<CoduxApp>,
    title: String,
    relative_path: Option<String>,
    cx: &mut Context<FilePreviewWindowView>,
) -> gpui::Div {
    let copy_path_text =
        file_editor_i18n(app_entity.clone(), cx, "files.panel.copy_path", "Copy Path");
    let open_external_text = file_editor_i18n(
        app_entity.clone(),
        cx,
        "files.preview.open_external",
        "Open Externally",
    );
    let reveal_text = file_editor_i18n(
        app_entity.clone(),
        cx,
        "files.preview.reveal_finder",
        "Show in File Manager",
    );
    let path_for_copy = relative_path.clone();
    let path_for_open = relative_path.clone();
    let path_for_reveal = relative_path;

    div()
        .h(px(FILE_EDITOR_TOOLBAR_HEIGHT))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .pr(px(12.0))
        .when(cfg!(target_os = "macos"), |this| this.pl(px(86.0)))
        .when(!cfg!(target_os = "macos"), |this| this.pl(px(18.0)))
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().title_bar)
        .child(
            // Drag lives on the title only: overlapping a Drag ancestor over the
            // buttons makes Windows NC hit-testing return HTCAPTION for them.
            div()
                .min_w_0()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .window_control_area(WindowControlArea::Drag)
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT))
                .truncate()
                .child(title),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(file_preview_toolbar_button(
                    app_entity.clone(),
                    "file-preview-copy-path",
                    HeroIconName::ClipboardDocument,
                    copy_path_text,
                    path_for_copy.is_none(),
                    cx,
                    move |view, _event, _window, cx| {
                        let Some(path) = path_for_copy.clone() else {
                            return;
                        };
                        let app_entity = view.app_entity.clone();
                        cx.update_entity(&app_entity, |app, cx| {
                            app.copy_file_path_to_clipboard(path, cx);
                        });
                    },
                ))
                .child(file_preview_toolbar_button(
                    app_entity.clone(),
                    "file-preview-open-external",
                    HeroIconName::ArrowTopRightOnSquare,
                    open_external_text,
                    path_for_open.is_none(),
                    cx,
                    move |view, _event, _window, cx| {
                        let Some(path) = path_for_open.clone() else {
                            return;
                        };
                        let app_entity = view.app_entity.clone();
                        cx.update_entity(&app_entity, |app, cx| {
                            app.open_file_entry_external(path, cx);
                        });
                    },
                ))
                .child(file_preview_toolbar_button(
                    app_entity.clone(),
                    "file-preview-reveal",
                    HeroIconName::Folder,
                    reveal_text,
                    path_for_reveal.is_none(),
                    cx,
                    move |view, _event, _window, cx| {
                        let Some(path) = path_for_reveal.clone() else {
                            return;
                        };
                        let app_entity = view.app_entity.clone();
                        cx.update_entity(&app_entity, |app, cx| {
                            app.run_file_entry_system_action("reveal", path, cx);
                        });
                    },
                ))
                .when(!cfg!(target_os = "macos"), |this| {
                    this.child(
                        Button::new("file-preview-window-close")
                            .compact()
                            .ghost()
                            .h(px(28.0))
                            .w(px(28.0))
                            .text_color(cx.theme().muted_foreground)
                            .window_control_area(WindowControlArea::Close)
                            .on_click(|_, window, _| window.remove_window())
                            .child(Icon::new(HeroIconName::XMark).size_3()),
                    )
                }),
        )
}

fn file_preview_toolbar_button(
    app_entity: gpui::Entity<CoduxApp>,
    id: &'static str,
    icon: HeroIconName,
    tooltip: String,
    disabled: bool,
    cx: &mut Context<FilePreviewWindowView>,
    on_click: impl Fn(
        &mut FilePreviewWindowView,
        &gpui::ClickEvent,
        &mut Window,
        &mut Context<FilePreviewWindowView>,
    ) + 'static,
) -> impl IntoElement {
    codux_tooltip_container(app_entity, id, tooltip).child(
        Button::new(id)
            .compact()
            .ghost()
            .disabled(disabled)
            .icon(Icon::new(icon).with_size(Size::XSmall))
            .on_click(cx.listener(on_click)),
    )
}

fn file_preview_image(path: String, loading_text: String, error_text: String) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(18.0))
        .child(
            img(PathBuf::from(path))
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain)
                .with_loading(move || file_preview_media_loading(loading_text.clone()))
                .with_fallback(move || file_preview_media_error(error_text.clone())),
        )
        .into_any_element()
}

fn file_preview_media_loading(message: String) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .text_size(rems(0.8125))
        .text_color(color(theme::TEXT_DIM))
        .child(Spinner::new().small().color(color(theme::TEXT_DIM)))
        .child(message)
        .into_any_element()
}

fn file_preview_media_error(message: String) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p_5()
        .text_size(rems(0.8125))
        .line_height(rems(1.25))
        .text_color(color(theme::TEXT_DIM))
        .child(message)
        .into_any_element()
}

fn file_preview_markdown(markdown_state: gpui::Entity<TextViewState>) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .child(
            TextView::new(&markdown_state)
                .size_full()
                .p_5()
                .selectable(true)
                .scrollable(true),
        )
        .into_any_element()
}

fn file_preview_markdown_split(
    editor: gpui::Entity<InputState>,
    markdown_state: gpui::Entity<TextViewState>,
    cx: &mut Context<FilePreviewWindowView>,
) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .child(
            div()
                .flex_1()
                .w(relative(0.5))
                .min_w_0()
                .min_h_0()
                .border_r_1()
                .border_color(cx.theme().border)
                .child(file_preview_text(editor, true, cx)),
        )
        .child(
            div()
                .flex_1()
                .w(relative(0.5))
                .min_w_0()
                .min_h_0()
                .child(file_preview_markdown(markdown_state)),
        )
        .into_any_element()
}

fn file_preview_text(
    editor: gpui::Entity<InputState>,
    read_only: bool,
    cx: &mut Context<FilePreviewWindowView>,
) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .child(
            Input::new(&editor)
                .appearance(false)
                .disabled(read_only)
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(cx.theme().mono_font_size)
                .size_full(),
        )
        .into_any_element()
}

fn file_preview_error(error: String, cx: &mut Context<FilePreviewWindowView>) -> AnyElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p_5()
        .text_size(rems(0.8125))
        .line_height(rems(1.25))
        .text_color(cx.theme().danger)
        .child(error)
        .into_any_element()
}
