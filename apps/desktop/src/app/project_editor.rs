use super::*;
use gpui_component::input::{Input, InputEvent, InputState};

impl CoduxApp {
    pub(in crate::app) fn project_editor_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let language = self.state.settings.language.as_str();
        let locale = locale_from_language_setting(language);
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        let is_editing = self.project_editor_project_id.is_some();
        let title = if is_editing {
            tr("project.edit.title", "Edit Project")
        } else {
            tr("project.create.title", "Create Project")
        };
        let submit_label = if is_editing {
            tr("common.save", "Save")
        } else {
            tr("common.create", "Create")
        };
        let can_submit = !self.project_editor_saving
            && !self.project_editor_name.trim().is_empty()
            && !self.project_editor_path.trim().is_empty();

        child_window_shell(title, cx)
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .flex()
                    .flex_col()
                    .p(px(18.0))
                    .child(project_editor_field(
                        tr("project.editor.name", "Project Name"),
                        "project-editor-name",
                        &self.project_editor_name,
                        "Project",
                        window,
                        cx,
                        |app, value, window, cx| app.set_project_editor_name(value, window, cx),
                    ))
                    .child(self.project_editor_device_field(window, cx))
                    .child(project_editor_path_field(
                        tr("project.editor.directory", "Project Directory"),
                        tr("project.editor.choose_directory.prompt", "Choose"),
                        &self.project_editor_path,
                        window,
                        cx,
                    ))
                    .child(project_editor_symbol_field(
                        tr("project.editor.icon", "Project Icon"),
                        tr("common.none", "None"),
                        self.project_editor_badge_symbol.as_deref(),
                        &self.project_editor_badge_color_hex,
                        cx,
                    ))
                    .child(project_editor_color_field(
                        tr("project.editor.color", "Project Color"),
                        &self.project_editor_badge_color_hex,
                        cx,
                    )),
            )
            .child(dialog_footer_bar(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(dialog_cancel_button(
                        "project-editor-cancel",
                        tr("common.cancel", "Cancel"),
                        cx,
                        |_app, _event, window, _cx| {
                            window.remove_window();
                        },
                    ))
                    .child(
                        dialog_primary_button(
                            "project-editor-save",
                            submit_label,
                            cx,
                            |app, _event, window, cx| {
                                app.save_project_editor(window, cx);
                            },
                        )
                        .disabled(!can_submit)
                        .loading(self.project_editor_saving),
                    ),
                cx,
            ))
    }

    /// The "Device" selector: This Mac (local) + each paired remote host, plus
    /// an inline "pair a new device" form. Selecting a remote device marks the
    /// project as hosted there (its domains route over the controller).
    fn project_editor_device_field(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let locale = locale_from_language_setting(self.state.settings.language.as_str());
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        let hosts = self.runtime_service.saved_remote_hosts();
        let selected = self.project_editor_host_device_id.clone();

        let mut row = div().flex().flex_wrap().items_center().gap_2().child(
            project_editor_device_chip(
                "project-editor-device-local".to_string(),
                tr("project.editor.device.local", "This Mac"),
                selected.is_none(),
                cx,
                |app, window, cx| app.set_project_editor_host_device_id(None, window, cx),
            ),
        );
        for host in &hosts {
            let device_id = host.device_id.clone();
            let is_selected = selected.as_deref() == Some(device_id.as_str());
            let label = if host.host_name.trim().is_empty() {
                host.host_id.clone()
            } else {
                host.host_name.clone()
            };
            row = row.child(project_editor_device_chip(
                format!("project-editor-device-{device_id}"),
                label,
                is_selected,
                cx,
                move |app, window, cx| {
                    app.set_project_editor_host_device_id(Some(device_id.clone()), window, cx)
                },
            ));
        }
        // Pairing now lives in Settings → Remote (unified device management);
        // this just jumps there. The newly paired host then shows up as a chip.
        row = row.child(
            Button::new("project-editor-device-pair")
                .secondary()
                .compact()
                .text_color(cx.theme().secondary_foreground)
                .child(dialog_button_label(tr(
                    "project.editor.device.pair",
                    "Pair device…",
                )))
                .on_click(cx.listener(|app, _event, _window, cx| {
                    app.open_settings_window_with_pane(super::settings::SettingsPane::Remote, cx);
                })),
        );

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .mb(px(24.0))
            .child(
                div()
                    .text_size(rems(0.875))
                    .line_height(rems(1.125))
                    .text_color(color(theme::TEXT))
                    .child(tr("project.editor.device", "Device")),
            )
            .child(row)
            .into_any_element()
    }

}

impl CoduxApp {
    /// The file-picker sub-window: a standard child window (shared title bar via
    /// `child_window_shell`, shared `dialog_footer_bar`) for browsing a local or
    /// remote-host directory and picking a folder. The chosen path is pushed back
    /// to the project-editor window (the opener).
    pub(in crate::app) fn file_picker_window(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let locale = locale_from_language_setting(self.state.settings.language.as_str());
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        let mode = self.file_picker_mode;
        let title = match mode {
            FilePickerMode::OpenFolder => tr("project.editor.browse.title", "Choose Folder"),
            FilePickerMode::OpenFile => tr("file.picker.open.title", "Open File"),
            FilePickerMode::Save => tr("file.picker.save.title", "Save As"),
        };
        let confirm_label = match mode {
            FilePickerMode::OpenFolder => tr("project.editor.browse.use", "Use this folder"),
            FilePickerMode::OpenFile => tr("file.picker.open.confirm", "Open"),
            FilePickerMode::Save => tr("file.picker.save.confirm", "Save"),
        };
        let current = self.project_editor_browse_path.clone();
        let can_confirm = self.file_picker_result_path().is_some() && !self.project_editor_browse_busy;

        // Left: the device sidebar (This Mac + each paired host). Clicking a
        // device re-lists from its root.
        let active_device = self.project_editor_host_device_id.clone();
        let mut devices = div()
            .w(px(160.0))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(8.0))
            .border_r_1()
            .border_color(cx.theme().border)
            .overflow_y_scrollbar()
            .child(file_picker_device_row(
                "file-picker-device-local".to_string(),
                tr("project.editor.device.local", "This Mac"),
                active_device.is_none(),
                cx,
                |app, window, cx| app.file_picker_switch_device(None, window, cx),
            ));
        for host in self.runtime_service.saved_remote_hosts() {
            let device_id = host.device_id.clone();
            let selected = active_device.as_deref() == Some(host.device_id.as_str());
            let label = if host.host_name.trim().is_empty() {
                host.host_id.clone()
            } else {
                host.host_name.clone()
            };
            devices = devices.child(file_picker_device_row(
                format!("file-picker-device-{}", host.device_id),
                label,
                selected,
                cx,
                move |app, window, cx| {
                    app.file_picker_switch_device(Some(device_id.clone()), window, cx)
                },
            ));
        }

        // Right: breadcrumb + listing + new-folder/filename rows.
        let mut list = div()
            .min_h_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .overflow_y_scrollbar();
        if let Some(parent) = self.project_editor_browse_parent.clone() {
            list = list.child(file_picker_entry_row(
                "file-picker-up".to_string(),
                "..".to_string(),
                true,
                false,
                cx,
                move |app, window, cx| {
                    app.project_editor_browse_navigate(Some(parent.clone()), window, cx)
                },
            ));
        }
        for entry in &self.project_editor_browse_entries {
            let path = entry.path.clone();
            let is_dir = entry.is_dir;
            let selected = !is_dir && self.file_picker_selected.as_deref() == Some(path.as_str());
            list = list.child(file_picker_entry_row(
                format!("file-picker-{path}"),
                entry.name.clone(),
                is_dir,
                selected,
                cx,
                move |app, window, cx| {
                    app.file_picker_choose_entry(path.clone(), is_dir, window, cx)
                },
            ));
        }

        let mut right = div()
            .min_h_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .p(px(16.0))
            .child(file_picker_breadcrumb(&current, cx))
            .child(list)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(project_editor_input(
                        "file-picker-newfolder",
                        &self.project_editor_browse_new_folder,
                        "New folder name",
                        window,
                        cx,
                        |app, value, window, cx| {
                            app.set_project_editor_browse_new_folder(value, window, cx)
                        },
                    )))
                    .child(
                        Button::new("file-picker-create")
                            .secondary()
                            .compact()
                            .text_color(cx.theme().secondary_foreground)
                            .child(dialog_button_label(tr(
                                "project.editor.browse.new_folder",
                                "New folder",
                            )))
                            .disabled(self.project_editor_browse_busy)
                            .on_click(cx.listener(|app, _event, window, cx| {
                                app.project_editor_browse_create_folder(window, cx);
                            })),
                    ),
            );
        // Save mode: a filename row (prefilled when an existing file is clicked).
        if mode == FilePickerMode::Save {
            right = right.child(div().min_w_0().child(project_editor_input(
                "file-picker-filename",
                &self.file_picker_filename,
                "File name",
                window,
                cx,
                |app, value, window, cx| app.set_file_picker_filename(value, window, cx),
            )));
        }
        if let Some(error) = self.project_editor_browse_error.as_ref() {
            right = right.child(
                div()
                    .text_size(rems(0.8125))
                    .text_color(color(theme::ORANGE))
                    .child(error.clone()),
            );
        }

        let body = div()
            .min_h_0()
            .flex_1()
            .flex()
            .child(devices)
            .child(right);

        child_window_shell(SharedString::from(title), cx)
            .child(body)
            .child(dialog_footer_bar(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(dialog_cancel_button(
                        "file-picker-cancel",
                        tr("common.cancel", "Cancel"),
                        cx,
                        |_app, _event, window, _cx| {
                            window.remove_window();
                        },
                    ))
                    .child(
                        dialog_primary_button(
                            "file-picker-use",
                            confirm_label,
                            cx,
                            |app, _event, window, cx| app.file_picker_select(window, cx),
                        )
                        .disabled(!can_confirm),
                    ),
                cx,
            ))
            .into_any_element()
    }
}

fn file_picker_entry_row(
    id: String,
    label: String,
    is_dir: bool,
    selected: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let mut row = div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .gap_2()
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| on_click(app, window, cx)))
        .child(
            Icon::new(if is_dir {
                HeroIconName::Folder
            } else {
                HeroIconName::Document
            })
            .size_3()
            .text_color(color(theme::TEXT_MUTED)),
        )
        .child(
            div()
                .text_size(rems(0.8125))
                .text_color(color(theme::TEXT))
                .truncate()
                .child(label),
        );
    if selected {
        row = row.bg(cx.theme().secondary);
    }
    row.into_any_element()
}

/// A device row in the file picker's left sidebar (This Mac / a host).
fn file_picker_device_row(
    id: String,
    label: String,
    selected: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let mut row = div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .gap_2()
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| on_click(app, window, cx)))
        .child(
            Icon::new(HeroIconName::GlobeAlt)
                .size_3()
                .text_color(color(theme::TEXT_MUTED)),
        )
        .child(
            div()
                .text_size(rems(0.8125))
                .text_color(color(theme::TEXT))
                .truncate()
                .child(label),
        );
    if selected {
        row = row.bg(cx.theme().secondary);
    }
    row.into_any_element()
}

/// A clickable breadcrumb for the current directory: each path segment navigates
/// to that ancestor.
fn file_picker_breadcrumb(path: &str, cx: &mut Context<CoduxApp>) -> AnyElement {
    let trimmed = path.trim();
    let mut row = div().flex().flex_wrap().items_center().gap_1();
    if trimmed.is_empty() {
        return row
            .child(
                div()
                    .text_size(rems(0.8125))
                    .text_color(color(theme::TEXT_MUTED))
                    .child("Loading…"),
            )
            .into_any_element();
    }
    let absolute = trimmed.starts_with('/');
    if absolute {
        row = row.child(file_picker_crumb("file-picker-crumb-root", "/", "/", cx));
    }
    let mut cumulative = String::new();
    for part in trimmed.split('/').filter(|segment| !segment.is_empty()) {
        cumulative = if absolute {
            format!("{}/{part}", cumulative.trim_end_matches('/'))
        } else if cumulative.is_empty() {
            part.to_string()
        } else {
            format!("{cumulative}/{part}")
        };
        row = row.child(
            div()
                .text_size(rems(0.75))
                .text_color(color(theme::TEXT_DIM))
                .child("/"),
        );
        row = row.child(file_picker_crumb(
            &format!("file-picker-crumb-{cumulative}"),
            part,
            &cumulative,
            cx,
        ));
    }
    row.into_any_element()
}

fn file_picker_crumb(
    id: &str,
    label: &str,
    target: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let target = target.to_string();
    div()
        .id(SharedString::from(id.to_string()))
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary))
        .text_size(rems(0.8125))
        .text_color(color(theme::TEXT))
        .child(label.to_string())
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.project_editor_browse_navigate(Some(target.clone()), window, cx)
        }))
        .into_any_element()
}

fn project_editor_device_chip(
    id: String,
    label: String,
    selected: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(id))
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(color(if selected {
            theme::BORDER
        } else {
            theme::BORDER_SOFT
        }))
        .bg(if selected {
            cx.theme().secondary_hover
        } else {
            cx.theme().secondary
        })
        .text_size(rems(0.8125))
        .text_color(color(theme::TEXT))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| on_click(app, window, cx)))
        .child(label)
        .into_any_element()
}

struct ProjectEditorSymbol {
    id: &'static str,
    icon: Option<HeroIconName>,
}

const PROJECT_EDITOR_SYMBOLS: &[ProjectEditorSymbol] = &[
    ProjectEditorSymbol {
        id: "none",
        icon: None,
    },
    ProjectEditorSymbol {
        id: "terminal",
        icon: Some(HeroIconName::CommandLine),
    },
    ProjectEditorSymbol {
        id: "folder",
        icon: Some(HeroIconName::Folder),
    },
    ProjectEditorSymbol {
        id: "shippingbox",
        icon: Some(HeroIconName::Sparkles),
    },
    ProjectEditorSymbol {
        id: "hammer",
        icon: Some(HeroIconName::WrenchScrewdriver),
    },
    ProjectEditorSymbol {
        id: "server.rack",
        icon: Some(HeroIconName::GlobeAlt),
    },
    ProjectEditorSymbol {
        id: "globe",
        icon: Some(HeroIconName::GlobeAlt),
    },
    ProjectEditorSymbol {
        id: "bolt",
        icon: Some(HeroIconName::Star),
    },
    ProjectEditorSymbol {
        id: "wrench",
        icon: Some(HeroIconName::Cog6Tooth),
    },
    ProjectEditorSymbol {
        id: "doc.text",
        icon: Some(HeroIconName::Document),
    },
    ProjectEditorSymbol {
        id: "book",
        icon: Some(HeroIconName::BookOpen),
    },
    ProjectEditorSymbol {
        id: "person.2",
        icon: Some(HeroIconName::UserCircle),
    },
];

fn project_editor_field(
    label: String,
    id: &'static str,
    value: &str,
    placeholder: impl Into<String>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    action: impl Fn(&mut CoduxApp, String, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(24.0))
        .child(
            div()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
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

fn project_editor_symbol_field(
    label: String,
    none_label: String,
    selected_symbol: Option<&str>,
    selected_color: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let accent = hex_color(selected_color).unwrap_or(theme::ACCENT);
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(24.0))
        .child(
            div()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT))
                .child(label),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(PROJECT_EDITOR_SYMBOLS.iter().map(|symbol| {
                    let id = symbol.id;
                    let selected = if id == "none" {
                        selected_symbol.is_none()
                    } else {
                        selected_symbol == Some(id)
                    };
                    div()
                        .id(SharedString::from(format!("project-editor-symbol-{id}")))
                        .size(px(36.0))
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(color(if selected {
                            theme::BORDER
                        } else {
                            theme::BORDER_SOFT
                        }))
                        .bg(if selected {
                            cx.theme().secondary_hover
                        } else {
                            cx.theme().secondary
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.theme().secondary_hover))
                        .on_click(cx.listener(move |app, _event, window, cx| {
                            let next = (id != "none").then(|| id.to_string());
                            app.set_project_editor_badge_symbol(next, window, cx);
                        }))
                        .child(match symbol.icon.clone() {
                            Some(icon) => Icon::new(icon)
                                .size_4()
                                .text_color(color(accent))
                                .into_any_element(),
                            None => div()
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(color(theme::TEXT_MUTED))
                                .child(none_label.clone())
                                .into_any_element(),
                        })
                        .into_any_element()
                })),
        )
        .into_any_element()
}

fn project_editor_color_field(
    label: String,
    selected_color: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT))
                .child(label),
        )
        .child(div().flex().flex_wrap().items_center().gap_3().children(
            PROJECT_BADGE_COLORS.iter().map(|value| {
                let selected = *value == selected_color;
                let swatch = hex_color(value).unwrap_or(theme::ACCENT);
                div()
                    .id(SharedString::from(format!("project-editor-color-{value}")))
                    .size(px(24.0))
                    .rounded_full()
                    .bg(color(swatch))
                    .border_1()
                    .border_color(color(if selected {
                        theme::TEXT
                    } else {
                        theme::BORDER_SOFT
                    }))
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.86))
                    .on_click(cx.listener(move |app, _event, window, cx| {
                        app.set_project_editor_badge_color((*value).to_string(), window, cx);
                    }))
                    .into_any_element()
            }),
        ))
        .into_any_element()
}

fn hex_color(value: &str) -> Option<u32> {
    let value = value.trim().trim_start_matches('#');
    if value.len() == 6 {
        u32::from_str_radix(value, 16).ok()
    } else {
        None
    }
}

fn project_editor_input(
    id: &'static str,
    value: &str,
    placeholder: impl Into<String>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    action: impl Fn(&mut CoduxApp, String, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let value = value.to_string();
    let placeholder = placeholder.into();
    let input_state = window.use_keyed_state(SharedString::from(id), cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(value.clone())
            .placeholder(placeholder.clone())
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
    label: String,
    choose_label: String,
    path: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .mb(px(24.0))
        .child(
            div()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT))
                .child(label),
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
                        .compact()
                        .text_color(cx.theme().secondary_foreground)
                        .child(dialog_button_label(choose_label))
                        .on_click(cx.listener(|app, _event, window, cx| {
                            app.choose_project_editor_directory(window, cx);
                        })),
                ),
        )
        .into_any_element()
}
