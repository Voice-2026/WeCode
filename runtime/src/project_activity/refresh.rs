impl ProjectActivityCoordinator {
    pub fn refresh_project_now(&self, project: ProjectSummary) {
        self.mark_project_summary(&project);
        self.refresh_git_once(&project);
        if self.mark_ai_activation(&project.id) {
            self.refresh_ai_once(project);
        }
    }

    pub fn refresh_git_once(&self, project: &ProjectSummary) {
        self.mark_project_summary(project);
        let mut tracked_project = TrackedProject::from(project.clone());
        if let Ok(mut guard) = self.projects.lock() {
            if let Some(tracked) = guard.get_mut(&project.id) {
                tracked.last_git_refresh = Some(Instant::now());
                tracked_project = tracked.clone();
            }
        }
        self.git_jobs.submit(GitJob::Refresh {
            project: tracked_project,
        });
    }

    pub fn refresh_git_sidecars_by_path(&self, project: ProjectSummary) {
        self.mark_project_summary(&project);
        self.git_jobs.submit(GitJob::Worktree {
            support_dir: self.support_dir.clone(),
            project: project.clone(),
        });
        self.git_jobs.submit(GitJob::Review {
            project: TrackedProject::from(project),
        });
    }

    pub fn refresh_git_changed(
        &self,
        project_store: &ProjectStore,
        project_path: String,
        repository_path: String,
        changed_paths: Vec<String>,
    ) {
        let Some(project) = project_store.workspace_summary_by_path(&project_path) else {
            return;
        };
        self.mark_project_summary(&project);
        if let Ok(mut guard) = self.projects.lock() {
            if let Some(tracked) = guard.get_mut(&project.id) {
                tracked.last_git_refresh = Some(Instant::now());
            }
        }
        if let Ok(mut events) = self.events.lock() {
            events.push_back(ProjectActivityEvent::GitChanged {
                project_path,
                repository_path,
                changed_paths,
            });
            while events.len() > 128 {
                events.pop_front();
            }
        }
        self.git_jobs.submit(GitJob::Worktree {
            support_dir: self.support_dir.clone(),
            project: project.clone(),
        });
        self.git_jobs.submit(GitJob::Refresh {
            project: TrackedProject::from(project.clone()),
        });
        self.git_jobs.submit(GitJob::Review {
            project: TrackedProject::from(project),
        });
    }

    pub fn refresh_ai_once(&self, project: ProjectSummary) {
        self.mark_project_summary(&project);
        let _ = self.mark_ai_activation(&project.id);
        if let Ok(mut guard) = self.projects.lock() {
            if let Some(tracked) = guard.get_mut(&project.id) {
                tracked.last_ai_refresh = Some(Instant::now());
            }
        }
        let ai_history = self.ai_history.clone();
        thread::spawn(move || {
            let request: AIHistoryProjectRequest = project.clone().into();
            let _ = ai_history.refresh_project(request);
        });
    }

    pub fn run_tick(&self, settings: &SettingsSummary) {
        let git_interval =
            configured_interval_seconds(&settings.git_refresh, MIN_GIT_REFRESH_SECONDS);
        let ai_foreground_interval =
            configured_interval_seconds(&settings.ai_refresh, MIN_AI_REFRESH_SECONDS);
        let ai_background_interval =
            configured_interval_seconds(&settings.ai_background_refresh, MIN_AI_REFRESH_SECONDS);

        if let Some(interval) = git_interval {
            let background_interval = interval
                .checked_mul(4)
                .unwrap_or_else(|| Duration::from_secs(MIN_GIT_REFRESH_SECONDS * 4))
                .max(Duration::from_secs(MIN_GIT_REFRESH_SECONDS * 4));
            for project in self.projects_due_for_git(interval, background_interval) {
                self.git_jobs.submit(GitJob::Refresh { project });
            }
        }

        if let Some(foreground_interval) = ai_foreground_interval.or(ai_background_interval) {
            let background_interval = ai_background_interval
                .unwrap_or_else(|| {
                    foreground_interval
                        .checked_mul(4)
                        .unwrap_or_else(|| Duration::from_secs(MIN_AI_REFRESH_SECONDS * 4))
                })
                .max(foreground_interval);
            for project in self.projects_due_for_ai(foreground_interval, background_interval) {
                self.refresh_ai_once(ProjectSummary::from(project));
            }
        }
    }

    fn mark_ai_activation(&self, project_id: &str) -> bool {
        self.activated_ai_projects
            .lock()
            .map(|mut activated| activated.insert(project_id.to_string()))
            .unwrap_or(false)
    }

    fn projects_due_for_git(
        &self,
        foreground_interval: Duration,
        background_interval: Duration,
    ) -> Vec<TrackedProject> {
        let active_project_id = self.active_project_id.lock().ok().and_then(|id| id.clone());
        let is_foreground = self.main_window_visible.load(Ordering::Relaxed)
            || self.main_window_focused.load(Ordering::Relaxed);
        projects_due_for_git_interval(
            &self.projects,
            active_project_id.as_deref(),
            is_foreground,
            foreground_interval,
            background_interval,
            MAX_BACKGROUND_GIT_REFRESH_PER_TICK,
        )
    }

    fn projects_due_for_ai(
        &self,
        foreground_interval: Duration,
        background_interval: Duration,
    ) -> Vec<TrackedProject> {
        let active_project_id = self.active_project_id.lock().ok().and_then(|id| id.clone());
        let is_foreground = self.main_window_visible.load(Ordering::Relaxed)
            || self.main_window_focused.load(Ordering::Relaxed);
        projects_due_by_interval_mut(
            &self.projects,
            |project| {
                if is_foreground && active_project_id.as_deref() == Some(project.id.as_str()) {
                    foreground_interval
                } else {
                    background_interval
                }
            },
            |project| &mut project.last_ai_refresh,
        )
    }
}
