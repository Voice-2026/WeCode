impl GitService {
    pub fn stash_push(
        project_path: &str,
        message: Option<&str>,
        include_untracked: bool,
    ) -> Result<(), String> {
        let mut repo = open_git_repository(project_path)?;
        stash_push_git2(&mut repo, message, include_untracked)
    }

    pub fn stash_apply(project_path: &str, index: usize) -> Result<(), String> {
        let mut repo = open_git_repository(project_path)?;
        stash_apply_git2(&mut repo, index)
    }

    pub fn stash_pop(project_path: &str, index: usize) -> Result<(), String> {
        let mut repo = open_git_repository(project_path)?;
        stash_pop_git2(&mut repo, index)
    }

    pub fn stash_drop(project_path: &str, index: usize) -> Result<(), String> {
        let mut repo = open_git_repository(project_path)?;
        stash_drop_git2(&mut repo, index)
    }

    pub fn stash_drop_all(project_path: &str) -> Result<(), String> {
        let mut repo = open_git_repository(project_path)?;
        stash_drop_all_git2(&mut repo)
    }
}
