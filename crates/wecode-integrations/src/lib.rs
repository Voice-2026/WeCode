use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

pub const CONTROL_SKILL_NAME: &str = "wecode-control";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    Codex,
    ClaudeCode,
    Kiro,
}

impl AgentKind {
    pub const ALL: [Self; 3] = [Self::Codex, Self::ClaudeCode, Self::Kiro];

    pub fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Kiro => "kiro",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Kiro => "Kiro",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentIntegrationStatus {
    pub agent: AgentKind,
    pub display_name: String,
    pub detected: bool,
    pub installed: bool,
    pub managed: bool,
    pub path: PathBuf,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationSnapshot {
    pub source_available: bool,
    pub source_path: PathBuf,
    pub canonical_path: PathBuf,
    pub source_fingerprint: Option<String>,
    pub installed_fingerprint: Option<String>,
    pub update_available: bool,
    pub agents: Vec<AgentIntegrationStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub bundled: bool,
    pub bundled_path: PathBuf,
    pub command_path: PathBuf,
    pub installed: bool,
    pub managed: bool,
}

#[derive(Clone, Debug)]
pub struct IntegrationManager {
    source_dir: PathBuf,
    canonical_dir: PathBuf,
    cli_source: PathBuf,
    cli_target: PathBuf,
    codex_root: PathBuf,
    claude_root: PathBuf,
    kiro_root: PathBuf,
}

impl IntegrationManager {
    pub fn discover() -> Result<Self, String> {
        let home_dir = home_dir()?;
        let invoked_exe = env::current_exe().unwrap_or_default();
        let resolved_exe = fs::canonicalize(&invoked_exe).unwrap_or_else(|_| invoked_exe.clone());
        let source_dir = env::var_os("WECODE_CONTROL_SKILL_SOURCE")
            .map(PathBuf::from)
            .unwrap_or_else(|| discover_bundled_skill_source(&resolved_exe));
        let support_root = env::var_os("WECODE_INTEGRATION_SUPPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_support_root(&home_dir));
        let cli_source = env::var_os("WECODE_BUNDLED_CLI_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| discover_bundled_cli(&resolved_exe));
        let cli_target = env::var_os("WECODE_CLI_INSTALL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_cli_target(&invoked_exe, &cli_source));
        let mut manager = Self::new(
            source_dir,
            support_root.join("skills").join(CONTROL_SKILL_NAME),
            home_dir.clone(),
            cli_source,
            cli_target,
        );
        manager.codex_root = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir.join(".codex"));
        manager.claude_root = env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir.join(".claude"));
        manager.kiro_root = env::var_os("KIRO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir.join(".kiro"));
        Ok(manager)
    }

    pub fn new(
        source_dir: PathBuf,
        canonical_dir: PathBuf,
        home_dir: PathBuf,
        cli_source: PathBuf,
        cli_target: PathBuf,
    ) -> Self {
        Self {
            source_dir,
            canonical_dir,
            cli_source,
            cli_target,
            codex_root: home_dir.join(".codex"),
            claude_root: home_dir.join(".claude"),
            kiro_root: home_dir.join(".kiro"),
        }
    }

    pub fn snapshot(&self) -> IntegrationSnapshot {
        let source_fingerprint = skill_fingerprint(&self.source_dir).ok();
        let installed_fingerprint = skill_fingerprint(&self.canonical_dir).ok();
        let agents = AgentKind::ALL
            .into_iter()
            .map(|agent| self.agent_status(agent))
            .collect();
        IntegrationSnapshot {
            source_available: self.source_dir.join("SKILL.md").is_file(),
            source_path: self.source_dir.clone(),
            canonical_path: self.canonical_dir.clone(),
            update_available: source_fingerprint.is_some()
                && installed_fingerprint.is_some()
                && source_fingerprint != installed_fingerprint,
            source_fingerprint,
            installed_fingerprint,
            agents,
        }
    }

    pub fn cli_status(&self) -> CliStatus {
        let managed = symlink_points_to(&self.cli_target, &self.cli_source);
        CliStatus {
            bundled: self.cli_source.is_file(),
            bundled_path: self.cli_source.clone(),
            command_path: self.cli_target.clone(),
            installed: self.cli_target.exists() || self.cli_target.is_symlink(),
            managed,
        }
    }

    pub fn detected_agents(&self) -> Vec<AgentKind> {
        AgentKind::ALL
            .into_iter()
            .filter(|agent| self.agent_root(*agent).exists())
            .collect()
    }

    pub fn should_offer_setup(&self) -> bool {
        let snapshot = self.snapshot();
        snapshot.source_available
            && snapshot.agents.iter().any(|agent| agent.detected)
            && snapshot.agents.iter().all(|agent| !agent.managed)
            && !self.setup_marker_path().is_file()
    }

    pub fn mark_setup_offered(&self) -> Result<(), String> {
        let marker = self.setup_marker_path();
        let parent = marker
            .parent()
            .ok_or_else(|| "integration setup marker has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
        fs::write(&marker, b"1\n").map_err(|error| format!("write {}: {error}", marker.display()))
    }

    pub fn install_cli(&self) -> Result<CliStatus, String> {
        if !self.cli_source.is_file() {
            return Err(format!(
                "bundled CLI not found at {}",
                self.cli_source.display()
            ));
        }
        if self.cli_target.exists() || self.cli_target.is_symlink() {
            if symlink_points_to(&self.cli_target, &self.cli_source) {
                return Ok(self.cli_status());
            }
            return Err(format!(
                "refusing to replace existing command at {}",
                self.cli_target.display()
            ));
        }
        let parent = self
            .cli_target
            .parent()
            .ok_or_else(|| "CLI install path has no parent".to_string())?;
        let direct = fs::create_dir_all(parent)
            .and_then(|_| create_file_symlink(&self.cli_source, &self.cli_target));
        if let Err(error) = direct {
            if error.kind() != io::ErrorKind::PermissionDenied {
                return Err(format!(
                    "link {} to {}: {error}",
                    self.cli_target.display(),
                    self.cli_source.display()
                ));
            }
            install_cli_with_privileges(&self.cli_source, &self.cli_target)?;
        }
        Ok(self.cli_status())
    }

    pub fn install(&self, agents: &[AgentKind]) -> Result<IntegrationSnapshot, String> {
        validate_skill_dir(&self.source_dir)?;
        for agent in agents {
            self.validate_agent_target(*agent)?;
        }
        replace_directory(&self.source_dir, &self.canonical_dir)?;
        for agent in agents {
            self.link_agent(*agent)?;
        }
        Ok(self.snapshot())
    }

    pub fn update(&self) -> Result<IntegrationSnapshot, String> {
        validate_skill_dir(&self.source_dir)?;
        if !self.canonical_dir.join("SKILL.md").is_file() {
            return Err("wecode-control is not installed yet".to_string());
        }
        replace_directory(&self.source_dir, &self.canonical_dir)?;
        Ok(self.snapshot())
    }

    pub fn uninstall(&self, agents: &[AgentKind]) -> Result<IntegrationSnapshot, String> {
        for agent in agents {
            let path = self.agent_skill_path(*agent);
            if path.is_symlink() && symlink_points_to(&path, &self.canonical_dir) {
                fs::remove_file(&path)
                    .map_err(|error| format!("remove {}: {error}", path.display()))?;
            } else if path.exists() {
                return Err(format!(
                    "refusing to remove unmanaged skill at {}",
                    path.display()
                ));
            }
        }
        if AgentKind::ALL
            .into_iter()
            .all(|agent| !symlink_points_to(&self.agent_skill_path(agent), &self.canonical_dir))
            && self.canonical_dir.exists()
        {
            fs::remove_dir_all(&self.canonical_dir)
                .map_err(|error| format!("remove {}: {error}", self.canonical_dir.display()))?;
        }
        Ok(self.snapshot())
    }

    fn agent_status(&self, agent: AgentKind) -> AgentIntegrationStatus {
        let root = self.agent_root(agent);
        let path = root.join("skills").join(CONTROL_SKILL_NAME);
        let managed = symlink_points_to(&path, &self.canonical_dir);
        let installed = path.join("SKILL.md").is_file();
        let detail = if managed {
            "Managed by WeCode".to_string()
        } else if installed {
            "Existing unmanaged installation".to_string()
        } else if path.is_symlink() {
            "Broken or unrelated symlink".to_string()
        } else {
            "Not installed".to_string()
        };
        AgentIntegrationStatus {
            agent,
            display_name: agent.display_name().to_string(),
            detected: root.exists(),
            installed,
            managed,
            path,
            detail,
        }
    }

    fn link_agent(&self, agent: AgentKind) -> Result<(), String> {
        let path = self.agent_skill_path(agent);
        if path.is_symlink() && symlink_points_to(&path, &self.canonical_dir) {
            return Ok(());
        }
        if path.exists() || path.is_symlink() {
            return Err(format!(
                "refusing to replace existing skill at {}",
                path.display()
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| "agent skill path has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
        create_dir_symlink(&self.canonical_dir, &path).map_err(|error| {
            format!(
                "link {} to {}: {error}",
                path.display(),
                self.canonical_dir.display()
            )
        })
    }

    fn validate_agent_target(&self, agent: AgentKind) -> Result<(), String> {
        let path = self.agent_skill_path(agent);
        if path.is_symlink() && symlink_points_to(&path, &self.canonical_dir) {
            return Ok(());
        }
        if path.exists() || path.is_symlink() {
            return Err(format!(
                "refusing to replace existing skill at {}",
                path.display()
            ));
        }
        Ok(())
    }

    fn agent_skill_path(&self, agent: AgentKind) -> PathBuf {
        self.agent_root(agent)
            .join("skills")
            .join(CONTROL_SKILL_NAME)
    }

    fn agent_root(&self, agent: AgentKind) -> PathBuf {
        match agent {
            AgentKind::Codex => self.codex_root.clone(),
            AgentKind::ClaudeCode => self.claude_root.clone(),
            AgentKind::Kiro => self.kiro_root.clone(),
        }
    }

    fn setup_marker_path(&self) -> PathBuf {
        self.canonical_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new(""))
            .join("setup-v1-offered")
    }
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn default_support_root(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/WeCode/integrations")
    } else {
        home.join(".local/share/wecode/integrations")
    }
}

fn discover_bundled_skill_source(exe: &Path) -> PathBuf {
    let exe_dir = exe.parent().unwrap_or_else(|| Path::new(""));
    let candidates = [
        exe_dir.join("../Resources/skills").join(CONTROL_SKILL_NAME),
        exe_dir.join("../skills").join(CONTROL_SKILL_NAME),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../skills")
            .join(CONTROL_SKILL_NAME),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("SKILL.md").is_file())
        .map(canonicalize_if_possible)
        .unwrap_or_else(|| exe_dir.join("../Resources/skills").join(CONTROL_SKILL_NAME))
}

fn discover_bundled_cli(exe: &Path) -> PathBuf {
    let exe_dir = exe.parent().unwrap_or_else(|| Path::new(""));
    let candidates = [
        exe_dir.join("../Resources/bin/wecode"),
        exe_dir.join("../bin/wecode"),
        exe.to_path_buf(),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(canonicalize_if_possible)
        .unwrap_or_else(|| exe_dir.join("../Resources/bin/wecode"))
}

fn default_cli_target(invoked_exe: &Path, cli_source: &Path) -> PathBuf {
    if invoked_exe.is_symlink() && symlink_points_to(invoked_exe, cli_source) {
        invoked_exe.to_path_buf()
    } else {
        PathBuf::from("/usr/local/bin/wecode")
    }
}

fn canonicalize_if_possible(path: PathBuf) -> PathBuf {
    fs::canonicalize(&path).unwrap_or(path)
}

fn validate_skill_dir(path: &Path) -> Result<(), String> {
    if !path.join("SKILL.md").is_file() {
        return Err(format!("SKILL.md not found in {}", path.display()));
    }
    Ok(())
}

fn replace_directory(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "canonical skill path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let stage = parent.join(format!(
        ".{CONTROL_SKILL_NAME}.stage-{}",
        std::process::id()
    ));
    let backup = parent.join(format!(
        ".{CONTROL_SKILL_NAME}.backup-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&stage);
    let _ = fs::remove_dir_all(&backup);
    copy_directory(source, &stage)?;
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|error| {
            format!("move existing {} to backup: {error}", destination.display())
        })?;
    }
    if let Err(error) = fs::rename(&stage, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(format!("activate {}: {error}", destination.display()));
    }
    let _ = fs::remove_dir_all(backup);
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("create {}: {error}", destination.display()))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("read {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", entry.path().display()))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|error| format!("copy {}: {error}", entry.path().display()))?;
        } else {
            return Err(format!(
                "unsupported skill entry {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn skill_fingerprint(path: &Path) -> Result<String, String> {
    validate_skill_dir(path)?;
    let mut hasher = Sha256::new();
    hash_directory(path, path, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_directory(root: &Path, current: &Path, hasher: &mut Sha256) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("read {}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read {}: {error}", current.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
        hasher.update(relative.to_string_lossy().as_bytes());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            hash_directory(root, &path, hasher)?;
        } else if file_type.is_file() {
            let bytes =
                fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
            hasher.update(&bytes);
        } else {
            return Err(format!("unsupported skill entry {}", path.display()));
        }
    }
    Ok(())
}

fn symlink_points_to(link: &Path, expected: &Path) -> bool {
    let Ok(target) = fs::read_link(link) else {
        return false;
    };
    let resolved = if target.is_absolute() {
        target
    } else {
        link.parent().unwrap_or_else(|| Path::new("")).join(target)
    };
    normalize_path(&resolved) == normalize_path(expected)
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(unix)]
fn create_dir_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(unix)]
fn create_file_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(not(unix))]
fn create_dir_symlink(_source: &Path, _target: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "WeCode integrations currently support macOS and Linux",
    ))
}

#[cfg(not(unix))]
fn create_file_symlink(_source: &Path, _target: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "WeCode CLI installation currently supports macOS and Linux",
    ))
}

#[cfg(target_os = "macos")]
fn install_cli_with_privileges(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "CLI install path has no parent".to_string())?;
    let command = format!(
        "mkdir -p {} && ln -s {} {}",
        shell_quote(parent),
        shell_quote(source),
        shell_quote(target)
    );
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        command.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let output = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| format!("start macOS administrator prompt: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("User canceled") || stderr.contains("-128") {
        return Err("CLI installation was cancelled".to_string());
    }
    Err(format!(
        "install CLI with administrator privileges: {}",
        stderr.trim()
    ))
}

#[cfg(not(target_os = "macos"))]
fn install_cli_with_privileges(_source: &Path, _target: &Path) -> Result<(), String> {
    Err("CLI install path is not writable".to_string())
}

#[cfg(target_os = "macos")]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> (PathBuf, IntegrationManager) {
        let root = env::temp_dir().join(format!(
            "wecode-integrations-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source/wecode-control");
        fs::create_dir_all(source.join("references")).unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: wecode-control\n---\n").unwrap();
        fs::write(source.join("references/cli.md"), "v1\n").unwrap();
        let cli_source = root.join("app/Resources/bin/wecode");
        fs::create_dir_all(cli_source.parent().unwrap()).unwrap();
        fs::write(&cli_source, "cli").unwrap();
        let manager = IntegrationManager::new(
            source,
            root.join("support/skills/wecode-control"),
            root.join("home"),
            cli_source,
            root.join("bin/wecode"),
        );
        (root, manager)
    }

    #[test]
    fn installs_one_canonical_skill_and_agent_links() {
        let (root, manager) = fixture();
        let snapshot = manager.install(&AgentKind::ALL).unwrap();
        assert!(snapshot.canonical_path.join("SKILL.md").is_file());
        assert!(snapshot.agents.iter().all(|agent| agent.managed));
        assert!(snapshot.agents.iter().all(|agent| agent.installed));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn update_replaces_canonical_content_without_relinking() {
        let (root, manager) = fixture();
        manager.install(&[AgentKind::Codex]).unwrap();
        fs::write(manager.source_dir.join("references/cli.md"), "v2\n").unwrap();
        assert!(manager.snapshot().update_available);
        let updated = manager.update().unwrap();
        assert!(!updated.update_available);
        assert_eq!(
            fs::read_to_string(manager.canonical_dir.join("references/cli.md")).unwrap(),
            "v2\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_replace_an_unmanaged_skill() {
        let (root, manager) = fixture();
        let unmanaged = manager.agent_skill_path(AgentKind::Codex);
        fs::create_dir_all(&unmanaged).unwrap();
        fs::write(unmanaged.join("SKILL.md"), "existing").unwrap();
        let error = manager.install(&[AgentKind::Codex]).unwrap_err();
        assert!(error.contains("refusing to replace existing skill"));
        assert_eq!(
            fs::read_to_string(unmanaged.join("SKILL.md")).unwrap(),
            "existing"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn installs_cli_without_replacing_existing_commands() {
        let (root, manager) = fixture();
        assert!(manager.install_cli().unwrap().managed);
        assert!(manager.cli_target.is_symlink());
        fs::remove_file(&manager.cli_target).unwrap();
        fs::write(&manager.cli_target, "other").unwrap();
        assert!(manager.install_cli().is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn uninstall_only_removes_managed_links() {
        let (root, manager) = fixture();
        manager.install(&[AgentKind::Codex]).unwrap();
        let snapshot = manager.uninstall(&[AgentKind::Codex]).unwrap();
        assert!(!snapshot.canonical_path.exists());
        assert!(!snapshot.agents[0].installed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn setup_offer_is_recorded_once_for_a_detected_agent() {
        let (root, manager) = fixture();
        fs::create_dir_all(&manager.codex_root).unwrap();
        assert!(manager.should_offer_setup());
        manager.mark_setup_offered().unwrap();
        assert!(!manager.should_offer_setup());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_bundle_resources_and_cli_status_through_homebrew_symlink() {
        let root = env::temp_dir().join(format!(
            "wecode-homebrew-integration-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let resources = root.join("Applications/WeCode.app/Contents/Resources");
        let cli_source = resources.join("bin/wecode");
        let skill_source = resources.join("skills/wecode-control");
        fs::create_dir_all(&skill_source).unwrap();
        fs::create_dir_all(cli_source.parent().unwrap()).unwrap();
        fs::write(&cli_source, "cli").unwrap();
        fs::write(
            skill_source.join("SKILL.md"),
            "---\nname: wecode-control\n---\n",
        )
        .unwrap();
        let command_path = root.join("opt/homebrew/bin/wecode");
        fs::create_dir_all(command_path.parent().unwrap()).unwrap();
        create_file_symlink(&cli_source, &command_path).unwrap();

        let resolved_exe = fs::canonicalize(&command_path).unwrap();
        let discovered_skill = discover_bundled_skill_source(&resolved_exe);
        let discovered_cli = discover_bundled_cli(&resolved_exe);
        let cli_target = default_cli_target(&command_path, &discovered_cli);
        let manager = IntegrationManager::new(
            discovered_skill,
            root.join("support/skills/wecode-control"),
            root.join("home"),
            discovered_cli,
            cli_target,
        );

        assert!(manager.snapshot().source_available);
        let status = manager.cli_status();
        assert!(status.bundled);
        assert!(status.installed);
        assert!(status.managed);
        assert_eq!(status.command_path, command_path);
        assert_eq!(status.bundled_path, fs::canonicalize(cli_source).unwrap());
        let _ = fs::remove_dir_all(root);
    }
}
