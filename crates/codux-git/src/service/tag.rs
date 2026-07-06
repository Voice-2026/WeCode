impl GitService {
    pub fn create_tag(project_path: &str, name: &str, message: Option<&str>) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Tag name cannot be empty.".to_string());
        }
        let repo = open_git_repository(project_path)?;
        create_tag_git2(&repo, name, message)
    }

    pub fn delete_tag(project_path: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Tag name cannot be empty.".to_string());
        }
        let repo = open_git_repository(project_path)?;
        delete_tag_git2(&repo, name)
    }
}
