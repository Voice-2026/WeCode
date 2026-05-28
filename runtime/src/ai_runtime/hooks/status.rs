use super::{command::is_managed_hook, json::load_json_object};
use crate::{
    ai_runtime::bridge::{AIRuntimeHookConfigStatus, AIRuntimeToolHookConfigStatus},
    runtime_paths::home_dir,
};
use serde_json::{Map, Value};
use std::path::Path;

pub fn hook_config_status(opencode_config_dir: &Path) -> AIRuntimeHookConfigStatus {
    AIRuntimeHookConfigStatus {
        codex: tool_hook_config_status(
            &home_dir().join(".codex").join("hooks.json"),
            "codex",
            &[
                ("SessionStart", "codex-session-start"),
                ("UserPromptSubmit", "codex-prompt-submit"),
                ("PermissionRequest", "codex-permission-request"),
                ("Stop", "codex-stop"),
            ],
        ),
        claude: tool_hook_config_status(
            &home_dir().join(".claude").join("settings.json"),
            "claude",
            &[
                ("SessionStart", "session-start"),
                ("UserPromptSubmit", "prompt-submit"),
                ("PreCompact", "pre-compact"),
                ("PostCompact", "post-compact"),
                ("Stop", "stop"),
                ("StopFailure", "stop-failure"),
                ("SessionEnd", "session-end"),
                ("PermissionRequest", "permission-request"),
                ("PermissionDenied", "permission-denied"),
                ("Elicitation", "elicitation"),
                ("ElicitationResult", "elicitation-result"),
            ],
        ),
        gemini: tool_hook_config_status(
            &home_dir().join(".gemini").join("settings.json"),
            "gemini",
            &[
                ("SessionStart", "session-start"),
                ("BeforeAgent", "before-agent"),
                ("AfterAgent", "after-agent"),
                ("Notification", "notification"),
                ("SessionEnd", "session-end"),
            ],
        ),
        opencode: opencode_hook_config_status(opencode_config_dir),
        kiro: tool_hook_config_status(
            &home_dir()
                .join(".kiro")
                .join("agents")
                .join("codux-managed.json"),
            "kiro",
            &[("agentSpawn", "session-start"), ("stop", "session-end")],
        ),
    }
}

pub fn tool_hook_config_status(
    path: &Path,
    tool: &str,
    definitions: &[(&str, &str)],
) -> AIRuntimeToolHookConfigStatus {
    let root = load_json_object(path).unwrap_or_default();
    let hooks = root
        .get("hooks")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let missing = definitions
        .iter()
        .filter_map(|(event_key, action)| {
            (!has_managed_hook_for_event(&hooks, event_key, action, tool))
                .then(|| format!("{event_key}:{action}"))
        })
        .collect::<Vec<_>>();
    AIRuntimeToolHookConfigStatus {
        configured: missing.is_empty(),
        config_path: path.display().to_string(),
        missing,
    }
}

pub fn opencode_hook_config_status(config_dir: &Path) -> AIRuntimeToolHookConfigStatus {
    let expected = [
        "package.json",
        "plugins/dmux-runtime.js",
        "node_modules/@opencode-ai/plugin/package.json",
    ];
    let missing = expected
        .iter()
        .filter(|relative| !config_dir.join(relative).exists())
        .map(|relative| relative.to_string())
        .collect::<Vec<_>>();
    AIRuntimeToolHookConfigStatus {
        configured: missing.is_empty(),
        config_path: config_dir.display().to_string(),
        missing,
    }
}

fn has_managed_hook_for_event(
    hooks: &Map<String, Value>,
    event_key: &str,
    action: &str,
    tool: &str,
) -> bool {
    hooks
        .get(event_key)
        .and_then(|value| value.as_array())
        .map(|groups| {
            groups.iter().any(|group| {
                is_managed_hook(group, action, tool)
                    || group
                        .get("hooks")
                        .and_then(|value| value.as_array())
                        .map(|items| items.iter().any(|item| is_managed_hook(item, action, tool)))
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn tool_hook_config_status_requires_claude_compaction_hooks() {
        let root = std::env::temp_dir().join(format!("codux-claude-hooks-{}.json", Uuid::new_v4()));
        fs::write(
            &root,
            r#"{
              "hooks": {
                "PreCompact": [
                  {
                    "matcher": "",
                    "hooks": [
                      {
                        "type": "command",
                        "command": "'/tmp/dmux-ai-state.sh' 'pre-compact' 'codux-tauri' 'claude'",
                        "timeout": 10
                      }
                    ]
                  }
                ],
                "PostCompact": [
                  {
                    "matcher": "",
                    "hooks": [
                      {
                        "type": "command",
                        "command": "'/tmp/dmux-ai-state.sh' 'post-compact' 'codux-tauri' 'claude'",
                        "timeout": 10
                      }
                    ]
                  }
                ]
              }
            }"#,
        )
        .unwrap();

        let status = tool_hook_config_status(
            &root,
            "claude",
            &[
                ("PreCompact", "pre-compact"),
                ("PostCompact", "post-compact"),
                ("Stop", "stop"),
            ],
        );

        assert!(!status.configured);
        assert_eq!(status.missing, vec!["Stop:stop"]);
        fs::remove_file(root).unwrap();
    }
}
