use std::{fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const RUNTIME_ASSETS: &[(&str, &[u8])] = &[
    (
        "scripts/shell-hooks/dmux-ai-hook.zsh",
        include_bytes!("../../../runtime-assets/scripts/shell-hooks/dmux-ai-hook.zsh"),
    ),
    (
        "scripts/shell-hooks/zsh/.zlogin",
        include_bytes!("../../../runtime-assets/scripts/shell-hooks/zsh/.zlogin"),
    ),
    (
        "scripts/shell-hooks/zsh/.zprofile",
        include_bytes!("../../../runtime-assets/scripts/shell-hooks/zsh/.zprofile"),
    ),
    (
        "scripts/shell-hooks/zsh/.zshenv",
        include_bytes!("../../../runtime-assets/scripts/shell-hooks/zsh/.zshenv"),
    ),
    (
        "scripts/shell-hooks/zsh/.zshrc",
        include_bytes!("../../../runtime-assets/scripts/shell-hooks/zsh/.zshrc"),
    ),
    (
        "scripts/wrappers/bin/agy",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/agy"),
    ),
    (
        "scripts/wrappers/bin/agy.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/agy.cmd"),
    ),
    (
        "scripts/wrappers/bin/claude",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/claude"),
    ),
    (
        "scripts/wrappers/bin/claude-code",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/claude-code"),
    ),
    (
        "scripts/wrappers/bin/claude-code.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/claude-code.cmd"),
    ),
    (
        "scripts/wrappers/bin/claude.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/claude.cmd"),
    ),
    (
        "scripts/wrappers/bin/codex",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/codex"),
    ),
    (
        "scripts/wrappers/bin/codex.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/codex.cmd"),
    ),
    (
        "scripts/wrappers/bin/codux-ssh",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/codux-ssh"),
    ),
    (
        "scripts/wrappers/bin/codux-ssh.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/codux-ssh.cmd"),
    ),
    (
        "scripts/wrappers/bin/gemini",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/gemini"),
    ),
    (
        "scripts/wrappers/bin/gemini.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/gemini.cmd"),
    ),
    (
        "scripts/wrappers/bin/kiro",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/kiro"),
    ),
    (
        "scripts/wrappers/bin/kiro-cli",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/kiro-cli"),
    ),
    (
        "scripts/wrappers/bin/kiro-cli.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/kiro-cli.cmd"),
    ),
    (
        "scripts/wrappers/bin/kiro.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/kiro.cmd"),
    ),
    (
        "scripts/wrappers/bin/opencode",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/opencode"),
    ),
    (
        "scripts/wrappers/bin/opencode.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/bin/opencode.cmd"),
    ),
    (
        "scripts/wrappers/codux-ssh-expect.exp",
        include_bytes!("../../../runtime-assets/scripts/wrappers/codux-ssh-expect.exp"),
    ),
    (
        "scripts/wrappers/codux-ssh.ps1",
        include_bytes!("../../../runtime-assets/scripts/wrappers/codux-ssh.ps1"),
    ),
    (
        "scripts/wrappers/dmux-ai-state.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/dmux-ai-state.cmd"),
    ),
    (
        "scripts/wrappers/dmux-ai-state.ps1",
        include_bytes!("../../../runtime-assets/scripts/wrappers/dmux-ai-state.ps1"),
    ),
    (
        "scripts/wrappers/dmux-ai-state.sh",
        include_bytes!("../../../runtime-assets/scripts/wrappers/dmux-ai-state.sh"),
    ),
    (
        "scripts/wrappers/opencode-config/package.json",
        include_bytes!("../../../runtime-assets/scripts/wrappers/opencode-config/package.json"),
    ),
    (
        "scripts/wrappers/opencode-config/plugins/dmux-runtime.js",
        include_bytes!(
            "../../../runtime-assets/scripts/wrappers/opencode-config/plugins/dmux-runtime.js"
        ),
    ),
    (
        "scripts/wrappers/tool-wrapper.cmd",
        include_bytes!("../../../runtime-assets/scripts/wrappers/tool-wrapper.cmd"),
    ),
    (
        "scripts/wrappers/tool-wrapper.ps1",
        include_bytes!("../../../runtime-assets/scripts/wrappers/tool-wrapper.ps1"),
    ),
    (
        "scripts/wrappers/tool-wrapper.sh",
        include_bytes!("../../../runtime-assets/scripts/wrappers/tool-wrapper.sh"),
    ),
];

pub fn runtime_asset_content(relative_path: &str) -> Option<&'static [u8]> {
    RUNTIME_ASSETS
        .iter()
        .find_map(|(path, content)| (*path == relative_path).then_some(*content))
}

pub fn stage_runtime_asset(
    relative_path: &str,
    destination: &Path,
    executable: bool,
) -> Result<(), String> {
    #[cfg(not(unix))]
    let _ = executable;

    let content = runtime_asset_content(relative_path)
        .ok_or_else(|| format!("runtime asset {relative_path} was not embedded"))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let should_write = match fs::read(destination) {
        Ok(existing) => existing != content,
        Err(_) => true,
    };

    if should_write {
        fs::write(destination, content).map_err(|error| error.to_string())?;
    }

    #[cfg(unix)]
    if executable {
        let permissions = fs::Permissions::from_mode(0o755);
        let _ = fs::set_permissions(destination, permissions);
    }

    Ok(())
}

pub fn stage_runtime_dir(relative_path: &str, destination: &Path) -> Result<(), String> {
    let prefix = format!("{}/", relative_path.trim_end_matches('/'));
    let mut staged = 0usize;
    for (asset_path, _) in RUNTIME_ASSETS {
        let Some(child_path) = asset_path.strip_prefix(&prefix) else {
            continue;
        };
        stage_runtime_asset(asset_path, &destination.join(child_path), false)?;
        staged += 1;
    }
    if staged == 0 {
        return Err(format!(
            "runtime asset directory {relative_path} was not embedded"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn stages_runtime_asset_and_nested_directory() {
        let dir = std::env::temp_dir().join(format!("codux-runtime-assets-{}", Uuid::new_v4()));
        let script = dir.join("dmux-ai-state.sh");
        stage_runtime_asset("scripts/wrappers/dmux-ai-state.sh", &script, true).unwrap();
        assert!(script.is_file());
        assert!(!fs::read(&script).unwrap().is_empty());

        let config_dir = dir.join("opencode-config");
        stage_runtime_dir("scripts/wrappers/opencode-config", &config_dir).unwrap();
        assert!(config_dir.join("package.json").is_file());
        assert!(config_dir.join("plugins/dmux-runtime.js").is_file());

        fs::remove_dir_all(dir).unwrap();
    }
}
