use std::path::PathBuf;

pub fn app_support_dir() -> PathBuf {
    app_support_candidates()
        .into_iter()
        .find(|path| path.join("state.json").is_file() || path.join("settings.json").is_file())
        .unwrap_or_else(default_app_support_dir)
}

pub fn runtime_temp_dir() -> PathBuf {
    std::env::temp_dir().join(app_slug())
}

pub fn runtime_log_path() -> PathBuf {
    app_support_dir().join("runtime.log")
}

pub fn live_log_path() -> PathBuf {
    runtime_temp_dir().join("live.log")
}

pub fn app_display_name() -> &'static str {
    if cfg!(debug_assertions) {
        "Codux Dev"
    } else {
        "Codux"
    }
}

pub fn app_slug() -> &'static str {
    if cfg!(debug_assertions) {
        "codux-dev"
    } else {
        "codux"
    }
}

pub fn app_support_candidates() -> Vec<PathBuf> {
    let home = home_dir();

    #[cfg(target_os = "macos")]
    {
        return vec![
            home.join("Library/Application Support/Codux"),
            home.join("Library/Application Support/Codux Dev"),
            home.join("Library/Application Support/Codux-dev"),
        ];
    }

    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"));
        return vec![
            base.join("Codux"),
            base.join("Codux Dev"),
            base.join("codux-dev"),
        ];
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        vec![base.join("codux"), base.join("codux-dev")]
    }
}

pub fn default_app_support_dir() -> PathBuf {
    let mut candidates = app_support_candidates();
    candidates
        .drain(..)
        .next()
        .unwrap_or_else(|| home_dir().join(".codux"))
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(windows_user_profile)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "windows")]
fn windows_user_profile() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let drive = std::env::var_os("HOMEDRIVE")?;
            let path = std::env::var_os("HOMEPATH")?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

#[cfg(not(target_os = "windows"))]
fn windows_user_profile() -> Option<PathBuf> {
    None
}
