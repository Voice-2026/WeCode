use crate::ai_runtime::{
    probe::codex::probe_codex_runtime,
    tool_driver::{
        AIRuntimeJsonHookDriver, AIRuntimeJsonHookFormat, AIRuntimeLifecycleHookFormat,
        AIRuntimeMemoryInjectionDriver, AIRuntimeToolDriver, AIRuntimeToolHookDriver,
        NO_SCREEN_PATTERNS, hook,
    },
};

pub const DRIVER: AIRuntimeToolDriver = AIRuntimeToolDriver {
    id: "codex",
    aliases: &["codex"],
    process_names: &["codex"],
    wrapper_bins: &["codex"],
    liveness_from_process: false,
    screen_starts_idle: false,
    screen_patterns: NO_SCREEN_PATTERNS,
    hook: AIRuntimeToolHookDriver::Json(AIRuntimeJsonHookDriver {
        tool: "codex",
        path_segments: &[".codex", "hooks.json"],
        format: AIRuntimeJsonHookFormat::Standard,
        definitions: &[
            hook("SessionStart", "codex-session-start", 1000, false),
            hook("UserPromptSubmit", "codex-prompt-submit", 1000, false),
            hook("PermissionRequest", "codex-permission-request", 1000, false),
            hook("Stop", "codex-stop", 1000, false),
        ],
    }),
    probe: Some(probe_codex_runtime),
    resource_paths: Some(crate::ai_runtime::tool_driver::transcript_resource_paths),
    memory_injection: AIRuntimeMemoryInjectionDriver::CodexDeveloperInstructions,
    lifecycle_hook_format: AIRuntimeLifecycleHookFormat::None,
    lifecycle_hooks: &[],
    lifecycle_config: None,
};
