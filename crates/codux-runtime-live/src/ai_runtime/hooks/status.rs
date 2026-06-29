use super::{
    codewhale::codewhale_hook_config_status_in, command::is_managed_hook, json::load_json_object,
    kimi::kimi_hook_config_status_in,
};
use crate::{
    ai_runtime::bridge::{AIRuntimeHookConfigStatus, AIRuntimeToolHookConfigStatus},
    ai_runtime::tool_driver::{AIRuntimeToolHookDriver, ai_runtime_tool_drivers},
    runtime_paths::{app_slug, home_dir},
};
use serde_json::{Map, Value};
use std::path::Path;

pub fn hook_config_status(wrapper_dir: &Path) -> AIRuntimeHookConfigStatus {
    hook_config_status_in(&home_dir(), wrapper_dir)
}

pub fn hook_config_status_in(home_dir: &Path, wrapper_dir: &Path) -> AIRuntimeHookConfigStatus {
    let mut codex = AIRuntimeToolHookConfigStatus::default();
    let mut claude = AIRuntimeToolHookConfigStatus::default();
    let mut kiro = AIRuntimeToolHookConfigStatus::default();
    let mut codewhale = AIRuntimeToolHookConfigStatus::default();
    let mut kimi = AIRuntimeToolHookConfigStatus::default();
    let opencode = opencode_hook_config_status(&wrapper_dir.join("opencode-config"));
    let mimo = opencode.clone();

    for driver in ai_runtime_tool_drivers() {
        let status = match driver.hook {
            AIRuntimeToolHookDriver::Json(hook) => {
                let path = hook
                    .path_segments
                    .iter()
                    .fold(home_dir.to_path_buf(), |path, segment| path.join(segment));
                tool_hook_config_status(
                    &path,
                    hook.tool,
                    hook.definitions
                        .iter()
                        .map(|definition| (definition.event_key, definition.action))
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            }
            AIRuntimeToolHookDriver::CodeWhaleToml => {
                let Some(config) = driver.lifecycle_config else {
                    continue;
                };
                codewhale_hook_config_status_in(
                    &wrapper_dir.join(config.relative_path),
                    driver.lifecycle_hooks,
                )
            }
            AIRuntimeToolHookDriver::KimiToml => kimi_hook_config_status_in(home_dir),
            AIRuntimeToolHookDriver::OpenCodePlugin | AIRuntimeToolHookDriver::None => continue,
        };
        match driver.id {
            "codex" => codex = status,
            "claude" => claude = status,
            "kiro" => kiro = status,
            "codewhale" => codewhale = status,
            "kimi" => kimi = status,
            _ => {}
        }
    }

    AIRuntimeHookConfigStatus {
        codex,
        claude,
        opencode,
        mimo,
        kiro,
        codewhale,
        kimi,
    }
}

pub fn tool_hook_config_status(
    path: &Path,
    tool: &str,
    definitions: &[(&str, &str)],
) -> AIRuntimeToolHookConfigStatus {
    let owner = app_slug();
    let root = load_json_object(path).unwrap_or_default();
    let hooks = root
        .get("hooks")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let missing = definitions
        .iter()
        .filter_map(|(event_key, action)| {
            (!has_managed_hook_for_event(&hooks, event_key, action, owner, tool))
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
    let expected = ["package.json", "plugins/dmux-runtime.js"];
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
    owner: &str,
    tool: &str,
) -> bool {
    hooks
        .get(event_key)
        .and_then(|value| value.as_array())
        .map(|groups| {
            groups.iter().any(|group| {
                is_managed_hook(group, action, owner, tool)
                    || group
                        .get("hooks")
                        .and_then(|value| value.as_array())
                        .map(|items| {
                            items
                                .iter()
                                .any(|item| is_managed_hook(item, action, owner, tool))
                        })
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_paths::app_slug;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn tool_hook_config_status_requires_claude_compaction_hooks() {
        let root = std::env::temp_dir().join(format!("codux-claude-hooks-{}.json", Uuid::new_v4()));
        fs::write(
            &root,
            serde_json::json!({
                "hooks": {
                    "PreCompact": [{
                        "matcher": "",
                        "hooks": [{
                            "type": "command",
                            "command": format!("'/tmp/dmux-ai-state.sh' 'pre-compact' '{}' 'claude'", app_slug()),
                            "timeout": 10
                        }]
                    }],
                    "PostCompact": [{
                        "matcher": "",
                        "hooks": [{
                            "type": "command",
                            "command": format!("'/tmp/dmux-ai-state.sh' 'post-compact' '{}' 'claude'", app_slug()),
                            "timeout": 10
                        }]
                    }]
                }
            })
            .to_string(),
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

    #[test]
    fn tool_hook_config_status_ignores_other_owner_hooks() {
        let root = std::env::temp_dir().join(format!("codux-owner-hooks-{}.json", Uuid::new_v4()));
        let other_owner = if app_slug() == "codux" {
            "codux-dev"
        } else {
            "codux"
        };
        fs::write(
            &root,
            serde_json::json!({
                "hooks": {
                    "Stop": [{
                        "matcher": "",
                        "hooks": [{
                            "type": "command",
                            "command": format!("'/tmp/dmux-ai-state.sh' 'stop' '{}' 'claude'", other_owner),
                            "timeout": 10
                        }]
                    }]
                }
            })
            .to_string(),
        )
        .unwrap();

        let status = tool_hook_config_status(&root, "claude", &[("Stop", "stop")]);

        assert!(!status.configured);
        assert_eq!(status.missing, vec!["Stop:stop"]);
        fs::remove_file(root).unwrap();
    }

    #[test]
    fn opencode_hook_config_status_matches_embedded_runtime_assets() {
        let home = std::env::temp_dir().join(format!("codux-opencode-hooks-{}", Uuid::new_v4()));
        let config = home.join("opencode-config");
        fs::create_dir_all(config.join("plugins")).unwrap();
        fs::write(config.join("package.json"), "{}").unwrap();
        fs::write(config.join("plugins/dmux-runtime.js"), "export {};").unwrap();

        let status = opencode_hook_config_status(&config);

        assert!(status.configured);
        assert!(status.missing.is_empty());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn hook_config_status_reports_missing_codewhale_hooks() {
        let root = std::env::temp_dir().join(format!("codux-codewhale-hooks-{}", Uuid::new_v4()));
        let home = root.join("home");
        let wrapper_dir = root.join("wrappers");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let status = hook_config_status_in(&home, &wrapper_dir);

        assert!(!status.codewhale.configured);
        assert!(!status.codewhale.missing.is_empty());
        assert!(
            status
                .codewhale
                .config_path
                .ends_with("managed-config/codewhale.toml")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hook_config_status_accepts_staged_codewhale_lifecycle_config() {
        let root = std::env::temp_dir().join(format!("codux-codewhale-hooks-{}", Uuid::new_v4()));
        let home = root.join("home");
        let wrapper_dir = root.join("wrappers");
        let config = wrapper_dir.join("managed-config").join("codewhale.toml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(
            &config,
            format!(
                r#"
[hooks]
enabled = true

[[hooks.hooks]]
name = "codux-codewhale-message-submit"
event = "message_submit"
command = "'/tmp/dmux-ai-state.sh' 'codewhale-message-submit' '{}' 'codewhale'"

[[hooks.hooks]]
name = "codux-codewhale-turn-end"
event = "turn_end"
command = "'/tmp/dmux-ai-state.sh' 'codewhale-turn-end' '{}' 'codewhale'"
"#,
                app_slug(),
                app_slug()
            ),
        )
        .unwrap();

        let status = hook_config_status_in(&home, &wrapper_dir);

        assert!(status.codewhale.configured);
        assert!(status.codewhale.missing.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hook_config_status_omits_db_only_agy() {
        let home = std::env::temp_dir().join(format!("codux-agy-hooks-{}", Uuid::new_v4()));
        fs::create_dir_all(home.join(".gemini")).unwrap();

        let status = hook_config_status_in(&home, &home.join("wrappers"));

        assert!(!status.codex.configured);
        assert!(!status.claude.configured);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn hook_config_status_omits_file_screen_only_kiro() {
        let home = std::env::temp_dir().join(format!("codux-kiro-hooks-{}", Uuid::new_v4()));
        fs::create_dir_all(home.join(".kiro").join("agents")).unwrap();

        let status = hook_config_status_in(&home, &home.join("wrappers"));

        assert!(!status.kiro.configured);
        assert!(status.kiro.config_path.is_empty());
        assert!(status.kiro.missing.is_empty());
        fs::remove_dir_all(home).unwrap();
    }
}
