fn git2_tags(repo: &GitRepository) -> Vec<String> {
    repo.tag_names(None)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.ok().flatten().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
