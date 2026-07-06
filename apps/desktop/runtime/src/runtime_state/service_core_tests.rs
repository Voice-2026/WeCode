#[cfg(test)]
mod app_runtime_ready_tests {
    use super::*;
    use crate::terminal_layout::{
        TerminalPaneSummary, TerminalTabSummary, terminal_layout_storage_key,
    };
    use crate::terminal_pty::TerminalLaunchContext;
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        thread,
        time::{Duration, Instant},
    };

    fn wait_for_active_watch_path(service: &RuntimeService, expected: &str) {
        for _ in 0..50 {
            let current = service
                .active_file_watch_path
                .lock()
                .expect("active file watch")
                .clone();
            if current.as_deref() == Some(expected) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            service
                .active_file_watch_path
                .lock()
                .expect("active file watch")
                .as_deref(),
            Some(expected)
        );
    }

    fn wait_for_ai_history_loading_event(
        service: &RuntimeService,
        project_id: &str,
        project_path: &str,
    ) {
        // Generous cap: the history indexer competes with the whole suite for
        // scheduling; passing runs return on the first matching drain.
        for _ in 0..800 {
            let result = service.drain_ai_history_events();
            if result.events.iter().any(|event| {
                matches!(
                    event,
                    AIHistoryEvent::ProjectState { state }
                        if state.project_id == project_id
                            && state.project_path == project_path
                            && state.is_loading
                )
            }) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let result = service.drain_ai_history_events();
        assert!(
            result.events.iter().any(|event| {
                matches!(
                    event,
                    AIHistoryEvent::ProjectState { state }
                        if state.project_id == project_id
                            && state.project_path == project_path
                            && state.is_loading
                )
            }),
            "expected AI history loading event for {project_id} at {project_path}, got {:?}",
            result.events
        );
    }

    #[test]
    fn launch_artifacts_include_tool_context_when_memory_is_disabled() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-runtime-tool-context-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(
            support_dir.join("settings.json"),
            serde_json::json!({
                "ai": {
                    "globalPrompt": "",
                    "memory": {
                        "enabled": false,
                        "automaticInjectionEnabled": false
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            support_dir.join("ssh_profiles.json"),
            serde_json::json!([{
                "id": "profile-1",
                "name": "Production",
                "host": "example.com",
                "port": 22,
                "username": "root",
                "credentialKind": "password",
                "privateKeyPath": "",
                "password": "secret-password",
                "keyPassphrase": "secret-passphrase",
                "updatedAt": 1
            }])
            .to_string(),
        )
        .unwrap();
        fs::write(
            support_dir.join("db_profiles.json"),
            serde_json::json!([{
                "id": "db-1",
                "projectId": "project-a",
                "name": "Production DB",
                "engine": "postgres",
                "host": "db.example.com",
                "port": 5432,
                "database": "app",
                "username": "app_user",
                "password": "db-secret",
                "sslMode": "require",
                "readOnly": true,
                "updatedAt": 1
            }])
            .to_string(),
        )
        .unwrap();

        let service = RuntimeService::new(support_dir.clone());
        let artifacts = service
            .prepare_memory_launch_artifacts("project-a", "Project A", "/workspace/project-a")
            .expect("tool launch context should create artifacts");
        let agents = fs::read_to_string(artifacts.workspace_root.join("AGENTS.md")).unwrap();

        assert!(agents.starts_with("# Codux Environment Directive"));
        assert!(agents.contains("codux-ssh list"));
        assert!(agents.contains("codux-ssh <profile-id> -- '<remote-command>'"));
        assert!(agents.contains("Do not grep the repository"));
        assert!(!agents.contains("profile-1"));
        assert!(!agents.contains("root@example.com:22"));
        assert!(!agents.contains("secret-password"));
        assert!(!agents.contains("secret-passphrase"));
        assert!(agents.contains("codux-db list"));
        assert!(agents.contains("codux-db <profile-id> -- '<SQL>'"));
        assert!(agents.contains("cast them to text"));
        assert!(!agents.contains("db-1"));
        assert!(!agents.contains("db.example.com:5432 / app"));
        assert!(!agents.contains("db-secret"));
        assert!(!agents.contains("app_user"));
        assert!(!agents.contains("project active entry"));

        fs::remove_dir_all(support_dir).ok();
        fs::remove_dir_all(artifacts.workspace_root).ok();
    }

    #[test]
    fn launch_artifacts_include_environment_directive_without_profiles() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-runtime-environment-directive-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(
            support_dir.join("settings.json"),
            serde_json::json!({
                "ai": {
                    "globalPrompt": "",
                    "memory": {
                        "enabled": false,
                        "automaticInjectionEnabled": false
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let service = RuntimeService::new(support_dir.clone());
        let artifacts = service
            .prepare_memory_launch_artifacts("project-a", "Project A", "/workspace/project-a")
            .expect("environment directive should create artifacts");
        let agents = fs::read_to_string(artifacts.workspace_root.join("AGENTS.md")).unwrap();

        assert!(agents.starts_with("# Codux Environment Directive"));
        assert!(agents.contains("codux-ssh list"));
        assert!(agents.contains("codux-db list"));
        assert!(agents.contains("# Codux Memory"));
        assert!(!agents.contains("project active entry"));

        fs::remove_dir_all(support_dir).ok();
        fs::remove_dir_all(artifacts.workspace_root).ok();
    }

    fn assert_tracked_project_has_git_refresh(service: &RuntimeService, project_id: &str) {
        let activity = service.project_activity_snapshot();
        let tracked = activity
            .tracked_projects
            .iter()
            .find(|project| project.id == project_id)
            .unwrap_or_else(|| panic!("missing tracked project {project_id}: {activity:?}"));
        assert!(
            tracked.has_git_refresh,
            "expected git refresh marker for {project_id}: {activity:?}"
        );
        assert!(
            activity.activated_git_count > 0,
            "expected activated git count after project activation: {activity:?}"
        );
    }

    #[cfg(unix)]
    fn recv_until_contains(
        rx: &flume::Receiver<Vec<u8>>,
        needle: &str,
        timeout: Duration,
    ) -> String {
        let deadline = Instant::now() + timeout;
        let mut output = String::new();
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(remaining.min(Duration::from_millis(50))) {
                Ok(bytes) => {
                    output.push_str(&String::from_utf8_lossy(&bytes));
                    if output.contains(needle) {
                        return output;
                    }
                }
                Err(flume::RecvTimeoutError::Timeout) => {}
                Err(flume::RecvTimeoutError::Disconnected) => break,
            }
        }
        output
    }

    #[test]
    fn app_runtime_ready_marks_selected_project_active_and_returns_startup_snapshots() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-runtime-ready-{}", uuid::Uuid::new_v4()));
        let project_dir = support_dir.join("project");
        fs::create_dir_all(&project_dir).expect("create project dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Runtime Ready",
                        "path": project_dir.to_string_lossy()
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write state");
        let service = RuntimeService::new(PathBuf::from(&support_dir));
        let snapshot = service.app_runtime_ready(true, true);

        assert_eq!(
            snapshot.projects.selected_project_id.as_deref(),
            Some("project-1")
        );
        assert_eq!(
            snapshot.project_activity.active_project_id.as_deref(),
            Some("project-1")
        );
        assert!(snapshot.project_activity.visible);
        assert!(snapshot.project_activity.focused);
        assert!(snapshot.window_state.project_activity.visible);
        assert!(snapshot.window_state.project_activity.focused);
        assert_eq!(snapshot.window_state.attention_count, 0);
        assert_eq!(snapshot.window_state.dock_badge_count, None);
        assert_eq!(snapshot.terminal_layouts.layouts.len(), 0);
        assert_eq!(snapshot.ai_runtime_state.sessions.len(), 0);
        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn project_update_marks_updated_project_active_and_rewatches_files() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-project-update-{}", uuid::Uuid::new_v4()));
        let old_project_dir = support_dir.join("old-project");
        let new_project_dir = support_dir.join("new-project");
        fs::create_dir_all(&old_project_dir).expect("create old project dir");
        fs::create_dir_all(&new_project_dir).expect("create new project dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Old Project",
                        "path": old_project_dir.to_string_lossy()
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write state");
        fs::write(
            support_dir.join("pet-state.json"),
            serde_json::to_vec(&crate::pet::PetSnapshot::default()).expect("encode empty pet"),
        )
        .expect("write pet state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        service.app_runtime_ready(true, true);

        service
            .update_project(
                "project-1",
                "New Project",
                new_project_dir.to_str().unwrap(),
            )
            .expect("update project");

        let expected_watch_path = new_project_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        wait_for_active_watch_path(&service, &expected_watch_path);
        assert_eq!(
            service
                .project_activity_snapshot()
                .active_project_id
                .as_deref(),
            Some("project-1")
        );

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn project_select_worktree_marks_root_project_active_and_watches_worktree_files() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-project-select-worktree-{}",
            uuid::Uuid::new_v4()
        ));
        let project_dir = support_dir.join("project");
        let worktree_dir = support_dir.join("worktree");
        fs::create_dir_all(&project_dir).expect("create project dir");
        fs::create_dir_all(&worktree_dir).expect("create worktree dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Project",
                        "path": project_dir.to_string_lossy()
                    }
                ],
                "worktrees": [
                    {
                        "id": "worktree-1",
                        "projectId": "project-1",
                        "name": "Feature",
                        "branch": "feature",
                        "path": worktree_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        service
            .project_select_worktree(crate::project_store::ProjectSelectWorktreeRequest {
                project_id: "project-1".to_string(),
                worktree_id: "worktree-1".to_string(),
            })
            .expect("select worktree");

        let expected_watch_path = worktree_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        wait_for_active_watch_path(&service, &expected_watch_path);
        assert_eq!(
            service
                .project_activity_snapshot()
                .active_project_id
                .as_deref(),
            Some("project-1")
        );
        assert_eq!(
            service
                .project_list()
                .selected_worktree_id_by_project
                .get("project-1")
                .map(String::as_str),
            Some("worktree-1")
        );

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn project_and_worktree_switch_loads_terminal_layout_for_selected_worktree() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-project-worktree-terminal-layout-{}",
            uuid::Uuid::new_v4()
        ));
        let project_dir = support_dir.join("project");
        let worktree_a_dir = support_dir.join("worktree-a");
        let worktree_b_dir = support_dir.join("worktree-b");
        fs::create_dir_all(&project_dir).expect("create project dir");
        fs::create_dir_all(&worktree_a_dir).expect("create worktree a dir");
        fs::create_dir_all(&worktree_b_dir).expect("create worktree b dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Project",
                        "path": project_dir.to_string_lossy()
                    }
                ],
                "worktrees": [
                    {
                        "id": "worktree-a",
                        "projectId": "project-1",
                        "name": "Task A",
                        "branch": "task-a",
                        "path": worktree_a_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    },
                    {
                        "id": "worktree-b",
                        "projectId": "project-1",
                        "name": "Task B",
                        "branch": "task-b",
                        "path": worktree_b_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    }
                ],
                "worktreeTasks": [
                    {
                        "worktreeId": "worktree-a",
                        "title": "Task A",
                        "baseBranch": "main",
                        "baseCommit": null,
                        "status": "active",
                        "createdAt": 1,
                        "updatedAt": 1,
                        "startedAt": null,
                        "completedAt": null
                    },
                    {
                        "worktreeId": "worktree-b",
                        "title": "Task B",
                        "baseBranch": "main",
                        "baseCommit": null,
                        "status": "active",
                        "createdAt": 1,
                        "updatedAt": 1,
                        "startedAt": null,
                        "completedAt": null
                    }
                ],
                "selectedProjectId": "project-1",
                "selectedWorktreeIdByProject": {
                    "project-1": "worktree-a"
                }
            })
            .to_string(),
        )
        .expect("write state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        service
            .save_terminal_layout(
                &crate::terminal_layout::terminal_layout_storage_key("project-1", "worktree-a"),
                Vec::new(),
                "terminal-a".to_string(),
                vec![TerminalPaneSummary {
                    title: "Task A".to_string(),
                    terminal_id: "terminal-a".to_string(),
                }],
                vec![1.0],
                0.18,
            )
            .expect("save worktree a terminal layout");
        service
            .save_terminal_layout(
                &crate::terminal_layout::terminal_layout_storage_key("project-1", "worktree-b"),
                Vec::new(),
                "terminal-b".to_string(),
                vec![TerminalPaneSummary {
                    title: "Task B".to_string(),
                    terminal_id: "terminal-b".to_string(),
                }],
                vec![1.0],
                0.52,
            )
            .expect("save worktree b terminal layout");

        let state = RuntimeState::load_from_support_dir(support_dir.clone());
        assert_eq!(
            state.worktrees.selected_worktree_id.as_deref(),
            Some("worktree-a")
        );
        assert_eq!(state.terminal_layout.active_terminal_id, "");
        assert_eq!(state.terminal_layout.top_panes[0].terminal_id, "terminal-a");
        assert_eq!(state.terminal_layout.bottom_ratio, 0.18);

        service
            .project_select_worktree(crate::project_store::ProjectSelectWorktreeRequest {
                project_id: "project-1".to_string(),
                worktree_id: "worktree-b".to_string(),
            })
            .expect("select worktree b");
        let state = RuntimeState::load_from_support_dir(support_dir.clone());
        assert_eq!(
            state.worktrees.selected_worktree_id.as_deref(),
            Some("worktree-b")
        );
        assert_eq!(state.terminal_layout.active_terminal_id, "");
        assert_eq!(state.terminal_layout.top_panes[0].terminal_id, "terminal-b");
        assert_eq!(state.terminal_layout.bottom_ratio, 0.52);

        service
            .project_select_worktree(crate::project_store::ProjectSelectWorktreeRequest {
                project_id: "project-1".to_string(),
                worktree_id: "worktree-a".to_string(),
            })
            .expect("select worktree a");
        let state = RuntimeState::load_from_support_dir(support_dir.clone());
        assert_eq!(
            state.worktrees.selected_worktree_id.as_deref(),
            Some("worktree-a")
        );
        assert_eq!(state.terminal_layout.active_terminal_id, "");
        assert_eq!(state.terminal_layout.top_panes[0].terminal_id, "terminal-a");
        assert_eq!(state.terminal_layout.bottom_ratio, 0.18);

        let _ = fs::remove_dir_all(support_dir);
    }

    #[cfg(unix)]
    #[test]
    fn project_and_worktree_switch_runs_runtime_activation_layout_pty_ai_and_git_flow() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-runtime-switch-full-flow-{}",
            uuid::Uuid::new_v4()
        ));
        let project_a_dir = support_dir.join("project-a");
        let project_b_dir = support_dir.join("project-b");
        let worktree_a_dir = support_dir.join("worktree-a");
        let worktree_b_dir = support_dir.join("worktree-b");
        fs::create_dir_all(&project_a_dir).expect("create project a dir");
        fs::create_dir_all(&project_b_dir).expect("create project b dir");
        fs::create_dir_all(&worktree_a_dir).expect("create worktree a dir");
        fs::create_dir_all(&worktree_b_dir).expect("create worktree b dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-a",
                        "name": "Project A",
                        "path": project_a_dir.to_string_lossy()
                    },
                    {
                        "id": "project-b",
                        "name": "Project B",
                        "path": project_b_dir.to_string_lossy()
                    }
                ],
                "worktrees": [
                    {
                        "id": "worktree-a",
                        "projectId": "project-a",
                        "name": "Task A",
                        "branch": "task-a",
                        "path": worktree_a_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    },
                    {
                        "id": "worktree-b",
                        "projectId": "project-a",
                        "name": "Task B",
                        "branch": "task-b",
                        "path": worktree_b_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    }
                ],
                "worktreeTasks": [
                    {
                        "worktreeId": "worktree-a",
                        "title": "Task A",
                        "baseBranch": "main",
                        "baseCommit": null,
                        "status": "active",
                        "createdAt": 1,
                        "updatedAt": 1,
                        "startedAt": null,
                        "completedAt": null
                    },
                    {
                        "worktreeId": "worktree-b",
                        "title": "Task B",
                        "baseBranch": "main",
                        "baseCommit": null,
                        "status": "active",
                        "createdAt": 1,
                        "updatedAt": 1,
                        "startedAt": null,
                        "completedAt": null
                    }
                ],
                "selectedProjectId": "project-a",
                "selectedWorktreeIdByProject": {
                    "project-a": "worktree-a"
                }
            })
            .to_string(),
        )
        .expect("write state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        let layout_a_key = terminal_layout_storage_key("project-a", "worktree-a");
        let layout_b_key = terminal_layout_storage_key("project-a", "worktree-b");
        let terminal_a_top = format!("terminal-a-top-{}", uuid::Uuid::new_v4());
        let terminal_a_tab = format!("terminal-a-tab-{}", uuid::Uuid::new_v4());
        let terminal_b_top = format!("terminal-b-top-{}", uuid::Uuid::new_v4());
        let terminal_project_b = format!("terminal-project-b-{}", uuid::Uuid::new_v4());
        service
            .save_terminal_layout(
                &layout_a_key,
                vec![TerminalTabSummary {
                    label: "Task A Tab".to_string(),
                    terminal_id: terminal_a_tab.clone(),
                }],
                terminal_a_top.clone(),
                vec![TerminalPaneSummary {
                    title: "Task A Top".to_string(),
                    terminal_id: terminal_a_top.clone(),
                }],
                vec![1.0],
                0.24,
            )
            .expect("save task a layout");
        service
            .save_terminal_layout(
                &layout_b_key,
                Vec::new(),
                terminal_b_top.clone(),
                vec![TerminalPaneSummary {
                    title: "Task B Top".to_string(),
                    terminal_id: terminal_b_top.clone(),
                }],
                vec![1.0],
                0.24,
            )
            .expect("save task b layout");
        service
            .save_terminal_layout(
                &terminal_layout_storage_key("project-b", "project-b"),
                Vec::new(),
                terminal_project_b.clone(),
                vec![TerminalPaneSummary {
                    title: "Project B".to_string(),
                    terminal_id: terminal_project_b.clone(),
                }],
                vec![1.0],
                0.24,
            )
            .expect("save project b layout");

        let ready = service.app_runtime_ready(true, true);
        assert_eq!(
            ready.projects.selected_project_id.as_deref(),
            Some("project-a")
        );
        assert_eq!(
            ready
                .projects
                .selected_worktree_id_by_project
                .get("project-a")
                .map(String::as_str),
            Some("worktree-a")
        );
        assert_tracked_project_has_git_refresh(&service, "project-a");
        wait_for_ai_history_loading_event(
            &service,
            "worktree-a",
            &worktree_a_dir.to_string_lossy(),
        );
        let layout = service.reload_terminal_layout(Some(&layout_a_key));
        assert_eq!(layout.active_terminal_id, "");
        assert_eq!(layout.top_panes[0].terminal_id, terminal_a_top);
        assert!(layout.tabs.is_empty());
        assert_eq!(layout.top_panes[1].terminal_id, terminal_a_tab);

        let terminal_manager = service.terminal_manager();
        let launch_context = TerminalLaunchContext {
            root_project_id: "project-a".to_string(),
            project_id: "worktree-a".to_string(),
            project_name: "Task A".to_string(),
            project_path: worktree_a_dir.clone(),
            support_dir: support_dir.clone(),
            runtime_root: support_dir.join("runtime-root"),
            terminal_id: Some(terminal_a_top.clone()),
            slot_id: None,
            session_key: None,
            session_title: Some("Task A Top".to_string()),
            session_cwd: Some(worktree_a_dir.clone()),
            session_instance_id: None,
            tool_permissions_file: None,
            memory_workspace_root: None,
            memory_prompt_file: None,
            memory_index_file: None,
            host_device_id: None,
        };
        let mut config = launch_context.to_config();
        config.shell = Some("/bin/cat".to_string());
        config.cols = Some(80);
        config.rows = Some(24);
        let ensured = terminal_manager
            .ensure_session_with_context(config.clone(), Some(&launch_context))
            .expect("ensure task a terminal pty");
        assert_eq!(ensured, terminal_a_top);
        let (attached, rx) = terminal_manager
            .attach_or_create_with_context(config, Some(&launch_context), Arc::new(|_| true))
            .expect("attach task a terminal pty");
        assert_eq!(attached.id(), terminal_a_top);
        attached
            .write(b"task-a-shared-pty\n")
            .expect("write task a terminal");
        assert!(
            recv_until_contains(&rx, "task-a-shared-pty", Duration::from_secs(2))
                .contains("task-a-shared-pty")
        );

        service
            .project_select_worktree(ProjectSelectWorktreeRequest {
                project_id: "project-a".to_string(),
                worktree_id: "worktree-b".to_string(),
            })
            .expect("select worktree b");
        assert_eq!(
            service
                .project_list()
                .selected_worktree_id_by_project
                .get("project-a")
                .map(String::as_str),
            Some("worktree-b")
        );
        assert_tracked_project_has_git_refresh(&service, "project-a");
        wait_for_ai_history_loading_event(
            &service,
            "worktree-b",
            &worktree_b_dir.to_string_lossy(),
        );
        let layout = service.reload_terminal_layout(Some(&layout_b_key));
        assert_eq!(layout.active_terminal_id, "");
        assert_eq!(layout.top_panes[0].terminal_id, terminal_b_top);

        service
            .select_project("project-b")
            .expect("select project b");
        assert_eq!(
            service.project_list().selected_project_id.as_deref(),
            Some("project-b")
        );
        assert_tracked_project_has_git_refresh(&service, "project-b");
        wait_for_ai_history_loading_event(&service, "project-b", &project_b_dir.to_string_lossy());
        let layout = service
            .reload_terminal_layout(Some(&terminal_layout_storage_key("project-b", "project-b")));
        assert_eq!(layout.active_terminal_id, "");
        assert_eq!(layout.top_panes[0].terminal_id, terminal_project_b);

        let _ = terminal_manager.kill(&terminal_a_top);
        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn project_close_keeps_pet_baseline() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-project-close-pet-baseline-{}",
            uuid::Uuid::new_v4()
        ));
        let first_dir = support_dir.join("first");
        let second_dir = support_dir.join("second");
        fs::create_dir_all(&first_dir).expect("create first project dir");
        fs::create_dir_all(&second_dir).expect("create second project dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "First",
                        "path": first_dir.to_string_lossy()
                    },
                    {
                        "id": "project-2",
                        "name": "Second",
                        "path": second_dir.to_string_lossy()
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write state");
        let mut pet_snapshot = crate::pet::PetSnapshot {
            claimed_at: Some(1),
            species: "codux".to_string(),
            global_normalized_total_watermark: Some(30),
            ..crate::pet::PetSnapshot::default()
        };
        pet_snapshot
            .project_normalized_token_watermarks
            .insert("project-1".to_string(), 10);
        pet_snapshot
            .project_normalized_token_watermarks
            .insert("project-2".to_string(), 20);
        fs::write(
            support_dir.join("pet-state.json"),
            serde_json::to_vec(&pet_snapshot).expect("encode pet"),
        )
        .expect("write pet state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));

        service
            .project_close(crate::project_store::ProjectCloseRequest {
                project_id: "project-1".to_string(),
            })
            .expect("close first project");
        let pet = service.pet_snapshot().expect("pet snapshot after close");
        assert_eq!(
            pet.project_normalized_token_watermarks.get("project-1"),
            Some(&10)
        );
        assert_eq!(
            pet.project_normalized_token_watermarks.get("project-2"),
            Some(&20)
        );
        assert_eq!(pet.global_normalized_total_watermark, Some(30));

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn project_close_cleans_workspace_cache_for_root_and_worktrees() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-project-close-workspace-cache-{}",
            uuid::Uuid::new_v4()
        ));
        let project_dir = support_dir.join("project");
        let worktree_dir = support_dir.join("worktree");
        fs::create_dir_all(&project_dir).expect("create project dir");
        fs::create_dir_all(&worktree_dir).expect("create worktree dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Project",
                        "path": project_dir.to_string_lossy()
                    }
                ],
                "worktrees": [
                    {
                        "id": "worktree-1",
                        "projectId": "project-1",
                        "name": "Task",
                        "branch": "task",
                        "path": worktree_dir.to_string_lossy(),
                        "status": "active",
                        "isDefault": false,
                        "createdAt": 1,
                        "updatedAt": 1
                    }
                ],
                "worktreeTasks": [
                    {
                        "worktreeId": "worktree-1",
                        "title": "Task",
                        "baseBranch": "main",
                        "status": "active",
                        "createdAt": 1,
                        "updatedAt": 1
                    }
                ],
                "selectedProjectId": "project-1",
                "selectedWorktreeIdByProject": {
                    "project-1": "worktree-1"
                }
            })
            .to_string(),
        )
        .expect("write state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        service
            .save_terminal_layout(
                "project-1",
                Vec::new(),
                "terminal-1".to_string(),
                vec![TerminalPaneSummary {
                    title: "Shell".to_string(),
                    terminal_id: "terminal-1".to_string(),
                }],
                vec![1.0],
                0.24,
            )
            .expect("save project terminal layout");
        service
            .save_file_editor_layout(
                "worktree-1",
                vec![FileEditorTabSummary {
                    path: "src/main.rs".to_string(),
                    label: "main.rs".to_string(),
                    language: "rust".to_string(),
                }],
                Some("src/main.rs".to_string()),
            )
            .expect("save worktree file editor layout");
        let obsolete_cache =
            crate::persistent_cache::PersistentCacheStore::for_support_dir(support_dir.clone())
                .expect("obsolete cache");
        obsolete_cache
            .put_json(
                "file-tree-state",
                "worktree-1",
                &serde_json::json!({
                    "fileDirectory": "src",
                    "selectedFileEntry": "src/main.rs"
                }),
            )
            .expect("save obsolete file tree state");
        obsolete_cache
            .put_json(
                "git-ui-state",
                "worktree-1",
                &serde_json::json!({
                    "selectedGitFile": "src/main.rs"
                }),
            )
            .expect("save obsolete git ui state");

        let mut pet_snapshot = crate::pet::PetSnapshot {
            claimed_at: Some(1),
            species: "codux".to_string(),
            global_normalized_total_watermark: Some(30),
            ..crate::pet::PetSnapshot::default()
        };
        pet_snapshot
            .project_normalized_token_watermarks
            .insert("project-1".to_string(), 10);
        pet_snapshot
            .project_normalized_token_watermarks
            .insert("worktree-1".to_string(), 20);
        fs::write(
            support_dir.join("pet-state.json"),
            serde_json::to_vec(&pet_snapshot).expect("encode pet"),
        )
        .expect("write pet state");

        service
            .project_close(ProjectCloseRequest {
                project_id: "project-1".to_string(),
            })
            .expect("close project");

        assert!(service.project_list().projects.is_empty());
        assert!(
            service
                .project_list()
                .selected_worktree_id_by_project
                .is_empty()
        );
        assert!(service.terminal_layout_record("project-1").is_none());
        assert!(
            service
                .reload_file_editor_layout(Some("worktree-1"))
                .tabs
                .is_empty()
        );
        assert_eq!(
            obsolete_cache
                .get_json::<serde_json::Value>("file-tree-state", "worktree-1")
                .expect("load obsolete file tree state"),
            None
        );
        assert_eq!(
            obsolete_cache
                .get_json::<serde_json::Value>("git-ui-state", "worktree-1")
                .expect("load obsolete git ui state"),
            None
        );
        let pet = service.pet_snapshot().expect("pet snapshot");
        assert_eq!(
            pet.project_normalized_token_watermarks.get("project-1"),
            Some(&10)
        );
        assert_eq!(
            pet.project_normalized_token_watermarks.get("worktree-1"),
            Some(&20)
        );
        assert_eq!(pet.global_normalized_total_watermark, Some(30));

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn indexed_pet_totals_are_filtered_to_active_project_workspaces() {
        let mut active = HashSet::new();
        active.insert("project-1".to_string());
        active.insert("worktree-1".to_string());

        let filtered = filter_active_indexed_project_totals(
            vec![
                crate::ai_usage_store::AIUsageProjectTotal {
                    project_id: "project-1".to_string(),
                    total_tokens: 10,
                },
                crate::ai_usage_store::AIUsageProjectTotal {
                    project_id: "removed-project".to_string(),
                    total_tokens: 9_999,
                },
                crate::ai_usage_store::AIUsageProjectTotal {
                    project_id: "worktree-1".to_string(),
                    total_tokens: 20,
                },
            ],
            &active,
        );

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].project_id, "project-1");
        assert_eq!(filtered[0].total_tokens, 10);
        assert_eq!(filtered[1].project_id, "worktree-1");
        assert_eq!(filtered[1].total_tokens, 20);
    }

    #[test]
    fn pet_refresh_uses_runtime_support_dir_history_store() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-pet-runtime-history-store-{}",
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&support_dir);
        let project_dir = support_dir.join("project");
        fs::create_dir_all(&project_dir).expect("create project dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Project",
                        "path": project_dir.to_string_lossy()
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write state");

        let mut pet_snapshot = crate::pet::PetSnapshot {
            claimed_at: Some(1),
            species: "codux".to_string(),
            global_normalized_total_watermark: Some(100),
            total_normalized_tokens: 100,
            ..crate::pet::PetSnapshot::default()
        };
        pet_snapshot
            .project_normalized_token_watermarks
            .insert("project-1".to_string(), 100);
        fs::write(
            support_dir.join("pet-state.json"),
            serde_json::to_vec(&pet_snapshot).expect("encode pet state"),
        )
        .expect("write pet state");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        write_usage_bucket(
            &support_dir,
            &project_dir,
            "project-1",
            "Project",
            "before-claim",
            100,
            1.0,
        );
        write_usage_bucket(
            &support_dir,
            &project_dir,
            "project-1",
            "Project",
            "after-claim",
            30,
            10.0,
        );

        let summary = service
            .refresh_pet_from_indexed_history()
            .expect("refresh pet from indexed history");
        assert_eq!(summary.total_xp, 30);
        assert_eq!(summary.daily_xp, 30);
        let snapshot = service.pet_snapshot().expect("pet snapshot");
        assert_eq!(
            snapshot
                .project_normalized_token_watermarks
                .get("project-1"),
            Some(&130)
        );

        let summary = service
            .refresh_pet_from_indexed_history()
            .expect("refresh pet from indexed history again");
        assert_eq!(summary.total_xp, 30);
        assert_eq!(summary.daily_xp, 30);

        let second_dir = support_dir.join("second");
        fs::create_dir_all(&second_dir).expect("create second project dir");
        fs::write(
            support_dir.join("state.json"),
            json!({
                "projects": [
                    {
                        "id": "project-1",
                        "name": "Project",
                        "path": project_dir.to_string_lossy()
                    },
                    {
                        "id": "project-2",
                        "name": "Second",
                        "path": second_dir.to_string_lossy()
                    }
                ],
                "selectedProjectId": "project-1"
            })
            .to_string(),
        )
        .expect("write updated state");
        write_usage_bucket(
            &support_dir,
            &second_dir,
            "project-2",
            "Second",
            "existing-second",
            10_000,
            1.0,
        );

        let summary = service
            .refresh_pet_from_indexed_history()
            .expect("refresh pet after adding project");
        assert_eq!(summary.total_xp, 30);
        assert_eq!(summary.daily_xp, 30);

        service
            .project_close(ProjectCloseRequest {
                project_id: "project-1".to_string(),
            })
            .expect("close first project");
        let summary = service
            .refresh_pet_from_indexed_history()
            .expect("refresh pet after removing project");
        assert_eq!(summary.total_xp, 30);
        assert_eq!(summary.daily_xp, 30);
        let snapshot = service.pet_snapshot().expect("pet snapshot after remove");
        assert_eq!(
            snapshot
                .project_normalized_token_watermarks
                .get("project-1"),
            Some(&130)
        );

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn file_watch_events_are_queued_and_drained_for_gpui() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-file-watch-events-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&support_dir).expect("create support dir");
        let service = RuntimeService::new(PathBuf::from(&support_dir));

        service
            .file_watch_events
            .lock()
            .expect("file event queue")
            .push_back(FileChangeEvent {
                project_path: "/tmp/project".to_string(),
                changed_paths: vec!["/tmp/project/src/main.rs".to_string()],
            });

        let events = service.drain_file_change_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].project_path, "/tmp/project");
        assert!(service.drain_file_change_events().is_empty());

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn revoke_remote_device_preserves_connected_host_snapshot() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-revoke-remote-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&support_dir).expect("create support dir");
        fs::write(
            support_dir.join("settings.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "remote": {
                    "isEnabled": true,
                    "relayUrl": crate::remote::remote_relay_url_for_preset("china-tencent", ""),
                    "hostID": "host-1",
                    "hostToken": "secret-token",
                    "cachedDevices": [
                        {"id": "device-1", "hostId": "host-1", "name": "Phone", "online": true}
                    ]
                }
            }))
            .expect("settings json"),
        )
        .expect("write settings");

        let service = RuntimeService::new(PathBuf::from(&support_dir));
        let mut connected = service.remote_host.reload_snapshot_from_settings();
        connected.status = "connected".to_string();
        connected.message = "Remote transport connected.".to_string();
        service.remote_host.apply_snapshot(connected);

        let summary = service
            .revoke_remote_device("device-1")
            .expect("revoke device");

        assert_eq!(summary.status, "connected");
        assert_eq!(summary.message, "Remote transport connected.");
        assert_eq!(summary.devices, 0);
        assert!(summary.device_list.is_empty());

        let _ = fs::remove_dir_all(support_dir);
    }

    #[test]
    fn runtime_dock_badge_count_matches_tauri_attention_semantics() {
        let mut snapshot = AIRuntimeStateSnapshot::default();

        assert_eq!(runtime_dock_badge_count(true, &snapshot), None);

        snapshot.needs_input_count = 2;
        snapshot.completion_count = 3;

        assert_eq!(runtime_dock_badge_count(true, &snapshot), Some(5));
        assert_eq!(runtime_dock_badge_count(false, &snapshot), None);
    }

    fn write_usage_bucket(
        support_dir: &Path,
        project_dir: &Path,
        project_id: &str,
        project_name: &str,
        session_key: &str,
        total_tokens: i64,
        bucket_start: f64,
    ) {
        let store =
            crate::ai_usage_store::AIUsageStore::at_path(support_dir.join("ai-usage.sqlite3"));
        let conn = store.connect().expect("connect ai usage store");
        let project_path = project_dir.to_string_lossy().to_string();
        conn.execute(
            r#"
            INSERT INTO ai_history_file_session_link (
                source, file_path, project_path, session_key, external_session_id, project_id,
                project_name, session_title, first_seen_at, last_seen_at, last_model,
                active_duration_seconds
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            rusqlite::params![
                "codex",
                "session.jsonl",
                project_path,
                session_key,
                session_key,
                project_id,
                project_name,
                "Session",
                bucket_start,
                bucket_start + 1_800.0,
                "gpt-5",
                60_i64
            ],
        )
        .expect("insert session link");
        conn.execute(
            r#"
            INSERT INTO ai_history_file_usage_bucket (
                source, file_path, project_path, session_key, model, bucket_start, bucket_end,
                input_tokens, output_tokens, total_tokens, cached_input_tokens, request_count
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            rusqlite::params![
                "codex",
                "session.jsonl",
                project_dir.to_string_lossy().to_string(),
                session_key,
                "gpt-5",
                bucket_start,
                bucket_start + 1_800.0,
                total_tokens / 2,
                total_tokens - (total_tokens / 2),
                total_tokens,
                0_i64,
                1_i64
            ],
        )
        .expect("insert usage bucket");
    }
}
