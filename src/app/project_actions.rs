use super::*;

impl CoduxApp {
    pub(super) fn refresh_files_panel_state(&mut self) {
        let Some(project) = &self.state.selected_project else {
            return;
        };
        self.state.files = self
            .runtime_service
            .reload_project_files(&project.path, file_directory_option(&self.file_directory));
        self.refresh_file_tree_cache();
        self.normalize_selected_file_entry();
    }

    pub(super) fn refresh_git_panel_state(&mut self) {
        let Some(project) = self.state.selected_project.clone() else {
            return;
        };
        self.state.git = self.runtime_service.reload_project_git(&project.path);
        self.refresh_git_review_for_project(&project.path);
        self.normalize_selected_git_file();
        self.normalize_selected_git_branch();
    }

    pub(super) fn select_project(
        &mut self,
        project_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous_project_id = self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.id.clone());
        if previous_project_id.as_deref() == Some(project_id.as_str()) {
            return;
        }
        self.cache_current_project_view();
        if previous_project_id.is_some() {
            self.spawn_persist_terminal_state(cx);
        }
        match self.runtime_service.select_project(&project_id) {
            Ok(()) => self.status_message = "selected project saved to state.json".to_string(),
            Err(error) => self.status_message = format!("selected in memory only: {error}"),
        }
        self.project_switch_generation = self.project_switch_generation.wrapping_add(1);
        let switch_generation = self.project_switch_generation;
        self.apply_selected_project_shell(&project_id);
        self.memory_manager_scope = "project".to_string();
        self.memory_manager_project_id = Some(project_id.clone());
        self.file_directory.clear();
        self.reset_file_tree_cache();
        self.file_preview = "select a file to preview it".to_string();
        self.file_editable = false;
        self.file_dirty = false;
        self.clear_file_selection();
        self.selected_git_file = None;
        self.normalize_selected_git_branch();
        self.git_diff_preview = "select a changed file to preview its diff".to_string();
        self.git_review_content = None;
        self.terminal_layout_loading = true;
        self.terminals.clear();
        self.active_terminal_id = 1;
        self.next_terminal_index = 1;
        self.notify_task_column(cx);
        self.spawn_project_switch_load(project_id, switch_generation, cx);
        self.spawn_runtime_scheduled_refresh(cx);
        self.sync_project_list_store(cx);
        cx.notify();
    }

    pub(super) fn apply_selected_project_shell(&mut self, project_id: &str) {
        let Some(project) = self
            .state
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
        else {
            return;
        };

        self.state.selected_project = Some(project.clone());
        self.state.git = GitSummary::default();
        self.git_review = GitReviewSummary::default();
        self.state.files.clear();
        if let Some(cache) = self.project_view_cache.get(project_id).cloned() {
            self.state.ai_history = cache.ai_history;
            self.state.ai_global_history = cache.ai_global_history;
            self.state.ai_session_detail = cache.ai_session_detail;
            self.state.memory = cache.memory;
            self.state.memory_manager = cache.memory_manager;
            self.state.worktrees = cache.worktrees;
            self.selected_ai_session_id = self
                .state
                .ai_history
                .sessions
                .first()
                .map(|session| session.id.clone());
        } else {
            self.selected_ai_session_id = None;
            self.state.ai_history = AIHistorySummary {
                is_loading: true,
                detail: "loading".to_string(),
                ..AIHistorySummary::default()
            };
            self.state.ai_session_detail = None;
            self.state.memory = MemorySummary::default();
            self.state.memory_manager = MemoryManagerSnapshot::default();
            self.state.worktrees = self
                .runtime_service
                .reload_worktrees_from_state(Some(&project.id), Some(&project.path));
        }
        self.state.terminal_layout = TerminalLayoutSummary::default();
        self.state.terminal_runtime = TerminalRuntimeSummary::default();
    }

    pub(super) fn cache_current_project_view(&mut self) {
        let Some(project_id) = self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.id.clone())
        else {
            return;
        };
        self.project_view_cache.insert(
            project_id,
            ProjectViewCache {
                ai_history: self.state.ai_history.clone(),
                ai_global_history: self.state.ai_global_history.clone(),
                ai_session_detail: self.state.ai_session_detail.clone(),
                memory: self.state.memory.clone(),
                memory_manager: self.state.memory_manager.clone(),
                worktrees: self.state.worktrees.clone(),
            },
        );
    }

    pub(super) fn notify_task_column(&self, cx: &mut Context<Self>) {
        if let Some(view) = &self.task_column_view {
            view.update(cx, |_view, cx| cx.notify());
        }
    }

    pub(super) fn spawn_persist_terminal_state(&mut self, _cx: &mut Context<Self>) {
        let Some(owner_id) = super::ai_runtime_status::terminal_layout_owner_id(&self.state) else {
            return;
        };
        self.refresh_terminal_slot_snapshots();
        let (tabs, active_tab_id, top_panes, active_slot_id) = self.terminal_layout_snapshot();
        let (active_terminal_id, active_runtime_slot_id, sessions) =
            self.terminal_runtime_snapshot();
        let runtime_service = self.runtime_service.clone();
        let support_dir = self.state.support_dir.clone();
        codux_runtime::async_runtime::spawn_blocking(move || {
            let _ = runtime_service.save_terminal_layout(
                &owner_id,
                tabs,
                active_tab_id,
                top_panes,
                active_slot_id,
            );
            let _ = TerminalRuntimeService::new(support_dir).save_from_gpui(
                active_terminal_id,
                active_runtime_slot_id,
                sessions,
            );
        });
    }

    pub(super) fn spawn_project_switch_load(
        &mut self,
        project_id: String,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self
            .state
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
        else {
            return;
        };
        let projects = self.state.projects.clone();
        let runtime_service = self.runtime_service.clone();
        let runtime_inventory = self.runtime.clone();
        let terminal_state = self.state.clone();
        let terminal_runtime_service = runtime_service.clone();
        let terminal_project = project.clone();
        let terminal_runtime_inventory = runtime_inventory.clone();
        let primary_runtime_service = runtime_service.clone();
        let primary_project = project.clone();
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            let terminal = codux_runtime::async_runtime::spawn_blocking(move || {
                let worktrees = terminal_runtime_service.reload_worktrees_from_state(
                    Some(&terminal_project.id),
                    Some(&terminal_project.path),
                );
                let terminal_owner_id = worktrees
                    .selected_worktree_id
                    .as_deref()
                    .unwrap_or(terminal_project.id.as_str())
                    .to_string();
                let terminal_layout =
                    terminal_runtime_service.reload_terminal_layout(Some(&terminal_owner_id));
                let terminal_runtime = terminal_runtime_service.reload_terminal_runtime();
                let mut terminal_state = terminal_state;
                terminal_state.selected_project = Some(terminal_project.clone());
                terminal_state.worktrees = worktrees.clone();
                terminal_state.terminal_layout = terminal_layout.clone();
                terminal_state.terminal_runtime = terminal_runtime.clone();
                prewarm_terminal_restore(&terminal_state, &terminal_runtime_inventory);
                ProjectSwitchTerminalLoad {
                    project_id: terminal_project.id,
                    generation,
                    worktrees,
                    terminal_layout,
                    terminal_runtime,
                }
            })
            .await
            .ok();

            let _ = this.update(cx, |app, cx| {
                if let Some(terminal) = terminal {
                    app.apply_project_switch_terminal_load(terminal, cx);
                }
            });

            let primary = codux_runtime::async_runtime::spawn_blocking(move || {
                let worktrees = primary_runtime_service.reload_worktrees_from_state(
                    Some(&primary_project.id),
                    Some(&primary_project.path),
                );
                let ai_history =
                    primary_runtime_service.reload_project_ai_history(&primary_project.path);
                let ai_session_detail = ai_history.sessions.first().map(|session| {
                    primary_runtime_service
                        .reload_project_ai_session_detail(&primary_project.path, &session.id)
                });
                ProjectSwitchPrimaryLoad {
                    project_id: primary_project.id,
                    generation,
                    ai_history,
                    ai_session_detail,
                    worktrees,
                }
            })
            .await
            .ok();

            let _ = this.update(cx, |app, cx| {
                if let Some(primary) = primary {
                    app.apply_project_switch_primary_load(primary, cx);
                }
            });

            let load = codux_runtime::async_runtime::spawn_blocking(move || {
                let ai_global_history = runtime_service.reload_global_ai_history();
                let memory = runtime_service.reload_memory(Some(&project.id));
                let memory_manager = runtime_service.reload_memory_manager(
                    &projects,
                    "project",
                    Some(&project.id),
                    "active",
                );
                let worktrees =
                    runtime_service.reload_worktrees(Some(&project.id), Some(&project.path));
                ProjectSwitchLoad {
                    project_id: project.id,
                    generation,
                    ai_global_history,
                    memory,
                    memory_manager,
                    worktrees,
                }
            })
            .await
            .ok();

            let _ = this.update(cx, |app, cx| {
                if let Some(load) = load {
                    app.apply_project_switch_load(load, cx);
                }
            });
        })
        .detach();
    }

    pub(super) fn apply_project_switch_terminal_load(
        &mut self,
        load: ProjectSwitchTerminalLoad,
        cx: &mut Context<Self>,
    ) {
        if self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.id.as_str())
            != Some(load.project_id.as_str())
            || self.project_switch_generation != load.generation
        {
            return;
        }
        self.state.worktrees = load.worktrees;
        self.schedule_terminal_layout_restore(
            load.terminal_layout,
            load.terminal_runtime,
            load.generation,
            cx,
        );
        self.notify_task_column(cx);
        cx.notify();
    }

    pub(super) fn apply_project_switch_primary_load(
        &mut self,
        load: ProjectSwitchPrimaryLoad,
        cx: &mut Context<Self>,
    ) {
        let existing = self.project_view_cache.get(&load.project_id).cloned();
        self.project_view_cache.insert(
            load.project_id.clone(),
            ProjectViewCache {
                ai_history: load.ai_history.clone(),
                ai_global_history: existing
                    .as_ref()
                    .map(|cache| cache.ai_global_history.clone())
                    .unwrap_or_else(|| self.state.ai_global_history.clone()),
                ai_session_detail: load.ai_session_detail.clone(),
                memory: existing
                    .as_ref()
                    .map(|cache| cache.memory.clone())
                    .unwrap_or_else(|| self.state.memory.clone()),
                memory_manager: existing
                    .as_ref()
                    .map(|cache| cache.memory_manager.clone())
                    .unwrap_or_else(|| self.state.memory_manager.clone()),
                worktrees: load.worktrees.clone(),
            },
        );
        if self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.id.as_str())
            != Some(load.project_id.as_str())
            || self.project_switch_generation != load.generation
        {
            return;
        }
        self.state.ai_history = load.ai_history;
        self.state.ai_session_detail = load.ai_session_detail;
        self.state.worktrees = load.worktrees;
        self.selected_ai_session_id = self
            .state
            .ai_history
            .sessions
            .first()
            .map(|session| session.id.clone());
        self.refresh_ai_history_after_project_switch(cx);
        self.cache_current_project_view();
        self.notify_task_column(cx);
        cx.notify();
    }

    pub(super) fn apply_project_switch_load(
        &mut self,
        load: ProjectSwitchLoad,
        cx: &mut Context<Self>,
    ) {
        let entry = self
            .project_view_cache
            .entry(load.project_id.clone())
            .or_insert_with(|| ProjectViewCache {
                ai_history: self.state.ai_history.clone(),
                ai_global_history: self.state.ai_global_history.clone(),
                ai_session_detail: self.state.ai_session_detail.clone(),
                memory: self.state.memory.clone(),
                memory_manager: self.state.memory_manager.clone(),
                worktrees: self.state.worktrees.clone(),
            });
        entry.ai_global_history = load.ai_global_history.clone();
        entry.memory = load.memory.clone();
        entry.memory_manager = load.memory_manager.clone();
        entry.worktrees = load.worktrees.clone();
        if self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.id.as_str())
            != Some(load.project_id.as_str())
            || self.project_switch_generation != load.generation
        {
            return;
        }
        self.state.ai_global_history = load.ai_global_history;
        self.state.memory = load.memory;
        self.state.memory_manager = load.memory_manager;
        self.state.worktrees = load.worktrees;
        self.cache_current_project_view();
        self.notify_task_column(cx);
        cx.notify();
    }

    pub(super) fn schedule_terminal_layout_restore(
        &mut self,
        terminal_layout: TerminalLayoutSummary,
        terminal_runtime: TerminalRuntimeSummary,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        self.state.terminal_layout = terminal_layout.clone();
        self.state.terminal_runtime = terminal_runtime.clone();
        self.terminal_layout_loading = true;
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            let _ = this.update(cx, |app, cx| {
                if app.project_switch_generation != generation {
                    return;
                }
                app.apply_terminal_layout_from_summary(terminal_layout, terminal_runtime, cx);
            });
        })
        .detach();
    }

    pub(super) fn reload_runtime_state(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.state = self.runtime_service.reload_state();
        self.project_open_applications = self.runtime_service.project_open_applications();
        self.file_directory.clear();
        self.reset_file_tree_cache();
        self.file_editable = false;
        self.file_dirty = false;
        self.clear_file_selection();
        self.selected_git_file = None;
        self.normalize_selected_git_branch();
        self.git_diff_preview = "select a changed file to preview its diff".to_string();
        self.git_review_content = None;
        self.normalize_selected_ai_session();
        self.normalize_selected_runtime_session();
        self.normalize_selected_ssh_profile();
        self.status_message = "state reloaded from Codux support files".to_string();
        self.sync_project_list_store(cx);
        cx.notify();
    }

    pub(super) fn reload_project_open_applications(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_open_applications = self.runtime_service.project_open_applications();
        self.status_message = "project application list refreshed".to_string();
        cx.notify();
    }

    pub(super) fn reveal_selected_project_in_file_manager(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to reveal".to_string();
            cx.notify();
            return;
        };

        match self
            .runtime_service
            .project_reveal_in_file_manager(&project.path)
        {
            Ok(()) => {
                self.status_message = format!("revealed project: {}", project.name);
            }
            Err(error) => self.status_message = format!("failed to reveal project: {error}"),
        }
        cx.notify();
    }

    pub(super) fn open_selected_project_in_application(
        &mut self,
        application_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to open".to_string();
            cx.notify();
            return;
        };

        let application_label = self
            .project_open_applications
            .iter()
            .find(|application| application.id == application_id)
            .map(|application| application.label.clone())
            .unwrap_or_else(|| application_id.clone());

        match self
            .runtime_service
            .project_open_in_application(project.path, application_id)
        {
            Ok(()) => {
                self.status_message = format!("opened {} in {application_label}", project.name);
            }
            Err(error) => {
                self.status_message = format!(
                    "failed to open {} in {application_label}: {error}",
                    project.name
                );
                self.project_open_applications = self.runtime_service.project_open_applications();
            }
        }
        cx.notify();
    }

    pub(super) fn open_project_folder_from_dialog(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let locale = locale_from_language_setting(&self.state.settings.language);
        match self
            .runtime_service
            .localized_open_dialog(LocalizedOpenDialogRequest {
                title: translate(&locale, "project.open_folder.title", "Open Folder"),
                message: translate(
                    &locale,
                    "project.open_folder.message",
                    "Choose a project folder to import.",
                ),
                prompt: translate(&locale, "project.open_folder.prompt", "Open"),
                default_path: None,
                filters: Vec::new(),
                directory: true,
                multiple: false,
                can_create_directories: Some(false),
            }) {
            Ok(Some(paths)) => {
                let Some(path) = paths.first().cloned() else {
                    self.status_message = "project import canceled".to_string();
                    cx.notify();
                    return;
                };
                let name = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or("Project")
                    .to_string();
                match self.runtime_service.create_or_select_project(&name, &path) {
                    Ok(project_id) => {
                        self.state = self.runtime_service.reload_state();
                        self.normalize_selected_ai_session();
                        self.normalize_selected_runtime_session();
                        self.normalize_selected_ssh_profile();
                        self.sync_project_list_store(cx);
                        self.status_message = format!("project added/selected: {project_id}");
                    }
                    Err(error) => {
                        self.status_message = format!("failed to add project: {error}");
                    }
                }
            }
            Ok(None) => {
                self.status_message = "project import canceled".to_string();
            }
            Err(error) => self.status_message = format!("failed to choose project folder: {error}"),
        }
        cx.notify();
    }

    pub(super) fn close_selected_project(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to close".to_string();
            cx.notify();
            return;
        };
        match self.runtime_service.close_project(&project.id) {
            Ok(next_project_id) => {
                self.state = self.runtime_service.reload_state();
                self.normalize_selected_ai_session();
                self.normalize_selected_runtime_session();
                self.normalize_selected_ssh_profile();
                self.sync_project_list_store(cx);
                self.status_message = match next_project_id {
                    Some(next_project_id) => {
                        format!("closed {}, selected {next_project_id}", project.name)
                    }
                    None => format!("closed {}, no projects left", project.name),
                };
            }
            Err(error) => self.status_message = format!("failed to close project: {error}"),
        }
        cx.notify();
    }

    pub(super) fn close_all_projects(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.state.projects.is_empty() {
            self.status_message = "no projects to close".to_string();
            cx.notify();
            return;
        }
        let closed = self.state.projects.len();
        match self.runtime_service.project_close_all() {
            Ok(_snapshot) => {
                self.state = self.runtime_service.reload_state();
                self.clear_file_selection();
                self.file_tree_expanded_dirs.clear();
                self.file_tree_children.clear();
                self.file_preview = "select a file to preview it".to_string();
                self.file_editable = false;
                self.file_dirty = false;
                self.selected_git_file = None;
                self.git_tree_children.clear();
                self.git_expanded_dirs.clear();
                self.git_diff_preview = "select a changed file to preview its diff".to_string();
                self.git_review_content = None;
                self.normalize_selected_ai_session();
                self.normalize_selected_runtime_session();
                self.normalize_selected_ssh_profile();
                self.sync_project_list_store(cx);
                self.status_message = format!(
                    "closed {closed} project{}",
                    if closed == 1 { "" } else { "s" }
                );
            }
            Err(error) => self.status_message = format!("failed to close projects: {error}"),
        }
        cx.notify();
    }

    pub(super) fn rename_selected_project(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_selected_project_editor_window(_window, cx);
    }

    pub(super) fn open_project_create_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let locale = locale_from_language_setting(&self.state.settings.language);
        if Self::activate_child_window(&mut self.project_editor_window, cx) {
            self.status_message = "project creator already opened".to_string();
            cx.notify();
            return;
        }

        let bounds = Bounds::centered(None, size(px(620.0), px(430.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(theme::codux_titlebar(translate(
                    &locale,
                    "project.create.title",
                    "Create Project",
                ))),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(520.0), px(390.0))),
                ..Default::default()
            },
            move |window, cx| {
                let app = CoduxApp::new_project_creator_window();
                theme::apply_component_theme(
                    &app.state.settings.theme,
                    &app.state.settings.theme_color,
                    Some(window),
                    cx,
                );
                let view = cx.new(|_| app);
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        self.status_message = match result {
            Ok(handle) => {
                self.project_editor_window = Some(handle.into());
                "project creator opened".to_string()
            }
            Err(error) => format!("failed to open project creator: {error}"),
        };
        cx.notify();
    }

    pub(super) fn open_selected_project_editor_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to edit".to_string();
            cx.notify();
            return;
        };
        let locale = locale_from_language_setting(&self.state.settings.language);

        if Self::activate_child_window(&mut self.project_editor_window, cx) {
            self.status_message = "project editor already opened".to_string();
            cx.notify();
            return;
        }

        let bounds = Bounds::centered(None, size(px(620.0), px(430.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(theme::codux_titlebar(translate(
                    &locale,
                    "project.edit.title",
                    "Edit Project",
                ))),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(520.0), px(390.0))),
                ..Default::default()
            },
            move |window, cx| {
                let app = CoduxApp::new_project_editor_window(project);
                theme::apply_component_theme(
                    &app.state.settings.theme,
                    &app.state.settings.theme_color,
                    Some(window),
                    cx,
                );
                let view = cx.new(|_| app);
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        self.status_message = match result {
            Ok(handle) => {
                self.project_editor_window = Some(handle.into());
                "project editor opened".to_string()
            }
            Err(error) => format!("failed to open project editor: {error}"),
        };
        cx.notify();
    }

    pub(super) fn set_project_editor_name(
        &mut self,
        value: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_editor_name = value;
        cx.notify();
    }

    pub(super) fn set_project_editor_path(
        &mut self,
        value: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_editor_path = value;
        cx.notify();
    }

    pub(super) fn set_project_editor_badge_symbol(
        &mut self,
        value: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_editor_badge_symbol = value;
        cx.notify();
    }

    pub(super) fn set_project_editor_badge_color(
        &mut self,
        value: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_editor_badge_color_hex = value;
        cx.notify();
    }

    pub(super) fn choose_project_editor_directory(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let locale = locale_from_language_setting(&self.state.settings.language);
        match self
            .runtime_service
            .localized_open_dialog(LocalizedOpenDialogRequest {
                title: translate(
                    &locale,
                    "project.editor.choose_directory.title",
                    "Choose Project Directory",
                ),
                message: translate(
                    &locale,
                    "project.editor.choose_directory.message",
                    "Select a folder for this project.",
                ),
                prompt: translate(&locale, "project.editor.choose_directory.prompt", "Choose"),
                default_path: Some(self.project_editor_path.clone()),
                filters: Vec::new(),
                directory: true,
                multiple: false,
                can_create_directories: Some(false),
            }) {
            Ok(Some(paths)) => {
                if let Some(path) = paths.first() {
                    self.project_editor_path = path.clone();
                    self.status_message = "project directory selected".to_string();
                } else {
                    self.status_message = "project directory selection canceled".to_string();
                }
            }
            Ok(None) => self.status_message = "project directory selection canceled".to_string(),
            Err(error) => {
                self.status_message = format!("failed to choose project directory: {error}")
            }
        }
        cx.notify();
    }

    pub(super) fn save_project_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.project_editor_name.trim().to_string();
        let path = self.project_editor_path.trim().to_string();
        if name.is_empty() || path.is_empty() {
            self.status_message = "project name and path are required".to_string();
            cx.notify();
            return;
        }

        if let Some(project_id) = self.project_editor_project_id.clone() {
            match self.runtime_service.project_update(ProjectUpdateRequest {
                project_id,
                name: name.clone(),
                path,
                badge_text: project_badge_text_from_name(&name),
                badge_symbol: self.project_editor_badge_symbol.clone(),
                badge_color_hex: Some(self.project_editor_badge_color_hex.clone()),
            }) {
                Ok(_snapshot) => {
                    self.state = self.runtime_service.reload_state();
                    self.sync_project_list_store(cx);
                    self.status_message = format!("project saved: {name}");
                    window.remove_window();
                }
                Err(error) => self.status_message = format!("failed to save project: {error}"),
            }
        } else {
            match self.runtime_service.project_create(ProjectCreateRequest {
                name: name.clone(),
                path,
                badge_text: project_badge_text_from_name(&name),
                badge_symbol: self.project_editor_badge_symbol.clone(),
                badge_color_hex: Some(self.project_editor_badge_color_hex.clone()),
            }) {
                Ok(_snapshot) => {
                    self.state = self.runtime_service.reload_state();
                    self.sync_project_list_store(cx);
                    self.status_message = format!("project created: {name}");
                    window.remove_window();
                }
                Err(error) => self.status_message = format!("failed to create project: {error}"),
            }
        }
        cx.notify();
    }

    pub(super) fn move_selected_project_up(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to move".to_string();
            cx.notify();
            return;
        };
        match self.runtime_service.move_project_up(&project.id) {
            Ok(()) => {
                self.state = self.runtime_service.reload_state();
                self.normalize_selected_ai_session();
                self.normalize_selected_runtime_session();
                self.normalize_selected_ssh_profile();
                self.sync_project_list_store(cx);
                self.status_message = format!("moved project up: {}", project.name);
            }
            Err(error) => self.status_message = format!("failed to move project: {error}"),
        }
        cx.notify();
    }

    pub(super) fn move_selected_project_down(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.state.selected_project.clone() else {
            self.status_message = "no selected project to move".to_string();
            cx.notify();
            return;
        };
        match self.runtime_service.move_project_down(&project.id) {
            Ok(()) => {
                self.state = self.runtime_service.reload_state();
                self.normalize_selected_ai_session();
                self.normalize_selected_runtime_session();
                self.normalize_selected_ssh_profile();
                self.sync_project_list_store(cx);
                self.status_message = format!("moved project down: {}", project.name);
            }
            Err(error) => self.status_message = format!("failed to move project: {error}"),
        }
        cx.notify();
    }
}
