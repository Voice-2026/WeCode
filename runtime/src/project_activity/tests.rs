use super::*;

#[test]
fn ai_refresh_uses_foreground_and_background_intervals() {
    let coordinator =
        ProjectActivityCoordinator::new(std::env::temp_dir(), AIHistoryIndexer::new());
    let now = Instant::now();
    {
        let mut projects = coordinator.projects.lock().unwrap();
        projects.insert(
            "active".to_string(),
            TrackedProject {
                id: "active".to_string(),
                name: "Active".to_string(),
                path: "/tmp/active".to_string(),
                last_git_refresh: None,
                last_ai_refresh: Some(now - Duration::from_secs(180)),
            },
        );
        projects.insert(
            "background".to_string(),
            TrackedProject {
                id: "background".to_string(),
                name: "Background".to_string(),
                path: "/tmp/background".to_string(),
                last_git_refresh: None,
                last_ai_refresh: Some(now - Duration::from_secs(180)),
            },
        );
    }
    *coordinator.active_project_id.lock().unwrap() = Some("active".to_string());
    coordinator.mark_main_window_visible(true);

    let due = coordinator.projects_due_for_ai(Duration::from_secs(120), Duration::from_secs(600));

    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, "active");
}

#[test]
fn ai_background_refresh_runs_when_background_project_is_due() {
    let coordinator =
        ProjectActivityCoordinator::new(std::env::temp_dir(), AIHistoryIndexer::new());
    let now = Instant::now();
    {
        let mut projects = coordinator.projects.lock().unwrap();
        projects.insert(
            "active".to_string(),
            TrackedProject {
                id: "active".to_string(),
                name: "Active".to_string(),
                path: "/tmp/active".to_string(),
                last_git_refresh: None,
                last_ai_refresh: Some(now - Duration::from_secs(700)),
            },
        );
        projects.insert(
            "background".to_string(),
            TrackedProject {
                id: "background".to_string(),
                name: "Background".to_string(),
                path: "/tmp/background".to_string(),
                last_git_refresh: None,
                last_ai_refresh: Some(now - Duration::from_secs(700)),
            },
        );
    }
    *coordinator.active_project_id.lock().unwrap() = Some("active".to_string());
    coordinator.mark_main_window_visible(true);

    let due = coordinator.projects_due_for_ai(Duration::from_secs(120), Duration::from_secs(600));
    let ids = due
        .into_iter()
        .map(|project| project.id)
        .collect::<HashSet<_>>();

    assert_eq!(
        ids,
        HashSet::from(["active".to_string(), "background".to_string()])
    );
}

#[test]
fn git_background_refresh_is_limited_per_tick() {
    let projects = Mutex::new(HashMap::new());
    let now = Instant::now();
    {
        let mut guard = projects.lock().unwrap();
        for index in 0..5 {
            guard.insert(
                format!("background-{index}"),
                TrackedProject {
                    id: format!("background-{index}"),
                    name: format!("Background {index}"),
                    path: format!("/tmp/background-{index}"),
                    last_git_refresh: Some(now - Duration::from_secs(700)),
                    last_ai_refresh: None,
                },
            );
        }
        guard.insert(
            "active".to_string(),
            TrackedProject {
                id: "active".to_string(),
                name: "Active".to_string(),
                path: "/tmp/active".to_string(),
                last_git_refresh: Some(now - Duration::from_secs(30)),
                last_ai_refresh: None,
            },
        );
    }

    let due = projects_due_for_git_interval(
        &projects,
        Some("active"),
        true,
        Duration::from_secs(15),
        Duration::from_secs(600),
        2,
    );
    let active_count = due.iter().filter(|project| project.id == "active").count();
    let background_count = due.iter().filter(|project| project.id != "active").count();

    assert_eq!(active_count, 1);
    assert_eq!(background_count, 2);
    assert_eq!(due.len(), 3);
}
