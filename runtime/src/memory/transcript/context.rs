pub(super) fn memory_project_context(
    projects: &[ProjectInfo],
    session: &AISessionSnapshot,
) -> Option<MemoryProjectContext> {
    projects
        .iter()
        .find(|project| project.id == session.project_id)
        .or_else(|| {
            session.project_path.as_ref().and_then(|path| {
                projects
                    .iter()
                    .find(|project| paths_equivalent(Some(project.path.as_str()), path))
            })
        })
        .map(|project| MemoryProjectContext {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            workspace_path: project.path.clone(),
        })
}

pub(super) fn memory_project_context_for_task(
    projects: &[ProjectInfo],
    task: &MemoryExtractionTask,
) -> Option<MemoryProjectContext> {
    projects
        .iter()
        .find(|project| project.id == task.project_id)
        .map(|project| MemoryProjectContext {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            workspace_path: task
                .workspace_path
                .as_deref()
                .and_then(|value| normalized_string(Some(value)))
                .unwrap_or_else(|| project.path.clone()),
        })
        .or_else(|| {
            task.workspace_path
                .as_deref()
                .and_then(|value| normalized_string(Some(value)))
                .map(|workspace_path| MemoryProjectContext {
                    project_id: task.project_id.clone(),
                    project_name: task.project_id.clone(),
                    workspace_path,
                })
        })
}
