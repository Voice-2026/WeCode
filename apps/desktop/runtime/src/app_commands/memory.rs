use super::*;

pub fn memory_extraction_cancel(
    service: &RuntimeService,
) -> Result<MemoryExtractionStatusSnapshot, String> {
    service.cancel_memory_extraction_queue()
}
pub fn memory_extraction_status(
    service: &RuntimeService,
) -> Result<MemoryExtractionStatusSnapshot, String> {
    service.memory_extraction_status()
}
pub fn memory_extraction_clear_failures(
    service: &RuntimeService,
) -> Result<MemoryExtractionStatusSnapshot, String> {
    service.clear_memory_extraction_failures()
}
pub fn memory_management_snapshot(
    service: &RuntimeService,
    request: MemoryManagementRequest,
) -> Result<MemoryManagementSnapshot, String> {
    service.memory_management_snapshot(request)
}
pub fn memory_manager_snapshot(
    service: &RuntimeService,
    request: MemoryManagerSnapshotRequest,
) -> Result<MemoryManagerSnapshot, String> {
    Ok(service.memory_manager_snapshot(&service.reload_state().projects, request))
}
pub fn memory_archive_entry(service: &RuntimeService, entry_id: String) -> Result<(), String> {
    service.archive_memory_entry(None, &entry_id).map(|_| ())
}
pub fn memory_delete_entry(service: &RuntimeService, entry_id: String) -> Result<(), String> {
    service.delete_memory_entry(None, &entry_id).map(|_| ())
}
pub fn memory_delete_summary(service: &RuntimeService, summary_id: String) -> Result<(), String> {
    service.delete_memory_summary(None, &summary_id).map(|_| ())
}
pub fn memory_delete_project_profile(
    service: &RuntimeService,
    project_id: String,
) -> Result<(), String> {
    service
        .delete_memory_project_profile(&project_id)
        .map(|_| ())
}
pub fn memory_delete_project(service: &RuntimeService, project_id: String) -> Result<(), String> {
    service.delete_memory_project(&project_id).map(|_| ())
}
pub fn memory_migrate_project(
    service: &RuntimeService,
    request: MemoryProjectMigrationRequest,
) -> Result<(), String> {
    service.migrate_memory_project(request).map(|_| ())
}
pub fn memory_update_summary(
    service: &RuntimeService,
    request: MemorySummaryUpdateRequest,
) -> Result<MemorySummaryRow, String> {
    service.update_memory_summary(request)
}
pub async fn memory_index_now(
    service: &RuntimeService,
) -> Result<MemoryExtractionStatusSnapshot, String> {
    service.process_memory_sessions_now().await
}
pub async fn memory_refresh_project_profile(
    service: &RuntimeService,
    project_id: String,
) -> Result<MemoryProjectProfileRefreshResult, String> {
    service
        .force_refresh_memory_project_profile_with_llm(&project_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn memory_commands_delegate_to_runtime_memory_store() {
        let support_dir =
            std::env::temp_dir().join(format!("wecode-app-command-memory-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&support_dir).expect("support dir");
        let service = RuntimeService::new(support_dir.clone());

        let status = memory_extraction_status(&service).expect("status");
        assert_eq!(status.pending_count, 0);

        let canceled = memory_extraction_cancel(&service).expect("cancel queue");
        assert_eq!(canceled.pending_count, 0);

        let manager = memory_manager_snapshot(
            &service,
            MemoryManagerSnapshotRequest {
                scope: "all".to_string(),
                project_id: None,
                tab: "active".to_string(),
                limit: Some(20),
            },
        )
        .expect("manager snapshot");
        assert!(!manager.selected_target_title.is_empty());
        assert!(manager.current_overview.active_entry_count >= 0);

        assert!(
            crate::async_runtime::block_on(memory_refresh_project_profile(
                &service,
                "missing-project".to_string(),
            ))
            .expect_err("missing project profile")
            .contains("Project not found")
        );

        assert!(
            memory_archive_entry(&service, String::new())
                .expect_err("empty archive id")
                .contains("Memory entry id is empty")
        );
        assert!(
            memory_delete_entry(&service, String::new())
                .expect_err("empty delete id")
                .contains("Memory entry id is empty")
        );
        assert!(
            memory_delete_summary(&service, String::new())
                .expect_err("empty summary id")
                .contains("Memory summary id is empty")
        );
        assert!(
            memory_delete_project_profile(&service, String::new())
                .expect_err("empty project profile id")
                .contains("Project id is empty")
        );
        assert!(
            memory_delete_project(&service, String::new())
                .expect_err("empty project id")
                .contains("Project id is empty")
        );
        assert!(
            memory_migrate_project(
                &service,
                MemoryProjectMigrationRequest {
                    from_project_id: String::new(),
                    to_project_id: "project-b".to_string(),
                    overwrite: false,
                },
            )
            .expect_err("empty migrate project id")
            .contains("project id cannot be empty")
        );
        assert!(
            memory_update_summary(
                &service,
                MemorySummaryUpdateRequest {
                    summary_id: String::new(),
                    content: String::new(),
                    max_versions: None,
                },
            )
            .expect_err("empty summary content")
            .contains("summary content cannot be empty")
        );

        assert!(
            crate::async_runtime::block_on(memory_index_now(&service))
                .expect_err("memory index without provider")
                .contains("Memory needs an enabled AI provider")
        );

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
