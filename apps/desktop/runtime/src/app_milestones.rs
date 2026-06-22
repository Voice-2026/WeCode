//! Tiny persisted record of one-time app lifecycle milestones (currently just
//! the first-launch time and whether the "star us on GitHub" nudge has been
//! shown). Kept out of user-facing settings — it is internal bookkeeping — and
//! stored as a small JSON file under the support directory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMilestones {
    /// Unix seconds of the first launch we observed. Seeded on first read.
    #[serde(default)]
    pub first_launch_at: i64,
    /// Whether the GitHub-star nudge has already been shown (manually or auto).
    #[serde(default)]
    pub star_prompt_shown: bool,
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn milestones_path(support_dir: &Path) -> PathBuf {
    support_dir.join("app-milestones.json")
}

fn save(support_dir: &Path, milestones: &AppMilestones) {
    let path = milestones_path(support_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(milestones) {
        let _ = std::fs::write(path, bytes);
    }
}

/// Load the milestones, seeding `first_launch_at` to now (and persisting) the
/// first time the file is missing or unreadable.
pub fn load_or_seed(support_dir: &Path) -> AppMilestones {
    if let Ok(bytes) = std::fs::read(milestones_path(support_dir))
        && let Ok(mut milestones) = serde_json::from_slice::<AppMilestones>(&bytes)
    {
        if milestones.first_launch_at <= 0 {
            milestones.first_launch_at = now_seconds();
            save(support_dir, &milestones);
        }
        return milestones;
    }
    let milestones = AppMilestones {
        first_launch_at: now_seconds(),
        star_prompt_shown: false,
    };
    save(support_dir, &milestones);
    milestones
}

/// Mark the GitHub-star nudge as shown so the automatic one-time popup will not
/// fire again. Returns the updated record.
pub fn mark_star_prompt_shown(support_dir: &Path) -> AppMilestones {
    let mut milestones = load_or_seed(support_dir);
    milestones.star_prompt_shown = true;
    save(support_dir, &milestones);
    milestones
}
