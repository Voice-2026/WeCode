#[test]
fn app_runtime_ready_marks_selected_project_active_and_returns_startup_snapshots() {
    let support_dir =
        std::env::temp_dir().join(format!("wecode-runtime-ready-{}", uuid::Uuid::new_v4()));
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
        std::env::temp_dir().join(format!("wecode-project-update-{}", uuid::Uuid::new_v4()));
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
        "wecode-project-select-worktree-{}",
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
fn project_and_worktree_switch_does_not_restore_saved_terminal_layout() {
    let support_dir = std::env::temp_dir().join(format!(
        "wecode-project-worktree-terminal-layout-{}",
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
    assert!(state.terminal_layout.top_panes.is_empty());

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
    assert!(state.terminal_layout.top_panes.is_empty());

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
    assert!(state.terminal_layout.top_panes.is_empty());

    let _ = fs::remove_dir_all(support_dir);
}

#[cfg(unix)]
#[test]
fn project_and_worktree_switch_runs_runtime_activation_layout_pty_ai_and_git_flow() {
    let support_dir = std::env::temp_dir().join(format!(
        "wecode-runtime-switch-full-flow-{}",
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
    wait_for_ai_history_loading_event(&service, "worktree-a", &worktree_a_dir.to_string_lossy());
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
    wait_for_ai_history_loading_event(&service, "worktree-b", &worktree_b_dir.to_string_lossy());
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
