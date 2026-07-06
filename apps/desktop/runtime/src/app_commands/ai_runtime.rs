use super::*;

pub fn llm_complete(
    service: &RuntimeService,
    request: LLMCompletionRequest,
) -> Result<LLMCompletionResponse, String> {
    service.complete_llm(request)
}
pub async fn llm_provider_test(
    provider: AIProviderSettings,
) -> Result<LLMProviderTestResult, String> {
    crate::llm::test_provider(provider).await
}
pub fn ai_runtime_snapshot(service: &RuntimeService) -> AIRuntimeBridgeSnapshot {
    service.ai_runtime_bridge_snapshot()
}
pub fn ai_runtime_probe(
    service: &RuntimeService,
    request: AIRuntimeProbeRequest,
) -> Option<AIRuntimeContextSnapshot> {
    service.ai_runtime_probe(request)
}
pub fn ai_runtime_state_snapshot(service: &RuntimeService) -> AIRuntimeStateSnapshot {
    service.ai_runtime_state_snapshot()
}
pub fn ai_runtime_dismiss_completion(service: &RuntimeService, project_id: String) -> bool {
    service.ai_runtime_dismiss_completion(&project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn llm_commands_delegate_to_runtime_llm_layer_without_network() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-app-command-llm-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&support_dir).expect("support dir");
        let service = RuntimeService::new(support_dir.clone());

        let completion_error = llm_complete(
            &service,
            LLMCompletionRequest {
                provider_id: Some("missing".to_string()),
                prompt: "Hello".to_string(),
                system_prompt: None,
                purpose: "chat".to_string(),
            },
        )
        .expect_err("missing provider");
        assert!(completion_error.contains("No available AI provider is configured"));

        let provider_error =
            crate::async_runtime::block_on(llm_provider_test(AIProviderSettings {
                id: "provider-a".to_string(),
                kind: "openAICompatible".to_string(),
                display_name: "Provider A".to_string(),
                is_enabled: true,
                model: "gpt-test".to_string(),
                base_url: "https://api.example.invalid/v1".to_string(),
                api_key: String::new(),
                use_for_memory_extraction: true,
                priority: 0,
            }))
            .expect_err("missing provider key");
        assert!(provider_error.contains("missing an API key"));

        let _ = std::fs::remove_dir_all(support_dir);
    }

    #[test]
    fn ai_runtime_and_desktop_pet_window_facades_are_available() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-app-command-window-runtime-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&support_dir).expect("support dir");
        let service = RuntimeService::new(support_dir.clone());

        let snapshot = ai_runtime_snapshot(&service);
        assert!(snapshot.terminals.is_empty());
        assert!(!snapshot.runtime_event_dir.is_empty());

        let runtime_state = ai_runtime_state_snapshot(&service);
        assert!(runtime_state.sessions.is_empty());
        assert_eq!(runtime_state.running_count, 0);

        let probed = ai_runtime_probe(
            &service,
            AIRuntimeProbeRequest {
                terminal_id: "terminal-a".to_string(),
                terminal_instance_id: None,
                project_id: "project-a".to_string(),
                project_path: None,
                tool: "codex".to_string(),
                external_session_id: None,
                transcript_path: None,
                started_at: None,
                updated_at: 0.0,
                occupied_external_session_ids: Default::default(),
            },
        );
        assert!(probed.is_none());
        assert!(!ai_runtime_dismiss_completion(
            &service,
            "project-a".to_string()
        ));

        desktop_pet_start_drag().expect("drag facade");
        desktop_pet_show_context_menu(&service).expect("context menu facade");
        let placement = desktop_pet_placement(
            &service,
            DesktopPetPhysicalPosition { x: 900.0, y: 0.0 },
            DesktopPetPhysicalSize {
                width: 352.0,
                height: 202.0,
            },
            DesktopPetWorkArea {
                x: 0.0,
                y: 0.0,
                width: 1200.0,
                height: 800.0,
                scale_factor: 1.0,
            },
        );
        assert_eq!(placement.side, "left");
        let visible = desktop_pet_set_bubble_visible(&service, true);
        assert!(visible.bubble_visible);
        let synced = desktop_pet_sync_visibility(&service).expect("sync visibility");
        assert!(synced.bubble_visible);

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
