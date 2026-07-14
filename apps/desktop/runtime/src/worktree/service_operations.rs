impl WorktreeService {
    pub fn create_from_request(
        &self,
        request: WorktreeCreateRequest,
    ) -> Result<WorktreeSnapshot, String> {
        self.create_from_request_impl(request, true)
    }

    /// Creates and registers a worktree without changing the project's current
    /// selection. Background automation uses this so a scheduled run never
    /// moves the user's visible workspace.
    pub fn create_unselected_from_request(
        &self,
        request: WorktreeCreateRequest,
    ) -> Result<WorktreeSnapshot, String> {
        self.create_from_request_impl(request, false)
    }

    fn create_from_request_impl(
        &self,
        request: WorktreeCreateRequest,
        select_created: bool,
    ) -> Result<WorktreeSnapshot, String> {
        let _guard = worktree_mutation_guard()?;
        // The git work (managed `.wecode/worktrees/<slug>` path + git2 branch
        // setup) lives in the shared `wecode_git::worktree` engine so the desktop
        // and the headless agent create worktrees identically. Only the
        // task/selection bookkeeping below is desktop-specific.
        let created = wecode_git::worktree::create_worktree(
            &request.project_path,
            &request.branch_name,
            request.base_branch.as_deref(),
        )?;
        let created_path = normalize_path(&created.display().to_string());
        let created_id = worktree_uuid(&request.project_id, &created_path);
        self.sync_from_git_unlocked(&request.project_id, &request.project_path)?;
        let task_title = request.task_title.as_deref().and_then(normalized_string);
        let base_branch = request.base_branch.as_deref().and_then(normalized_string);
        if task_title.is_some() || base_branch.is_some() {
            self.update_task_metadata(
                &created_id,
                task_title.as_deref(),
                base_branch.as_deref(),
            )?;
        }
        if select_created {
            self.select_worktree(&request.project_id, &created_id)?;
        }
        Ok(self.snapshot(request.project_id, request.project_path))
    }

    pub fn remove_from_request(
        &self,
        request: WorktreeRemoveRequest,
    ) -> Result<WorktreeSnapshot, String> {
        let _guard = worktree_mutation_guard()?;
        wecode_git::worktree::remove_worktree(
            &request.project_path,
            &request.worktree_path,
            request.remove_branch,
        )?;
        self.sync_from_git_unlocked(&request.project_id, &request.project_path)?;
        Ok(self.snapshot(request.project_id, request.project_path))
    }

    pub fn merge_from_request(
        &self,
        request: WorktreeMergeRequest,
    ) -> Result<WorktreeSnapshot, String> {
        let _guard = worktree_mutation_guard()?;
        wecode_git::worktree::merge_worktree(
            &request.project_path,
            &request.worktree_path,
            request.base_branch.as_deref(),
            request.remove_branch.unwrap_or(false),
        )?;
        self.sync_from_git_unlocked(&request.project_id, &request.project_path)?;
        Ok(self.snapshot(request.project_id, request.project_path))
    }

    pub fn create_worktree(
        &self,
        project_id: &str,
        project_path: &str,
    ) -> Result<WorktreeSummary, String> {
        let _guard = worktree_mutation_guard()?;
        let branch = format!("wecode-gpui-{}", now_seconds());
        let created = wecode_git::worktree::create_worktree(project_path, &branch, None)?;
        let created_path = normalize_path(&created.display().to_string());
        let created_id = worktree_uuid(project_id, &created_path);
        self.sync_from_git_unlocked(project_id, project_path)?;
        self.select_worktree(project_id, &created_id)?;
        Ok(self.summary(Some(project_id), Some(project_path)))
    }

    pub fn remove_worktree(
        &self,
        project_id: &str,
        project_path: &str,
        worktree_id: &str,
        remove_branch: bool,
    ) -> Result<WorktreeSummary, String> {
        let _guard = worktree_mutation_guard()?;
        let summary = self.summary(Some(project_id), Some(project_path));
        let worktree = summary
            .worktrees
            .iter()
            .find(|worktree| worktree.id == worktree_id)
            .ok_or_else(|| "Worktree not found.".to_string())?;
        if worktree.is_default || worktree.id == project_id {
            return Err("Default worktree cannot be removed.".to_string());
        }
        wecode_git::worktree::remove_worktree(project_path, &worktree.path, remove_branch)?;
        self.sync_from_git_unlocked(project_id, project_path)
    }

    pub fn merge_worktree(
        &self,
        project_id: &str,
        project_path: &str,
        worktree_id: &str,
    ) -> Result<WorktreeSummary, String> {
        let _guard = worktree_mutation_guard()?;
        let summary = self.summary(Some(project_id), Some(project_path));
        let worktree = summary
            .worktrees
            .iter()
            .find(|worktree| worktree.id == worktree_id)
            .ok_or_else(|| "Worktree not found.".to_string())?;
        if worktree.is_default || worktree.id == project_id {
            return Err("Default worktree cannot be merged into itself.".to_string());
        }
        // Prefer the task's recorded base branch; the shared engine falls back to
        // the repo's current branch when this is None.
        let base_branch = summary
            .tasks
            .iter()
            .find(|task| task.worktree_id == worktree_id)
            .map(|task| task.base_branch.trim().to_string())
            .filter(|branch| !branch.is_empty());
        wecode_git::worktree::merge_worktree(project_path, &worktree.path, base_branch.as_deref(), false)?;
        self.sync_from_git_unlocked(project_id, project_path)
    }
}
