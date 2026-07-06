use super::*;

#[test]
fn ai_stats_watcher_tracks_one_project_per_device_and_clears_on_disconnect() {
    let support_dir = temp_support_dir("codux-remote-ai-stats-watcher");
    let runtime = RemoteHostRuntime::new(support_dir.clone());

    runtime.register_ai_stats_watcher("project-a", "device-1", "project-a");
    runtime.register_ai_stats_watcher("project-a", "device-2", "worktree-x");
    {
        let watchers = runtime.ai_stats_watchers.lock().unwrap();
        assert_eq!(watchers["project-a"].len(), 2);
        assert_eq!(watchers["project-a"]["device-2"], "worktree-x");
    }

    // Switching a device to another project drops its old-project entry.
    runtime.register_ai_stats_watcher("project-b", "device-1", "project-b");
    {
        let watchers = runtime.ai_stats_watchers.lock().unwrap();
        assert!(!watchers["project-a"].contains_key("device-1"));
        assert!(watchers["project-b"].contains_key("device-1"));
        assert!(watchers["project-a"].contains_key("device-2"));
    }

    // Disconnect drops the device from every project, pruning empties.
    runtime.clear_ai_stats_watcher_device("device-1");
    runtime.clear_ai_stats_watcher_device("device-2");
    assert!(runtime.ai_stats_watchers.lock().unwrap().is_empty());

    fs::remove_dir_all(support_dir).ok();
}
