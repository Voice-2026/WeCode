fn git2_stashes(repo: &GitRepository) -> Vec<GitStashSummary> {
    // stash_foreach needs &mut; open a private handle instead of changing every caller.
    let Ok(mut own) = GitRepository::open(repo.path()) else {
        return Vec::new();
    };
    let mut stashes = Vec::new();
    let _ = own.stash_foreach(|index, message, _oid| {
        stashes.push(GitStashSummary {
            index,
            message: message.to_string(),
        });
        true
    });
    stashes
}
