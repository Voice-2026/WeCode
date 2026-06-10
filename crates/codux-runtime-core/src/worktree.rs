use crate::git::GitBranchSummary;
use serde_json::{Value, json};

pub fn worktree_base_branches(branch: &str, branches: &[GitBranchSummary]) -> Vec<String> {
    let mut values = Vec::new();
    push_unique_branch(&mut values, branch);
    for branch in branches {
        push_unique_branch(&mut values, branch.name.as_str());
    }
    values
}

pub fn default_worktree_base_branch(branch: &str, branches: &[GitBranchSummary]) -> String {
    branches
        .iter()
        .find(|branch| branch.is_current)
        .or_else(|| branches.first())
        .map(|branch| branch.name.clone())
        .filter(|branch| !branch.trim().is_empty())
        .unwrap_or_else(|| branch.to_string())
}

pub fn worktree_summary_payload(
    project_id: impl Into<String>,
    selected_worktree_id: Option<String>,
    worktrees: Value,
    tasks: Value,
    available: bool,
    base_branches: Vec<String>,
    default_base_branch: String,
    error: Option<String>,
) -> Value {
    json!({
        "projectId": project_id.into(),
        "selectedWorktreeId": selected_worktree_id,
        "worktrees": worktrees,
        "tasks": tasks,
        "available": available,
        "baseBranches": base_branches,
        "defaultBaseBranch": default_base_branch,
        "error": error,
    })
}

pub fn worktree_update_payload(
    project_id: impl Into<String>,
    selected_worktree_id: impl Into<String>,
    worktrees: Value,
    tasks: Value,
    base_branches: Vec<String>,
    default_base_branch: String,
    error: Option<String>,
) -> Value {
    json!({
        "projectId": project_id.into(),
        "selectedWorktreeId": selected_worktree_id.into(),
        "worktrees": worktrees,
        "tasks": tasks,
        "baseBranches": base_branches,
        "defaultBaseBranch": default_base_branch,
        "error": error,
    })
}

fn push_unique_branch(values: &mut Vec<String>, value: &str) {
    let branch = value.trim();
    if branch.is_empty() || values.iter().any(|item| item == branch) {
        return;
    }
    values.push(branch.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_base_branches_are_unique_and_current_first() {
        let branches = vec![
            GitBranchSummary {
                name: "main".to_string(),
                is_current: true,
            },
            GitBranchSummary {
                name: "feature".to_string(),
                is_current: false,
            },
        ];

        assert_eq!(
            worktree_base_branches("main", &branches),
            vec!["main".to_string(), "feature".to_string()]
        );
        assert_eq!(default_worktree_base_branch("fallback", &branches), "main");
    }
}
