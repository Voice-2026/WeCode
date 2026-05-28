use serde_json::Value;
use std::path::Path;

pub(super) fn is_managed_hook(value: &Value, action: &str, tool: &str) -> bool {
    is_managed_hook_action(value, action, Some(tool))
}

pub(super) fn is_managed_hook_action(value: &Value, action: &str, tool: Option<&str>) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(command) = object.get("command").and_then(|value| value.as_str()) else {
        return false;
    };
    if command.contains("dmux-ai-state.sh")
        && command.contains(&shell_quote(action))
        && tool
            .map(|tool| command.contains(&shell_quote(tool)))
            .unwrap_or(true)
    {
        return true;
    }
    #[cfg(windows)]
    {
        command.contains("dmux-ai-state.cmd")
            && command.contains(&windows_cmd_quote(action))
            && tool
                .map(|tool| command.contains(&windows_cmd_quote(tool)))
                .unwrap_or(true)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(super) fn hook_command(helper_script: &Path, action: &str, owner: &str, tool: &str) -> String {
    #[cfg(windows)]
    {
        return format!(
            "cmd /d /c call {} {} {} {}",
            windows_cmd_quote(&helper_script.with_extension("cmd").display().to_string()),
            windows_cmd_quote(action),
            windows_cmd_quote(owner),
            windows_cmd_quote(tool),
        );
    }

    #[cfg(not(windows))]
    [
        shell_quote(&helper_script.display().to_string()),
        shell_quote(action),
        shell_quote(owner),
        shell_quote(tool),
    ]
    .join(" ")
}

#[cfg(windows)]
fn windows_cmd_quote(value: &str) -> String {
    windows_cmd_quote_cross_platform(value)
}

pub(super) fn windows_cmd_quote_cross_platform(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(super) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
