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
        "wecode-pet-runtime-history-store-{}",
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
        species: "wecode".to_string(),
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
