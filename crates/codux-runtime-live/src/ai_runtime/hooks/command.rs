use serde_json::Value;

#[cfg(test)]
fn is_managed_hook(value: &Value, action: &str, owner: &str, tool: &str) -> bool {
    is_managed_hook_action(value, action, Some(owner), Some(tool))
}

pub(super) fn is_managed_hook_action(
    value: &Value,
    action: &str,
    owner: Option<&str>,
    tool: Option<&str>,
) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(command) = object.get("command").and_then(|value| value.as_str()) else {
        return false;
    };
    if command.contains("dmux-ai-state.sh")
        && command.contains(&shell_quote(action))
        && owner
            .map(|owner| command.contains(&shell_quote(owner)))
            .unwrap_or(true)
        && tool
            .map(|tool| command.contains(&shell_quote(tool)))
            .unwrap_or(true)
    {
        return true;
    }
    let is_current_windows = command.contains("dmux-ai-state.ps1")
        && command.contains(&windows_powershell_quote_cross_platform(action))
        && owner
            .map(|owner| command.contains(&windows_powershell_quote_cross_platform(owner)))
            .unwrap_or(true)
        && tool
            .map(|tool| command.contains(&windows_powershell_quote_cross_platform(tool)))
            .unwrap_or(true);
    let is_legacy_windows = command.contains("dmux-ai-state.cmd")
        && command.contains(&windows_cmd_quote_cross_platform(action))
        && owner
            .map(|owner| command.contains(&windows_cmd_quote_cross_platform(owner)))
            .unwrap_or(true)
        && tool
            .map(|tool| command.contains(&windows_cmd_quote_cross_platform(tool)))
            .unwrap_or(true);
    is_current_windows || is_legacy_windows
}

pub(super) fn windows_powershell_quote_cross_platform(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn windows_cmd_quote_cross_platform(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(super) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_only_matching_owner_for_shell_hooks() {
        let hook = json!({
            "command": "'/tmp/dmux-ai-state.sh' 'codex-stop' 'codux-dev' 'codex'"
        });

        assert!(is_managed_hook(&hook, "codex-stop", "codux-dev", "codex"));
        assert!(!is_managed_hook(&hook, "codex-stop", "codux", "codex"));
    }

    #[test]
    fn recognizes_windows_hooks_for_cross_platform_cleanup() {
        let current = json!({
            "command": "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File 'C:\\Codux\\dmux-ai-state.ps1' 'codex-session-start' 'codux' 'codex'"
        });
        let hook = json!({
            "command": "cmd /d /c call \"C:\\Codux\\dmux-ai-state.cmd\" \"codex-session-start\" \"codux\" \"codex\""
        });

        assert!(is_managed_hook(
            &current,
            "codex-session-start",
            "codux",
            "codex"
        ));
        assert!(is_managed_hook(
            &hook,
            "codex-session-start",
            "codux",
            "codex"
        ));
        assert!(!is_managed_hook(
            &hook,
            "codex-session-start",
            "codux-dev",
            "codex"
        ));
    }
}
