fn stash_push_git2(
    repo: &mut GitRepository,
    message: Option<&str>,
    include_untracked: bool,
) -> Result<(), String> {
    // Owned signature: stash_save2 needs &mut repo, repo_signature borrows it.
    let stasher = {
        let signature = repo_signature(repo)?;
        git2::Signature::now(
            signature.name().unwrap_or("codux"),
            signature.email().unwrap_or("codux@local"),
        )
        .map_err(|error| error.message().to_string())?
    };
    let flags = if include_untracked {
        git2::StashFlags::INCLUDE_UNTRACKED
    } else {
        git2::StashFlags::DEFAULT
    };
    repo.stash_save2(&stasher, message, Some(flags))
        .map(|_| ())
        .map_err(|error| error.message().to_string())
}

fn stash_apply_git2(repo: &mut GitRepository, index: usize) -> Result<(), String> {
    repo.stash_apply(index, None)
        .map_err(|error| error.message().to_string())
}

fn stash_pop_git2(repo: &mut GitRepository, index: usize) -> Result<(), String> {
    repo.stash_pop(index, None)
        .map_err(|error| error.message().to_string())
}

fn stash_drop_git2(repo: &mut GitRepository, index: usize) -> Result<(), String> {
    repo.stash_drop(index)
        .map_err(|error| error.message().to_string())
}

fn stash_drop_all_git2(repo: &mut GitRepository) -> Result<(), String> {
    // Always drop index 0: indices reshuffle after each drop.
    while !git2_stashes(repo).is_empty() {
        stash_drop_git2(repo, 0)?;
    }
    Ok(())
}
