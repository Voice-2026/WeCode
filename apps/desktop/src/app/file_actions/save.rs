use super::*;

impl WeCodeApp {
    pub(in crate::app) fn save_selected_file_preview(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project_path) = self.selected_worktree_path() else {
            self.status_message = "no selected project for file save".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let entry_path = self
            .active_file_editor_tab
            .clone()
            .or_else(|| self.selected_file_entry.clone());
        let Some(entry_path) = entry_path else {
            self.status_message = "no selected file to save".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let tab_editable = self
            .file_editor_tabs
            .iter()
            .find(|tab| tab.relative_path == entry_path)
            .map(|tab| tab.editable)
            .unwrap_or(self.file_editable);
        if !tab_editable {
            self.status_message = "selected file preview is read-only".to_string();
            self.invalidate_file_panel(cx);
            return;
        }
        let content = self
            .active_file_editor_state()
            .map(|state| state.read(cx).value().to_string())
            .unwrap_or_else(|| self.file_preview.clone());
        let file_directory = self.file_directory.clone();
        let expanded_dirs = self
            .file_tree_expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let runtime_service = self.runtime_service.clone();
        self.spawn_file_mutation(
            format!("saving file: {entry_path}"),
            "file save task failed",
            cx,
            move || {
                let preview =
                    runtime_service.write_project_file(&project_path, &entry_path, &content)?;
                let (files, file_tree_children) = load_file_mutation_tree(
                    &runtime_service,
                    &project_path,
                    file_directory_option(&file_directory),
                    &expanded_dirs,
                )
                .map_err(|error| file_mutation_refresh_error("file saved", error))?;
                let git = runtime_service.reload_project_git(&project_path);
                Ok(FileMutationResult {
                    files,
                    file_tree_children,
                    expanded_dirs: expanded_dirs.into_iter().collect(),
                    selection: FileMutationSelection::Single(entry_path.clone()),
                    preview: Some((preview, true, false)),
                    git,
                    status: format!("file saved: {entry_path}"),
                    clear_draft: false,
                    saved_editor_path: Some(entry_path),
                    saved_editor_content: Some(content.clone()),
                })
            },
        );
    }

    pub(in crate::app) fn reload_active_file_editor_tab(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project_path) = self.selected_worktree_path() else {
            self.status_message = "no selected project for file reload".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let Some(entry_path) = self.active_file_editor_tab.clone() else {
            self.status_message = "no active file to reload".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let runtime_service = self.runtime_service.clone();
        let generation = self.project_switch_generation;
        let scope_key = super::app_state::current_worktree_scope_key(&self.state);
        self.status_message = format!("reloading file: {entry_path}");
        self.invalidate_file_panel(cx);
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            let worker_entry_path = entry_path.clone();
            let result = wecode_runtime::async_runtime::run_limited_blocking_with_priority(
                wecode_runtime::async_runtime::BLOCKING_PRIORITY_FOREGROUND + generation,
                move || {
                    runtime_service.read_project_file_edit_buffer(&project_path, &worker_entry_path)
                },
            )
            .await
            .ok()
            .unwrap_or_else(|| Err("file reload task failed".to_string()));

            let _ = this.update_in(cx, |app, window, cx| {
                if app.project_switch_generation != generation
                    || super::app_state::current_worktree_scope_key(&app.state) != scope_key
                {
                    app.invalidate_file_panel(cx);
                    return;
                }
                app.apply_active_file_editor_tab_reload(entry_path, result, window, cx);
            });
        })
        .detach();
    }

    fn apply_active_file_editor_tab_reload(
        &mut self,
        entry_path: String,
        result: Result<(String, bool), String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok((content, editable)) => {
                let key = self.file_editor_state_key(&entry_path);
                let language = self
                    .file_editor_tabs
                    .iter()
                    .find(|tab| tab.relative_path == entry_path)
                    .map(|tab| tab.language.clone())
                    .unwrap_or_else(|| "text".to_string());
                if let Some(editor) = self.file_editor_states.get(&key) {
                    editor.update(cx, |state, cx| {
                        state.set_value(content.clone(), window, cx);
                        state.focus(window, cx);
                    });
                } else {
                    self.ensure_file_editor_state(
                        key,
                        entry_path.clone(),
                        language,
                        content.clone(),
                        window,
                        cx,
                    );
                }
                if let Some(tab) = self
                    .file_editor_tabs
                    .iter_mut()
                    .find(|tab| tab.relative_path == entry_path)
                {
                    tab.editable = editable;
                    tab.dirty = false;
                }
                self.file_preview = content;
                self.file_editable = editable;
                self.file_dirty = false;
                self.status_message = format!("file reloaded: {entry_path}");
            }
            Err(error) => self.status_message = format!("failed to reload file: {error}"),
        }
        self.invalidate_file_panel(cx);
    }
}
