use super::{
    codewhale::uninstall_codewhale_hooks_in,
    codex::uninstall_codex_config,
    command::is_managed_hook_action,
    json::load_json_object,
    json::write_json_object,
    kimi::uninstall_kimi_hooks_in,
    legacy::{
        AGY_LEGACY_HOOKS, KIRO_LEGACY_HOOKS, LEGACY_JSON_HOOK_CONFIGS, LegacyHookDefinition,
        legacy_json_hook_path,
    },
};
use crate::ai_runtime::tool_driver::{AIRuntimeToolHookDriver, ai_runtime_tool_drivers};
use serde_json::{Map, Value};
use std::path::Path;

/// Remove every wecode-managed hook entry this build (or a prior one) wrote into
/// the CLIs' own config files, leaving them genuinely hookless -- the point of
/// the non-intrusive runtime. Each format preserves the user's own hooks, skips
/// its write when nothing changed, and never creates a config file that was
/// absent (so a tool the user never set up stays untouched). Stripping is
/// idempotent, so it is safe to run on every start; once a config is clean the
/// subsequent runs are no-op no-write passes.
pub fn uninstall_managed_hook_configs_in(home_dir: &Path) -> Result<(), String> {
    for config in LEGACY_JSON_HOOK_CONFIGS {
        let path = legacy_json_hook_path(home_dir, config);
        uninstall_tool_hooks(&path, config.tool, config.definitions)?;
        if config.tool == "codex" {
            uninstall_codex_config(&path)?;
        }
    }
    for driver in ai_runtime_tool_drivers() {
        match driver.hook {
            AIRuntimeToolHookDriver::CodeWhaleToml => {
                uninstall_codewhale_hooks_in(home_dir)?;
            }
            AIRuntimeToolHookDriver::OpenCodePlugin | AIRuntimeToolHookDriver::None => {}
        }
    }
    uninstall_kimi_hooks_in(home_dir)?;
    let agy_settings_path = home_dir
        .join(".gemini")
        .join("antigravity-cli")
        .join("settings.json");
    uninstall_tool_hooks(&agy_settings_path, "agy", AGY_LEGACY_HOOKS)?;
    uninstall_kiro_tool_hooks(
        &home_dir
            .join(".kiro")
            .join("agents")
            .join("wecode-managed.json"),
        KIRO_LEGACY_HOOKS,
    )?;
    Ok(())
}

/// Strip every wecode-managed hook entry (any wecode owner) from a Standard JSON
/// config, preserving the user's own hooks and the rest of the file. Never
/// creates the file if it is absent, and `write_json_object` skips the write when
/// nothing changed.
fn uninstall_tool_hooks(
    path: &Path,
    tool: &str,
    definitions: &[LegacyHookDefinition],
) -> Result<(), String> {
    if tool == "kiro" {
        return uninstall_kiro_tool_hooks(path, definitions);
    }
    if !path.exists() {
        return Ok(());
    }
    let mut root = load_json_object(path)?;
    let mut hooks = root
        .remove("hooks")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    for (event_key, action) in removed_hook_definitions(tool) {
        strip_managed_action_from_hooks(&mut hooks, event_key, action, None, Some(tool));
    }
    if tool == "claude" {
        strip_managed_action_from_hooks(
            &mut hooks,
            "Notification",
            "notification",
            None,
            Some("claude"),
        );
    }
    for definition in definitions {
        strip_managed_action_from_hooks(
            &mut hooks,
            definition.event_key,
            definition.action,
            None,
            Some(tool),
        );
    }

    if !hooks.is_empty() {
        root.insert("hooks".to_string(), Value::Object(hooks));
    }
    write_json_object(path, root)
}

/// Kiro's flat-array uninstall: entries sit directly under each event key, so
/// filter the wecode-managed ones out per entry. Leaves the inert
/// name/description/prompt agent fields (removing them risks clobbering a
/// user-authored agent).
fn uninstall_kiro_tool_hooks(
    path: &Path,
    definitions: &[LegacyHookDefinition],
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = load_json_object(path)?;
    let mut hooks = root
        .remove("hooks")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    for definition in definitions {
        let entries = hooks
            .remove(definition.event_key)
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let cleaned = entries
            .into_iter()
            .filter(|entry| !is_managed_hook_action(entry, definition.action, None, Some("kiro")))
            .collect::<Vec<_>>();
        if !cleaned.is_empty() {
            hooks.insert(definition.event_key.to_string(), Value::Array(cleaned));
        }
    }

    if !hooks.is_empty() {
        root.insert("hooks".to_string(), Value::Object(hooks));
    }
    write_json_object(path, root)
}

fn removed_hook_definitions(tool: &str) -> &'static [(&'static str, &'static str)] {
    match tool {
        "codex" => &[
            ("PreToolUse", "codex-pre-tool-use"),
            ("PostToolUse", "codex-post-tool-use"),
            ("SessionEnd", "codex-session-end"),
        ],
        "claude" => &[
            ("PreToolUse", "pre-tool-use"),
            ("PostToolUse", "post-tool-use"),
            ("PostToolUseFailure", "post-tool-use-failure"),
        ],
        _ => &[],
    }
}

fn strip_managed_action_from_hooks(
    hooks: &mut Map<String, Value>,
    event_key: &str,
    action: &str,
    owner: Option<&str>,
    tool: Option<&str>,
) {
    let groups = hooks
        .remove(event_key)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    if groups.is_empty() {
        return;
    }

    let mut cleaned_groups = Vec::new();
    for group in groups {
        let Some(group_object) = group.as_object() else {
            continue;
        };
        let next_hooks = group_object
            .get("hooks")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|item| !is_managed_hook_action(item, action, owner, tool))
            .collect::<Vec<_>>();
        if next_hooks.is_empty() {
            continue;
        }
        let mut next_group = group_object.clone();
        next_group.insert("hooks".to_string(), Value::Array(next_hooks));
        cleaned_groups.push(Value::Object(next_group));
    }

    if !cleaned_groups.is_empty() {
        hooks.insert(event_key.to_string(), Value::Array(cleaned_groups));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strip_managed_action_removes_all_wecode_owners_when_owner_is_unspecified() {
        let mut hooks = Map::new();
        hooks.insert(
            "Stop".to_string(),
            json!([
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "'/tmp/wecode/dmux-ai-state.sh' 'codex-stop' 'wecode' 'codex'"
                        },
                        {
                            "type": "command",
                            "command": "'/tmp/wecode-dev/dmux-ai-state.sh' 'codex-stop' 'wecode-dev' 'codex'"
                        },
                        {
                            "type": "command",
                            "command": "'/tmp/custom.sh' 'codex-stop' 'custom' 'codex'"
                        }
                    ]
                }
            ])
            .as_array()
            .cloned()
            .map(Value::Array)
            .unwrap(),
        );

        strip_managed_action_from_hooks(&mut hooks, "Stop", "codex-stop", None, Some("codex"));

        let remaining = hooks
            .get("Stop")
            .and_then(|value| value.as_array())
            .and_then(|groups| groups.first())
            .and_then(|group| group.get("hooks"))
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0]["command"].as_str().unwrap().contains("custom"));
    }

    #[test]
    fn uninstall_strips_legacy_kiro_agent_hooks() {
        let home = std::env::temp_dir().join(format!(
            "wecode-legacy-kiro-uninstall-{}",
            uuid::Uuid::new_v4()
        ));
        let path = home
            .join(".kiro")
            .join("agents")
            .join("wecode-managed.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            serde_json::json!({
                "name": "WeCode Managed",
                "hooks": {
                    "agentSpawn": [{
                        "command": "'/tmp/dmux-ai-state.sh' 'session-start' 'wecode' 'kiro'",
                        "timeout_ms": 5000,
                        "matcher": ""
                    }],
                    "userPromptSubmit": [{
                        "command": "'/tmp/dmux-ai-state.sh' 'prompt-submit' 'wecode' 'kiro'",
                        "timeout_ms": 5000,
                        "matcher": ""
                    }],
                    "stop": [{
                        "command": "'/tmp/custom.sh' 'stop' 'custom' 'kiro'",
                        "timeout_ms": 5000,
                        "matcher": ""
                    }]
                }
            })
            .to_string(),
        )
        .unwrap();

        uninstall_managed_hook_configs_in(&home).unwrap();

        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let hooks = value["hooks"].as_object().unwrap();
        assert!(!hooks.contains_key("agentSpawn"));
        assert!(!hooks.contains_key("userPromptSubmit"));
        assert_eq!(hooks["stop"].as_array().unwrap().len(), 1);
        assert!(
            hooks["stop"][0]["command"]
                .as_str()
                .unwrap()
                .contains("custom")
        );
        std::fs::remove_dir_all(home).unwrap();
    }
}
