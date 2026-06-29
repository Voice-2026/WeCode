use crate::ai_runtime::{
    probe::claude::probe_claude_runtime,
    tool_driver::{
        AIRuntimeJsonHookDriver, AIRuntimeJsonHookFormat, AIRuntimeLifecycleHookFormat,
        AIRuntimeMemoryInjectionDriver, AIRuntimeToolDriver, AIRuntimeToolHookDriver,
        NO_SCREEN_PATTERNS, hook,
    },
};

pub const DRIVER: AIRuntimeToolDriver = AIRuntimeToolDriver {
    id: "claude",
    aliases: &["claude", "claude-code", "reclaude"],
    process_names: &["claude", "claude-code", "reclaude"],
    wrapper_bins: &["claude", "claude-code", "reclaude"],
    liveness_from_process: false,
    screen_starts_idle: false,
    screen_patterns: NO_SCREEN_PATTERNS,
    hook: AIRuntimeToolHookDriver::Json(AIRuntimeJsonHookDriver {
        tool: "claude",
        path_segments: &[".claude", "settings.json"],
        format: AIRuntimeJsonHookFormat::Standard,
        definitions: &[
            hook("SessionStart", "session-start", 10, false),
            hook("UserPromptSubmit", "prompt-submit", 10, false),
            hook("PreCompact", "pre-compact", 10, false),
            hook("PostCompact", "post-compact", 10, false),
            hook("Stop", "stop", 10, false),
            hook("StopFailure", "stop-failure", 10, false),
            hook("SessionEnd", "session-end", 1, false),
            hook("PermissionRequest", "permission-request", 5, true),
            hook("PermissionDenied", "permission-denied", 5, true),
            hook("Elicitation", "elicitation", 10, true),
            hook("ElicitationResult", "elicitation-result", 10, true),
        ],
    }),
    probe: Some(probe_claude_runtime),
    resource_paths: Some(crate::ai_runtime::tool_driver::transcript_resource_paths),
    memory_injection: AIRuntimeMemoryInjectionDriver::ClaudeAppendSystemPrompt,
    lifecycle_hook_format: AIRuntimeLifecycleHookFormat::None,
    lifecycle_hooks: &[],
    lifecycle_config: None,
};
