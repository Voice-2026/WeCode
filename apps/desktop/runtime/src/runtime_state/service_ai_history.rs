impl RuntimeService {
    pub fn indexed_project_ai_history_summary(
        &self,
        project: AIHistoryProjectRequest,
    ) -> Result<AIHistoryProjectState, String> {
        if let Some(device_id) = self.host_device_for_project_path(&project.path) {
            return self
                .remote_controllers
                .controller_for(&device_id)?
                .ai_state(&project.id, &project.name, &project.path);
        }
        self.ai_history_indexer.project_summary(project)
    }

    pub fn refresh_indexed_project_ai_history(
        &self,
        project: AIHistoryProjectRequest,
    ) -> Result<(), String> {
        self.ai_history_indexer.refresh_project(project)
    }

    pub fn active_ai_history_index_count(&self) -> usize {
        self.ai_history_indexer.active_project_count()
    }

    pub fn indexed_project_ai_history_state(
        &self,
        project: AIHistoryProjectRequest,
    ) -> Result<AIHistoryProjectState, String> {
        if let Some(device_id) = self.host_device_for_project_path(&project.path) {
            return self
                .remote_controllers
                .controller_for(&device_id)?
                .ai_state(&project.id, &project.name, &project.path);
        }
        self.ai_history_indexer.project_state(project)
    }

    pub fn indexed_global_ai_history_summary(
        &self,
        projects: Vec<AIHistoryProjectRequest>,
    ) -> Result<AIGlobalHistorySnapshot, String> {
        self.ai_history_indexer.global_summary(projects)
    }

    pub fn indexed_global_ai_history_state(
        &self,
        projects: Vec<AIHistoryProjectRequest>,
    ) -> Result<Option<AIGlobalHistorySnapshot>, String> {
        self.ai_history_indexer.global_state(projects)
    }

    pub fn refresh_indexed_global_ai_history(
        &self,
        projects: Vec<AIHistoryProjectRequest>,
    ) -> Result<(), String> {
        self.ai_history_indexer.refresh_global(projects)
    }

    pub fn global_today_normalized_ai_tokens(&self) -> Result<i64, String> {
        global_today_normalized_tokens_at(self.support_dir.join("ai-usage.sqlite3"))
            .map_err(|error| error.to_string())
    }

    pub fn rename_indexed_ai_session(
        &self,
        project: AIHistoryProjectRequest,
        session_id: String,
        title: String,
    ) -> Result<AIHistoryProjectState, String> {
        self.ai_history_indexer
            .rename_session(project, session_id, title)
    }

    pub fn remove_indexed_ai_session(
        &self,
        project: AIHistoryProjectRequest,
        session_id: String,
    ) -> Result<AIHistoryProjectState, String> {
        self.ai_history_indexer.remove_session(project, session_id)
    }

    pub fn drain_ai_history_events(&self) -> AIHistoryDrainResult {
        let events = self.ai_history_indexer.drain_events();
        let should_refresh_pet = events.iter().any(|event| {
            matches!(
                event,
                AIHistoryEvent::Project { .. }
                    | AIHistoryEvent::ProjectState {
                        state: AIHistoryProjectState {
                            is_loading: false,
                            queued: false,
                            error: None,
                            snapshot: Some(_),
                            ..
                        },
                    }
            )
        });
        if !should_refresh_pet {
            return AIHistoryDrainResult {
                events,
                ..Default::default()
            };
        }
        match self.refresh_pet_from_indexed_history() {
            Ok(pet) => {
                let pet_snapshot = self.pet_snapshot().ok();
                AIHistoryDrainResult {
                    events,
                    pet: Some(pet),
                    pet_snapshot,
                    pet_error: None,
                }
            }
            Err(error) => AIHistoryDrainResult {
                events,
                pet: None,
                pet_snapshot: None,
                pet_error: Some(error),
            },
        }
    }
}
