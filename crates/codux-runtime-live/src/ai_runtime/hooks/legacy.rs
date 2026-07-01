use std::path::{Path, PathBuf};

pub(in crate::ai_runtime::hooks) struct LegacyJsonHookConfig {
    pub tool: &'static str,
    pub path_segments: &'static [&'static str],
    pub definitions: &'static [LegacyHookDefinition],
}

#[derive(Debug, Clone, Copy)]
pub(in crate::ai_runtime::hooks) struct LegacyHookDefinition {
    pub event_key: &'static str,
    pub action: &'static str,
}

pub(in crate::ai_runtime::hooks) const LEGACY_JSON_HOOK_CONFIGS: &[LegacyJsonHookConfig] = &[
    LegacyJsonHookConfig {
        tool: "codex",
        path_segments: &[".codex", "hooks.json"],
        definitions: CODEX_LEGACY_HOOKS,
    },
    LegacyJsonHookConfig {
        tool: "claude",
        path_segments: &[".claude", "settings.json"],
        definitions: CLAUDE_LEGACY_HOOKS,
    },
];

pub(in crate::ai_runtime::hooks) const CODEX_LEGACY_HOOKS: &[LegacyHookDefinition] = &[
    LegacyHookDefinition {
        event_key: "SessionStart",
        action: "codex-session-start",
    },
    LegacyHookDefinition {
        event_key: "UserPromptSubmit",
        action: "codex-prompt-submit",
    },
    LegacyHookDefinition {
        event_key: "PermissionRequest",
        action: "codex-permission-request",
    },
    LegacyHookDefinition {
        event_key: "Stop",
        action: "codex-stop",
    },
];

pub(in crate::ai_runtime::hooks) const CLAUDE_LEGACY_HOOKS: &[LegacyHookDefinition] = &[
    LegacyHookDefinition {
        event_key: "SessionStart",
        action: "session-start",
    },
    LegacyHookDefinition {
        event_key: "UserPromptSubmit",
        action: "prompt-submit",
    },
    LegacyHookDefinition {
        event_key: "PreCompact",
        action: "pre-compact",
    },
    LegacyHookDefinition {
        event_key: "PostCompact",
        action: "post-compact",
    },
    LegacyHookDefinition {
        event_key: "Stop",
        action: "stop",
    },
    LegacyHookDefinition {
        event_key: "StopFailure",
        action: "stop-failure",
    },
    LegacyHookDefinition {
        event_key: "SessionEnd",
        action: "session-end",
    },
    LegacyHookDefinition {
        event_key: "PermissionRequest",
        action: "permission-request",
    },
    LegacyHookDefinition {
        event_key: "PermissionDenied",
        action: "permission-denied",
    },
    LegacyHookDefinition {
        event_key: "Elicitation",
        action: "elicitation",
    },
    LegacyHookDefinition {
        event_key: "ElicitationResult",
        action: "elicitation-result",
    },
];

pub(in crate::ai_runtime::hooks) const AGY_LEGACY_HOOKS: &[LegacyHookDefinition] = &[
    LegacyHookDefinition {
        event_key: "SessionStart",
        action: "session-start",
    },
    LegacyHookDefinition {
        event_key: "BeforeAgent",
        action: "before-agent",
    },
    LegacyHookDefinition {
        event_key: "AfterAgent",
        action: "after-agent",
    },
    LegacyHookDefinition {
        event_key: "Notification",
        action: "notification",
    },
    LegacyHookDefinition {
        event_key: "SessionEnd",
        action: "session-end",
    },
];

pub(in crate::ai_runtime::hooks) const KIRO_LEGACY_HOOKS: &[LegacyHookDefinition] = &[
    LegacyHookDefinition {
        event_key: "agentSpawn",
        action: "session-start",
    },
    LegacyHookDefinition {
        event_key: "userPromptSubmit",
        action: "prompt-submit",
    },
    LegacyHookDefinition {
        event_key: "stop",
        action: "stop",
    },
];

pub(in crate::ai_runtime::hooks) fn legacy_json_hook_path(
    home_dir: &Path,
    config: &LegacyJsonHookConfig,
) -> PathBuf {
    config
        .path_segments
        .iter()
        .fold(home_dir.to_path_buf(), |path, segment| path.join(segment))
}
