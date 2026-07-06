impl RuntimeService {
    pub fn reload_project_files(
        &self,
        project_path: &str,
        directory_path: Option<&str>,
    ) -> Vec<FileEntry> {
        self.try_reload_project_files(project_path, directory_path)
            .unwrap_or_else(|error| {
                // Log the errno instead of a silent empty tree (EPERM/EMFILE under fd pressure or a sandbox/TCC denial).
                crate::runtime_trace::runtime_trace(
                    "files",
                    &format!("reload failed project={project_path}: {error}"),
                );
                Vec::new()
            })
    }

    pub fn try_reload_project_files(
        &self,
        project_path: &str,
        directory_path: Option<&str>,
    ) -> Result<Vec<FileEntry>, String> {
        // Remote-hosted projects list their files on the host over the controller.
        if let Some(device_id) = self.host_device_for_project_path(project_path) {
            return self.remote_project_files(&device_id, project_path, directory_path);
        }
        try_load_file_entries(project_path, directory_path)
    }

    pub fn watch_project_files(
        &self,
        project_path: String,
        on_change: impl Fn(FileChangeEvent) + Send + 'static,
    ) -> Result<FileWatchRegistration, String> {
        self.file_watch_manager.watch(project_path, on_change)
    }

    pub fn unwatch_project_files(&self, project_path: String) -> Result<(), String> {
        self.file_watch_manager.unwatch(project_path)
    }

    pub fn file_watch(&self, project_path: String) -> Result<FileWatchRegistration, String> {
        let events = Arc::clone(&self.file_watch_events);
        self.watch_project_files(project_path, move |event| {
            if let Ok(mut events) = events.lock() {
                events.push_back(event);
                while events.len() > 128 {
                    events.pop_front();
                }
            }
        })
    }

    pub fn file_unwatch(&self, project_path: String) -> Result<(), String> {
        self.unwatch_project_files(project_path)
    }

    pub fn drain_file_change_events(&self) -> Vec<FileChangeEvent> {
        self.file_watch_events
            .lock()
            .map(|mut events| events.drain(..).collect())
            .unwrap_or_default()
    }
    fn watch_active_project_files(
        &self,
        project_path: String,
    ) -> Result<FileWatchRegistration, String> {
        let (registration, previous) = self.mark_active_project_file_path(&project_path)?;
        if previous.as_deref() == Some(registration.project_path.as_str()) {
            return Ok(registration);
        }

        if let Some(previous) = previous {
            let _ = self.file_unwatch(previous);
        }

        let registration = self.file_watch(project_path)?;
        if let Ok(mut active) = self.active_file_watch_path.lock() {
            *active = Some(registration.project_path.clone());
        }
        Ok(registration)
    }

    pub(crate) fn mark_active_project_file_path(
        &self,
        project_path: &str,
    ) -> Result<(FileWatchRegistration, Option<String>), String> {
        let registration = self.file_watch_manager.registration(project_path)?;
        let mut active = self
            .active_file_watch_path
            .lock()
            .map_err(|_| "Active file watcher lock is poisoned.".to_string())?;
        let previous = active.clone();
        if previous.as_deref() != Some(registration.project_path.as_str()) {
            *active = Some(registration.project_path.clone());
        }
        Ok((registration, previous))
    }

    fn stop_active_project_files(&self) {
        let previous = self
            .active_file_watch_path
            .lock()
            .ok()
            .and_then(|mut active| active.take());
        if let Some(previous) = previous {
            let _ = self.file_unwatch(previous);
        }
    }
}
