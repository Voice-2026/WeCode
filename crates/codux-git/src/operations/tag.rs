fn create_tag_git2(repo: &GitRepository, name: &str, message: Option<&str>) -> Result<(), String> {
    let target = repo
        .head()
        .and_then(|head| head.peel(git2::ObjectType::Commit))
        .map_err(|error| error.message().to_string())?;
    match message.map(str::trim).filter(|value| !value.is_empty()) {
        // Annotated when a message is given, lightweight otherwise (VS Code semantics).
        Some(message) => {
            let signature = repo_signature(repo)?;
            repo.tag(name, &target, &signature, message, false)
                .map(|_| ())
                .map_err(|error| error.message().to_string())
        }
        None => repo
            .tag_lightweight(name, &target, false)
            .map(|_| ())
            .map_err(|error| error.message().to_string()),
    }
}

fn delete_tag_git2(repo: &GitRepository, name: &str) -> Result<(), String> {
    repo.tag_delete(name)
        .map_err(|error| error.message().to_string())
}
