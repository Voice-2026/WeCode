impl MemoryService {
    pub fn prepare_launch_artifacts_for_project(
        &self,
        project_id: &str,
        project_name: &str,
        workspace_path: &str,
    ) -> Option<MemoryLaunchArtifacts> {
        let project_profile = self
            .project_profile_for_launch(project_id, project_name, workspace_path)
            .or_else(|| self.current_project_profile(project_id).ok().flatten());
        let summary = self.summary(Some(project_id));
        if !summary.available && summary.recent_entries.is_empty() && project_profile.is_none() {
            return None;
        }

        let artifacts = launch_artifact_paths(project_id);
        let content = render_launch_memory_index(
            project_id,
            project_name,
            workspace_path,
            &summary,
            project_profile.as_ref(),
            None,
            None,
        );

        self.write_launch_artifacts(&artifacts, &content, &render_recent_memory(&summary))?;
        Some(artifacts)
    }

    /// Write the launch context files. The same content goes to the prompt file,
    /// MEMORY.md, and the per-agent AGENTS/CLAUDE/GEMINI files; memory-recent.md
    /// gets the recent block. Each file is only rewritten when its content
    /// actually changed, so the 8+ launch triggers don't churn the disk.
    pub(super) fn write_launch_artifacts(
        &self,
        artifacts: &MemoryLaunchArtifacts,
        content: &str,
        recent: &str,
    ) -> Option<()> {
        fs::create_dir_all(&artifacts.workspace_root).ok()?;
        write_if_changed(&artifacts.prompt_file, content);
        write_if_changed(&artifacts.index_file, content);
        write_if_changed(&artifacts.workspace_root.join("memory-recent.md"), recent);
        write_if_changed(&artifacts.workspace_root.join("AGENTS.md"), content);
        write_if_changed(&artifacts.workspace_root.join("CLAUDE.md"), content);
        write_if_changed(&artifacts.workspace_root.join("GEMINI.md"), content);
        Some(())
    }
}

fn write_if_changed(path: &std::path::Path, content: &str) {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return;
    }
    let _ = fs::write(path, content);
}
