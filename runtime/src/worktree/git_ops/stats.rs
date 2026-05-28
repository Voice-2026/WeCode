pub(super) fn worktree_line_stats(path: &str) -> (i64, i64) {
    let Ok(repo) = GitRepository::discover(path) else {
        return (0, 0);
    };
    let mut total = (0, 0);
    if let Ok(diff) = diff_for_line_stats(&repo, true) {
        merge_diff_line_stats(&mut total, &diff);
    }
    if let Ok(diff) = diff_for_line_stats(&repo, false) {
        merge_diff_line_stats(&mut total, &diff);
    }
    total
}

fn diff_for_line_stats(repo: &GitRepository, staged: bool) -> Result<git2::Diff<'_>, git2::Error> {
    let tree = head_tree(repo).ok();
    if staged {
        repo.diff_tree_to_index(tree.as_ref(), None, None)
    } else {
        repo.diff_index_to_workdir(None, None)
    }
}

fn merge_diff_line_stats(total: &mut (i64, i64), diff: &git2::Diff<'_>) {
    for index in 0..diff.deltas().len() {
        let (additions, deletions) = patch_line_stats(diff, index);
        total.0 += additions;
        total.1 += deletions;
    }
}

fn patch_line_stats(diff: &git2::Diff<'_>, index: usize) -> (i64, i64) {
    let Some(delta) = diff.get_delta(index) else {
        return (0, 0);
    };
    let Ok(Some(patch)) = git2::Patch::from_diff(diff, index) else {
        return match delta.status() {
            git2::Delta::Added => (1, 0),
            git2::Delta::Deleted => (0, 1),
            _ => (0, 0),
        };
    };
    let mut additions = 0;
    let mut deletions = 0;
    for hunk_index in 0..patch.num_hunks() {
        let Ok((_hunk, line_count)) = patch.hunk(hunk_index) else {
            continue;
        };
        for line_index in 0..line_count {
            let Ok(line) = patch.line_in_hunk(hunk_index, line_index) else {
                continue;
            };
            match line.origin() {
                '+' => additions += 1,
                '-' => deletions += 1,
                _ => {}
            }
        }
    }
    if additions == 0 && deletions == 0 {
        match delta.status() {
            git2::Delta::Added => additions = 1,
            git2::Delta::Deleted => deletions = 1,
            _ => {}
        }
    }
    (additions, deletions)
}

fn head_tree(repo: &GitRepository) -> Result<git2::Tree<'_>, git2::Error> {
    repo.head()?.peel_to_commit()?.tree()
}
