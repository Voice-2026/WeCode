//! `codux update` — check GitHub Releases for a newer build, then download,
//! verify, atomically replace this binary, and restart the host if it was up.

use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use serde::Deserialize;
use std::time::Duration;

use crate::{cmd_start, cmd_service, runstate};

const RELEASES_API: &str = "https://api.github.com/repos/duxweb/codux/releases/latest";
const USER_AGENT: &str = "codux-agent-updater";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn run(current: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?;

    println!("Checking for updates…");
    let release: Release = client
        .get(RELEASES_API)
        .send()
        .map_err(|error| format!("failed to reach GitHub: {error}"))?
        .error_for_status()
        .map_err(|error| format!("release lookup failed: {error}"))?
        .json()
        .map_err(|error| format!("invalid release payload: {error}"))?;

    let latest = release.tag_name.trim_start_matches('v');
    if !is_newer(latest, current) {
        println!("Already up to date (v{current}).");
        return Ok(());
    }
    println!("A newer version is available: v{latest} (current v{current}).");

    let asset = pick_asset(&release.assets)?;
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Download and install {} now?", asset.name))
        .default(true)
        .interact()
        .map_err(|error| error.to_string())?;
    if !proceed {
        println!("Update cancelled.");
        return Ok(());
    }

    let was_running = runstate::is_running();
    println!("Downloading {}…", asset.name);
    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .map_err(|error| format!("download failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("download failed: {error}"))?
        .bytes()
        .map_err(|error| error.to_string())?;
    if bytes.is_empty() {
        return Err("downloaded an empty file".to_string());
    }

    replace_self(&bytes)?;
    println!("Installed v{latest}.");

    if was_running {
        println!("Restarting the host…");
        let _ = cmd_service::stop();
        std::thread::sleep(Duration::from_millis(400));
        cmd_start::run(true)?;
    }
    Ok(())
}

/// Choose the release asset for this OS + architecture
/// (`codux-<os>-<arch>[.exe]`).
fn pick_asset(assets: &[Asset]) -> Result<&Asset, String> {
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    };
    let arch = std::env::consts::ARCH; // x86_64 / aarch64
    let prefix = format!("codux-{os}-{arch}");
    assets
        .iter()
        .find(|asset| asset.name.starts_with(&prefix))
        .ok_or_else(|| {
            let available = assets
                .iter()
                .map(|asset| asset.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("no release asset for {prefix} (available: {available})")
        })
}

/// Atomically replace the running executable with `bytes`.
fn replace_self(bytes: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let dir = exe.parent().ok_or("cannot resolve binary directory")?;
    let staged = dir.join(".codux.update.new");
    std::fs::write(&staged, bytes).map_err(|error| format!("failed to write update: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&staged)
            .map_err(|error| error.to_string())?
            .permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&staged, perms);
        // Renaming over the running binary is allowed on unix.
        std::fs::rename(&staged, &exe).map_err(|error| format!("failed to replace binary: {error}"))?;
    }

    #[cfg(windows)]
    {
        // A running .exe can't be overwritten, but it can be renamed aside.
        let old = dir.join(".codux.old.exe");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).map_err(|error| format!("failed to move current binary: {error}"))?;
        std::fs::rename(&staged, &exe).map_err(|error| format!("failed to install update: {error}"))?;
    }

    Ok(())
}

/// Numeric dotted-version comparison (`a` strictly newer than `b`).
fn is_newer(a: &str, b: &str) -> bool {
    let parse = |value: &str| -> Vec<u64> {
        value
            .split(['.', '-', '+'])
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (left, right) = (parse(a), parse(b));
    for index in 0..left.len().max(right.len()) {
        let l = left.get(index).copied().unwrap_or(0);
        let r = right.get(index).copied().unwrap_or(0);
        if l != r {
            return l > r;
        }
    }
    false
}
