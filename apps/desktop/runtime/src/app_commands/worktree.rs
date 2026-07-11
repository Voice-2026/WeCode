use super::*;

pub fn worktree_create(
    service: &RuntimeService,
    request: WorktreeCreateRequest,
) -> Result<WorktreeSnapshot, String> {
    service.create_worktree_from_request(request)
}
pub fn worktree_remove(
    service: &RuntimeService,
    request: WorktreeRemoveRequest,
) -> Result<WorktreeSnapshot, String> {
    service.remove_worktree_from_request(request)
}
pub fn worktree_merge(
    service: &RuntimeService,
    request: WorktreeMergeRequest,
) -> Result<WorktreeSnapshot, String> {
    service.merge_worktree_from_request(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn worktree_commands_delegate_to_runtime_validation() {
        let support_dir =
            std::env::temp_dir().join(format!("wecode-app-command-worktree-{}", Uuid::new_v4()));
        let project_dir = support_dir.join("project");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        let service = RuntimeService::new(support_dir.clone());
        let project_path = project_dir.display().to_string();

        let create_error = worktree_create(
            &service,
            WorktreeCreateRequest {
                project_id: "project".to_string(),
                project_path: project_path.clone(),
                base_branch: None,
                branch_name: "feature/demo".to_string(),
                task_title: Some("Demo".to_string()),
            },
        )
        .expect_err("non git create should fail");
        assert!(create_error.contains("Not a Git repository"));

        let remove_error = worktree_remove(
            &service,
            WorktreeRemoveRequest {
                project_id: "project".to_string(),
                project_path: project_path.clone(),
                worktree_path: project_path.clone(),
                remove_branch: false,
            },
        )
        .expect_err("non git remove should fail");
        assert!(remove_error.contains("Not a Git repository"));

        let merge_error = worktree_merge(
            &service,
            WorktreeMergeRequest {
                project_id: "project".to_string(),
                project_path,
                worktree_path: project_dir.display().to_string(),
                base_branch: None,
                remove_branch: Some(false),
            },
        )
        .expect_err("non git merge should fail");
        assert!(merge_error.contains("Not a Git repository"));

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
