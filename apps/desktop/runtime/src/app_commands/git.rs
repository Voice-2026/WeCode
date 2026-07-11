use super::*;

pub fn git_cancel(service: &RuntimeService, project_path: String) -> Result<(), String> {
    service.cancel_project_git(&project_path)
}
pub fn git_refresh_project(service: &RuntimeService, project_path: String) -> GitSummary {
    service.reload_project_git(&project_path)
}
pub fn git_status(project_path: String) -> GitStatusSnapshot {
    crate::git::git_status(project_path)
}
pub fn git_watch(
    service: &RuntimeService,
    project_path: String,
) -> Result<GitWatchRegistration, String> {
    service.git_watch(project_path)
}
pub fn git_unwatch(service: &RuntimeService, project_path: String) -> Result<(), String> {
    service.git_unwatch(project_path)
}
pub fn git_fetch(service: &RuntimeService, project_path: String) -> Result<GitSummary, String> {
    service.fetch_project_git(&project_path)
}
pub fn git_pull(service: &RuntimeService, project_path: String) -> Result<GitSummary, String> {
    service.pull_project_git(&project_path)
}
pub fn git_push(service: &RuntimeService, project_path: String) -> Result<GitSummary, String> {
    service.push_project_git(&project_path)
}
pub fn git_sync(service: &RuntimeService, project_path: String) -> Result<GitSummary, String> {
    service.sync_project_git(&project_path)
}
pub fn git_force_push(
    service: &RuntimeService,
    project_path: String,
) -> Result<GitSummary, String> {
    service.force_push_project_git(&project_path)
}
pub fn git_push_remote(
    service: &RuntimeService,
    request: GitPushRemoteRequest,
) -> Result<GitSummary, String> {
    service.push_project_git_remote(&request.project_path, &request.remote)
}
pub fn git_push_remote_branch(
    service: &RuntimeService,
    request: GitPushRemoteBranchRequest,
) -> Result<GitSummary, String> {
    service.push_project_git_remote_branch(
        &request.project_path,
        &request.remote_branch,
        request.local_branch.as_deref(),
    )
}
pub fn git_stage(request: GitPathsRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_stage(request)
}
pub fn git_unstage(request: GitPathsRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_unstage(request)
}
pub fn git_commit(request: GitCommitRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_commit(request)
}
pub fn git_commit_action(request: GitCommitActionRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_commit_action(request)
}
pub fn git_amend_last_commit_message(
    request: GitCommitRequest,
) -> Result<GitStatusSnapshot, String> {
    crate::git::git_amend_last_commit_message(request)
}
pub fn git_last_commit_message(project_path: String) -> Result<String, String> {
    crate::git::git_last_commit_message(project_path)
}
pub fn git_undo_last_commit(project_path: String) -> Result<GitStatusSnapshot, String> {
    crate::git::git_undo_last_commit(project_path)
}
pub fn git_head_commit_pushed(project_path: String) -> Result<bool, String> {
    crate::git::git_head_commit_pushed(project_path)
}
pub fn git_init(project_path: String) -> Result<GitStatusSnapshot, String> {
    crate::git::git_init(project_path)
}
pub fn git_clone(request: GitCloneRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_clone(request)
}
pub fn git_discard(request: GitPathsRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_discard(request)
}
pub fn git_branches(project_path: String) -> GitBranchesSnapshot {
    crate::git::git_branches(project_path)
}
pub fn git_checkout_branch(request: GitBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_checkout_branch(request)
}
pub fn git_create_branch(request: GitCreateBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_create_branch(request)
}
pub fn git_checkout_remote_branch(request: GitBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_checkout_remote_branch(request)
}
pub fn git_merge_branch(request: GitBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_merge_branch(request)
}
pub fn git_squash_merge_branch(request: GitBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_squash_merge_branch(request)
}
pub fn git_delete_branch(request: GitDeleteBranchRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_delete_branch(request)
}
pub fn git_checkout_commit(request: GitCommitRefRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_checkout_commit(request)
}
pub fn git_revert_commit(request: GitCommitRefRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_revert_commit(request)
}
pub fn git_restore_commit(request: GitRestoreCommitRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_restore_commit(request)
}
pub fn git_add_remote(request: GitRemoteRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_add_remote(request)
}
pub fn git_remove_remote(request: GitRemoteRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_remove_remote(request)
}
pub fn git_append_gitignore(request: GitPathsRequest) -> Result<GitStatusSnapshot, String> {
    crate::git::git_append_gitignore(request)
}
pub fn git_diff_file(request: GitDiffRequest) -> GitDiffSnapshot {
    crate::git::git_diff_file(request)
}
pub fn git_commit_message_context(project_path: String) -> GitCommitMessageContextSnapshot {
    crate::git::git_commit_message_context(project_path)
}
pub fn git_review_diff_file(request: GitReviewDiffRequest) -> GitDiffSnapshot {
    crate::git::git_review_diff_file(request)
}
pub fn git_review_file_content(request: GitReviewContentRequest) -> GitReviewContentSnapshot {
    crate::git::git_review_file_content(request)
}
pub fn git_review(project_path: String, base_branch: Option<String>) -> GitReviewSnapshot {
    crate::git::git_review(project_path, base_branch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn git_remote_commands_delegate_to_runtime_git2_layer() {
        let support_dir =
            std::env::temp_dir().join(format!("wecode-app-command-git-{}", Uuid::new_v4()));
        let project_dir = support_dir.join("project");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        let service = RuntimeService::new(support_dir.clone());
        let project_path = project_dir.display().to_string();

        git_cancel(&service, project_path.clone()).expect("cancel without token is ok");
        let snapshot = git_refresh_project(&service, project_path.clone());
        assert!(!snapshot.is_repository);
        assert!(!git_branches(project_path.clone()).is_repository);
        assert!(
            !git_diff_file(GitDiffRequest {
                project_path: project_path.clone(),
                path: "README.md".to_string(),
                staged: false,
            })
            .is_repository
        );
        assert!(!git_review(project_path.clone(), None).is_repository);
        assert!(!git_commit_message_context(project_path.clone()).is_repository);

        for result in [
            git_fetch(&service, project_path.clone()),
            git_pull(&service, project_path.clone()),
            git_push(&service, project_path.clone()),
            git_sync(&service, project_path.clone()),
            git_force_push(&service, project_path.clone()),
            git_push_remote(
                &service,
                GitPushRemoteRequest {
                    project_path: project_path.clone(),
                    remote: "origin".to_string(),
                },
            ),
            git_push_remote_branch(
                &service,
                GitPushRemoteBranchRequest {
                    project_path: project_path.clone(),
                    remote_branch: "origin/main".to_string(),
                    local_branch: Some("main".to_string()),
                },
            ),
        ] {
            assert!(result.is_err());
        }

        for result in [
            git_stage(GitPathsRequest {
                project_path: project_path.clone(),
                paths: vec!["README.md".to_string()],
            }),
            git_unstage(GitPathsRequest {
                project_path: project_path.clone(),
                paths: vec!["README.md".to_string()],
            }),
            git_commit(GitCommitRequest {
                project_path: project_path.clone(),
                message: "test".to_string(),
            }),
            git_commit_action(GitCommitActionRequest {
                project_path: project_path.clone(),
                message: "test".to_string(),
                action: "commit".to_string(),
            }),
            git_amend_last_commit_message(GitCommitRequest {
                project_path: project_path.clone(),
                message: "test".to_string(),
            }),
            git_undo_last_commit(project_path.clone()),
            git_clone(GitCloneRequest {
                project_path: project_path.clone(),
                remote_url: "https://example.invalid/repo.git".to_string(),
            }),
            git_discard(GitPathsRequest {
                project_path: project_path.clone(),
                paths: vec!["README.md".to_string()],
            }),
            git_checkout_branch(GitBranchRequest {
                project_path: project_path.clone(),
                branch: "main".to_string(),
            }),
            git_create_branch(GitCreateBranchRequest {
                project_path: project_path.clone(),
                branch: "feature".to_string(),
                from: None,
                checkout: false,
            }),
            git_checkout_remote_branch(GitBranchRequest {
                project_path: project_path.clone(),
                branch: "origin/main".to_string(),
            }),
            git_merge_branch(GitBranchRequest {
                project_path: project_path.clone(),
                branch: "main".to_string(),
            }),
            git_squash_merge_branch(GitBranchRequest {
                project_path: project_path.clone(),
                branch: "main".to_string(),
            }),
            git_delete_branch(GitDeleteBranchRequest {
                project_path: project_path.clone(),
                branch: "feature".to_string(),
                force: true,
            }),
            git_checkout_commit(GitCommitRefRequest {
                project_path: project_path.clone(),
                commit: "HEAD".to_string(),
            }),
            git_revert_commit(GitCommitRefRequest {
                project_path: project_path.clone(),
                commit: "HEAD".to_string(),
            }),
            git_restore_commit(GitRestoreCommitRequest {
                project_path: project_path.clone(),
                commit: "HEAD".to_string(),
                force_remote: false,
            }),
            git_add_remote(GitRemoteRequest {
                project_path: project_path.clone(),
                name: "origin".to_string(),
                url: Some("https://example.invalid/repo.git".to_string()),
            }),
            git_remove_remote(GitRemoteRequest {
                project_path: project_path.clone(),
                name: "origin".to_string(),
                url: None,
            }),
            git_append_gitignore(GitPathsRequest {
                project_path: project_path.clone(),
                paths: vec!["target/".to_string()],
            }),
        ] {
            assert!(result.is_err());
        }

        git_init(project_path.clone()).expect("init git repository");
        std::fs::write(project_dir.join("README.md"), "hello\n").expect("write readme");
        let diff = git_diff_file(GitDiffRequest {
            project_path: project_path.clone(),
            path: "README.md".to_string(),
            staged: false,
        });
        assert!(diff.is_repository);
        assert!(diff.diff.contains("Untracked file"));
        let staged = git_stage(GitPathsRequest {
            project_path: project_path.clone(),
            paths: vec!["README.md".to_string()],
        })
        .expect("stage readme");
        assert_eq!(staged.staged.len(), 1);
        let review = git_review(project_path.clone(), None);
        assert!(review.is_repository);
        let context = git_commit_message_context(project_path.clone());
        assert!(context.is_repository);
        let review_diff = git_review_diff_file(GitReviewDiffRequest {
            project_path: project_path.clone(),
            path: "README.md".to_string(),
            base_branch: None,
        });
        assert!(review_diff.is_repository);
        let content = git_review_file_content(GitReviewContentRequest {
            project_path: project_path.clone(),
            path: "README.md".to_string(),
            base_branch: None,
        });
        assert!(content.is_repository);
        let _ = git_head_commit_pushed(project_path).expect("head pushed status is available");

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
