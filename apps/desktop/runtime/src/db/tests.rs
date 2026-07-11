use super::helpers::{
    db_profiles_file_path_in, render_db_launch_context_for_profiles, sanitize_request,
};
use super::*;
use serde_json::Value;
use std::fs;
use uuid::Uuid;

fn profile_with_secret(project_id: &str) -> DBConnectionProfile {
    DBConnectionProfile {
        id: "db-1".to_string(),
        project_id: project_id.to_string(),
        name: "Production DB".to_string(),
        engine: "postgres".to_string(),
        host: "db.example.com".to_string(),
        port: 5432,
        database: "app".to_string(),
        username: "app_user".to_string(),
        password: Some("secret-password".to_string()),
        ssl_mode: "require".to_string(),
        read_only: true,
        updated_at: 1,
    }
}

#[test]
fn launch_context_lists_project_profiles_without_secrets() {
    let mut profiles = vec![
        profile_with_secret("project-a"),
        profile_with_secret("project-b"),
    ];
    profiles[1].id = "db-2".to_string();

    let context =
        render_db_launch_context_for_profiles(&mut profiles, Some("project-a"), None).unwrap();

    assert!(context.contains("wecode-db list"));
    assert!(context.contains("wecode-db <profile-id> -- '<statement>'"));
    assert!(context.contains("Always run `wecode-db list` at the time of use"));
    assert!(context.contains("Do not grep the repository"));
    assert!(context.contains("cast them to text"));
    assert!(context.contains("column::text"));
    assert!(context.contains("CAST(column AS CHAR)"));
    assert!(!context.contains("Production DB"));
    assert!(!context.contains("db-1"));
    assert!(!context.contains("db-2"));
    assert!(!context.contains("secret-password"));
    assert!(!context.contains("app_user"));
}

#[cfg(unix)]
#[test]
fn db_test_profile_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let profile = profile_with_secret("project-a");
    let path = super::test_command::write_test_profile_file(&profile).unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    fs::remove_file(path).ok();
}

#[test]
fn db_store_filters_profiles_by_root_project() {
    let support_dir = std::env::temp_dir().join(format!("wecode-db-store-{}", Uuid::new_v4()));
    fs::create_dir_all(&support_dir).unwrap();
    let store = DBStore::from_support_dir(support_dir.clone());

    store
        .upsert(DBProfileUpsertRequest {
            id: Some("db-1".to_string()),
            project_id: "project-a".to_string(),
            name: "A".to_string(),
            engine: "postgres".to_string(),
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "app_a".to_string(),
            username: Some("user_a".to_string()),
            password: Some("secret-a".to_string()),
            ssl_mode: Some("prefer".to_string()),
            read_only: true,
        })
        .unwrap();
    store
        .upsert(DBProfileUpsertRequest {
            id: Some("db-2".to_string()),
            project_id: "project-b".to_string(),
            name: "B".to_string(),
            engine: "mysql".to_string(),
            host: Some("localhost".to_string()),
            port: Some(3306),
            database: "app_b".to_string(),
            username: Some("user_b".to_string()),
            password: Some("secret-b".to_string()),
            ssl_mode: Some("prefer".to_string()),
            read_only: false,
        })
        .unwrap();

    let project_a = store.snapshot(Some("project-a"));
    assert_eq!(project_a.profiles.len(), 1);
    assert_eq!(project_a.profiles[0].id, "db-1");

    let raw =
        crate::config::ConfigDocumentStore::for_file(db_profiles_file_path_in(support_dir.clone()))
            .snapshot();
    let profiles = raw.as_array().expect("db profiles root array");
    assert_eq!(profiles.len(), 2);
    assert_eq!(
        profiles[0].get("password").and_then(Value::as_str),
        Some("secret-a")
    );

    fs::remove_dir_all(support_dir).ok();
}

#[test]
fn sqlite_profiles_do_not_require_username_or_host() {
    let profile = sanitize_request(DBProfileUpsertRequest {
        id: None,
        project_id: "project-a".to_string(),
        name: "Local".to_string(),
        engine: "sqlite".to_string(),
        host: None,
        port: None,
        database: "/tmp/app.sqlite3".to_string(),
        username: None,
        password: None,
        ssl_mode: None,
        read_only: true,
    })
    .unwrap();

    assert_eq!(profile.engine, "sqlite");
    assert!(profile.username.is_empty());
}

#[cfg(not(windows))]
#[test]
fn wecode_db_wrapper_lists_project_profiles_without_secrets() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("wecode-db-wrapper-list-{}", Uuid::new_v4()));
    let wrappers = dir.join("runtime-assets/scripts/wrappers");
    let bin = wrappers.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let wrapper = bin.join("wecode-db");
    let helper = wrappers.join("wecode-wrapper-helper");
    let profiles = dir.join("db_profiles.json");

    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("runtime-assets/scripts/wrappers/bin/wecode-db"),
        &wrapper,
    )
    .unwrap();
    fs::write(
        &helper,
        "#!/bin/sh\n\
         if [ \"$1\" != \"--wecode-wrapper-helper\" ]; then exit 64; fi\n\
         if [ \"$2\" != \"db-list-profiles\" ]; then exit 64; fi\n\
         printf '%s\\n' '{\"profiles\":[{\"id\":\"db-1\",\"name\":\"Production\",\"engine\":\"postgres\",\"database\":\"app\",\"endpoint\":\"db.example.com:5432/app\",\"readOnly\":true}]}'\n",
    )
    .unwrap();
    fs::write(
        &profiles,
        serde_json::json!([{
            "id": "db-1",
            "projectId": "project-a",
            "name": "Production",
            "engine": "postgres",
            "host": "db.example.com",
            "port": 5432,
            "database": "app",
            "username": "app_user",
            "password": "secret-password",
            "readOnly": true,
            "updatedAt": 1
        }])
        .to_string(),
    )
    .unwrap();
    for executable in [&wrapper, &helper] {
        let mut permissions = fs::metadata(executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(executable, permissions).unwrap();
    }

    let output = Command::new("zsh")
        .arg(&wrapper)
        .arg("list")
        .env("WECODE_DB_PROFILES_FILE", &profiles)
        .env("WECODE_DB_PROJECT_ID", "project-a")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "wecode-db list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Production"), "{stdout}");
    assert!(!stdout.contains("secret-password"), "{stdout}");
    assert!(!stdout.contains("app_user"), "{stdout}");

    fs::remove_dir_all(dir).ok();
}
