use super::*;

pub fn ssh_profile_upsert(
    service: &RuntimeService,
    request: SSHProfileUpsertRequest,
) -> Result<SSHProfilesSnapshot, String> {
    service.upsert_ssh_profile(request)
}
pub fn ssh_profile_delete(
    service: &RuntimeService,
    profile_id: String,
) -> Result<SSHProfilesSnapshot, String> {
    service.delete_ssh_profile(profile_id)
}
pub fn ssh_profile_test(
    service: &RuntimeService,
    request: SSHProfileUpsertRequest,
    runtime_assets: PathBuf,
) -> Result<SSHProfileTestResult, String> {
    service.test_ssh_profile(request, runtime_assets)
}
pub fn ssh_profiles(service: &RuntimeService) -> SSHProfilesSnapshot {
    service.ssh_profiles()
}
pub fn ssh_launch_command(
    service: &RuntimeService,
    profile_id: String,
) -> Result<SSHLaunchCommand, String> {
    service.ssh_launch_command(profile_id)
}
pub fn db_profile_upsert(
    service: &RuntimeService,
    request: DBProfileUpsertRequest,
) -> Result<DBProfilesSnapshot, String> {
    service.upsert_db_profile(request)
}
pub fn db_profile_delete(
    service: &RuntimeService,
    project_id: String,
    profile_id: String,
) -> Result<DBProfilesSnapshot, String> {
    service.delete_db_profile(&project_id, profile_id)
}
pub fn db_profile_test(
    service: &RuntimeService,
    request: DBProfileUpsertRequest,
    runtime_assets: PathBuf,
) -> Result<DBQueryResult, String> {
    service.test_db_profile(request, runtime_assets)
}
pub fn db_profiles(service: &RuntimeService, project_id: Option<String>) -> DBProfilesSnapshot {
    service.db_profiles(project_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn ssh_profile_commands_upsert_delete_and_test_without_real_connection() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-app-command-ssh-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&support_dir).expect("support dir");
        let service = RuntimeService::new(support_dir.clone());
        let request = SSHProfileUpsertRequest {
            id: Some("profile-1".to_string()),
            name: "Production".to_string(),
            host: "example.com".to_string(),
            port: 2222,
            username: "root".to_string(),
            credential_kind: "password".to_string(),
            private_key_path: None,
            password: Some("secret".to_string()),
            key_passphrase: None,
        };

        let snapshot = ssh_profile_upsert(&service, request.clone()).expect("upsert profile");
        assert_eq!(snapshot.profiles.len(), 1);
        assert_eq!(snapshot.profiles[0].id, "profile-1");
        assert_eq!(snapshot.profiles[0].host, "example.com");

        let profiles = ssh_profiles(&service);
        assert_eq!(profiles.profiles.len(), 1);
        assert_eq!(profiles.profiles[0].id, "profile-1");

        let launch = ssh_launch_command(&service, "profile-1".to_string()).expect("launch command");
        assert!(launch.command.contains("codux-ssh"));
        assert!(launch.command.contains("profile-1"));
        assert!(launch.log_command.contains("codux-ssh"));
        assert!(launch.log_command.contains("profile-1"));

        let test_error = ssh_profile_test(&service, request, support_dir.join("missing-bin"))
            .expect_err("missing wrapper should fail before connecting");
        assert!(test_error.contains("codux-ssh wrapper is not ready"));

        let snapshot =
            ssh_profile_delete(&service, "profile-1".to_string()).expect("delete profile");
        assert!(snapshot.profiles.is_empty());

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
