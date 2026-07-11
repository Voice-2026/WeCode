use super::*;

impl WeCodeApp {
    pub(in crate::app) fn create_project_file(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Start with an empty field (the input shows its own placeholder) rather
        // than a pre-filled value the user has to clear first.
        self.start_file_name_draft(FileNameDraftKind::CreateFile, None, Some(String::new()), cx);
    }

    pub(in crate::app) fn create_project_file_in_directory(
        &mut self,
        parent: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_file_name_draft(
            FileNameDraftKind::CreateFile,
            Some(parent),
            Some(String::new()),
            cx,
        );
    }

    pub(in crate::app) fn create_project_directory(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_file_name_draft(
            FileNameDraftKind::CreateDirectory,
            None,
            Some(String::new()),
            cx,
        );
    }

    pub(in crate::app) fn create_project_directory_in_directory(
        &mut self,
        parent: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_file_name_draft(
            FileNameDraftKind::CreateDirectory,
            Some(parent),
            Some(String::new()),
            cx,
        );
    }

    pub(in crate::app) fn start_file_name_draft(
        &mut self,
        kind: FileNameDraftKind,
        parent: Option<String>,
        value: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let value = value.unwrap_or_else(|| {
            if kind == FileNameDraftKind::Rename {
                self.selected_file_entry()
                    .map(|entry| entry.name)
                    .unwrap_or_default()
            } else {
                generated_project_child_name(
                    &self.state.files,
                    kind == FileNameDraftKind::CreateDirectory,
                )
            }
        });
        self.file_name_draft_kind = Some(kind);
        self.file_name_draft_target = if kind == FileNameDraftKind::Rename {
            self.selected_file_entry.clone()
        } else {
            None
        };
        self.file_name_draft_parent = if kind == FileNameDraftKind::Rename {
            None
        } else {
            parent.filter(|path| !path.trim().is_empty())
        };
        if let Some(parent) = self.file_name_draft_parent.clone() {
            self.file_tree_expanded_dirs.insert(parent.clone());
            self.reload_file_tree_directory(&parent);
        }
        self.file_name_draft_select_all = true;
        self.file_name_draft_value = value;
        self.status_message = match kind {
            FileNameDraftKind::CreateFile => "enter file name".to_string(),
            FileNameDraftKind::CreateDirectory => "enter folder name".to_string(),
            FileNameDraftKind::Rename => "enter new file name".to_string(),
        };
        self.invalidate_file_panel(cx);
    }

    pub(in crate::app) fn set_file_name_draft_value(
        &mut self,
        value: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.file_name_draft_value = value;
        self.invalidate_file_panel(cx);
    }

    pub(in crate::app) fn handle_file_name_draft_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.file_name_draft_kind.is_none() {
            return false;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.control
            || keystroke.modifiers.alt
            || keystroke.modifiers.platform
            || keystroke.modifiers.function
        {
            return false;
        }

        if matches!(keystroke.key.as_str(), "escape" | "Escape") {
            self.cancel_file_name_draft(window, cx);
            true
        } else if matches!(
            keystroke.key.as_str(),
            "enter" | "Enter" | "return" | "Return"
        ) {
            self.confirm_file_name_draft(window, cx);
            true
        } else {
            false
        }
    }

    pub(in crate::app) fn finish_file_name_draft_on_blur(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file_name_draft_kind.is_none() {
            return;
        }
        let value = self.file_name_draft_value.trim();
        let unchanged_rename = self.file_name_draft_kind == Some(FileNameDraftKind::Rename)
            && self
                .selected_file_entry()
                .map(|entry| entry.name == value)
                .unwrap_or(false);
        if value.is_empty() || value.eq_ignore_ascii_case("undefined") || unchanged_rename {
            self.clear_file_name_draft();
            self.status_message = "file name edit canceled".to_string();
            self.invalidate_file_panel(cx);
        } else {
            self.confirm_file_name_draft(window, cx);
        }
    }

    pub(in crate::app) fn cancel_file_name_draft(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_file_name_draft();
        self.status_message = "file name edit canceled".to_string();
        self.invalidate_file_panel(cx);
    }

    pub(in crate::app) fn confirm_file_name_draft(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(kind) = self.file_name_draft_kind else {
            self.status_message = "no file name edit in progress".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let name = self.file_name_draft_value.trim().to_string();
        let parent = self.file_name_draft_parent.clone();
        self.clear_file_name_draft();
        if name.is_empty()
            || name.eq_ignore_ascii_case("undefined")
            || name.contains('/')
            || name.contains('\\')
        {
            self.status_message =
                "file name is required and cannot be undefined or contain path separators"
                    .to_string();
            self.invalidate_file_panel(cx);
            return;
        }

        match kind {
            FileNameDraftKind::CreateFile => {
                self.create_project_file_entry(false, parent, name, cx)
            }
            FileNameDraftKind::CreateDirectory => {
                self.create_project_file_entry(true, parent, name, cx)
            }
            FileNameDraftKind::Rename => self.rename_selected_file_entry_to(name, cx),
        }
    }

    pub(in crate::app) fn create_project_file_entry(
        &mut self,
        directory: bool,
        parent: Option<String>,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for file creation".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let project_path = project.path.clone();
        let parent =
            parent.or_else(|| file_directory_option(&self.file_directory).map(str::to_string));
        let file_directory = self.file_directory.clone();
        let mut expanded_dirs = self
            .file_tree_expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(parent) = &parent {
            expanded_dirs.push(parent.clone());
            expanded_dirs.sort();
            expanded_dirs.dedup();
        }
        let runtime_service = self.runtime_service.clone();
        let item_label = if directory { "directory" } else { "file" };
        self.spawn_file_mutation(
            format!("creating {item_label}: {name}"),
            "file creation task failed",
            cx,
            move || {
                if directory {
                    runtime_service.create_project_directory(
                        &project_path,
                        parent.as_deref(),
                        &name,
                    )?
                } else {
                    runtime_service.create_project_file(&project_path, parent.as_deref(), &name)?
                };
                let (files, file_tree_children) = load_file_mutation_tree(
                    &runtime_service,
                    &project_path,
                    file_directory_option(&file_directory),
                    &expanded_dirs,
                )
                .map_err(|error| file_mutation_refresh_error("file created", error))?;
                let relative_path =
                    join_relative_child_path(parent.as_deref().unwrap_or_default(), &name);
                let git = runtime_service.reload_project_git(&project_path);
                Ok(FileMutationResult {
                    files,
                    file_tree_children,
                    expanded_dirs: expanded_dirs.into_iter().collect(),
                    selection: FileMutationSelection::Single(relative_path.clone()),
                    preview: Some(if directory {
                        ("directory created".to_string(), false, false)
                    } else {
                        (String::new(), true, false)
                    }),
                    git,
                    status: format!("{item_label} created: {relative_path}"),
                    clear_draft: true,
                    saved_editor_path: None,
                    saved_editor_content: None,
                })
            },
        );
    }
    pub(in crate::app) fn rename_selected_file_entry(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_file_entry().is_none() {
            self.status_message = "no selected file entry to rename".to_string();
            self.invalidate_file_panel(cx);
            return;
        }
        self.start_file_name_draft(FileNameDraftKind::Rename, None, None, cx);
    }

    pub(in crate::app) fn rename_selected_file_entry_to(
        &mut self,
        new_name: String,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for file rename".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let Some(entry_path) = self.selected_file_entry.clone() else {
            self.status_message = "no selected file entry to rename".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let Some(entry) = self.selected_file_entry() else {
            self.status_message = "selected file entry is no longer available".to_string();
            self.normalize_selected_file_entry();
            self.invalidate_file_panel(cx);
            return;
        };
        let project_path = project.path.clone();
        let file_directory = self.file_directory.clone();
        let directory = file_directory_option(&file_directory).map(str::to_string);
        let expanded_dirs = self
            .file_tree_expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let runtime_service = self.runtime_service.clone();
        let was_file = matches!(entry.kind, FileKind::File);
        self.spawn_file_mutation(
            format!("renaming file entry: {entry_path}"),
            "file rename task failed",
            cx,
            move || {
                let (files, renamed_path) = runtime_service.rename_project_file_entry(
                    &project_path,
                    &entry_path,
                    &new_name,
                    directory.as_deref(),
                )?;
                let was_expanded = expanded_dirs.iter().any(|path| path == &entry_path);
                let mut next_expanded =
                    file_mutation_prune_expanded(&expanded_dirs, std::slice::from_ref(&entry_path));
                if was_expanded {
                    next_expanded.push(renamed_path.clone());
                }
                let file_tree_children =
                    load_file_mutation_children(&runtime_service, &project_path, &next_expanded)
                        .map_err(|error| file_mutation_refresh_error("file renamed", error))?;
                let preview = if was_file {
                    file_mutation_text_preview(
                        &runtime_service,
                        &project_path,
                        &renamed_path,
                        "file entry renamed",
                    )
                } else {
                    ("directory renamed".to_string(), false, false)
                };
                let git = runtime_service.reload_project_git(&project_path);
                Ok(FileMutationResult {
                    files,
                    file_tree_children,
                    expanded_dirs: next_expanded.into_iter().collect(),
                    selection: FileMutationSelection::Single(renamed_path.clone()),
                    preview: Some(preview),
                    git,
                    status: format!("renamed file entry: {renamed_path}"),
                    clear_draft: true,
                    saved_editor_path: None,
                    saved_editor_content: None,
                })
            },
        );
    }
}
