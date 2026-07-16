#![cfg(unix)]

use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn integration_cli_installs_updates_and_uninstalls_managed_skill() {
    let fixture = Fixture::new();

    let status = fixture.run(&["integration", "status", "--json"]);
    assert_success_json(&status);

    let missing_confirmation =
        fixture.run(&["integration", "install", "--agent", "codex", "--json"]);
    assert_eq!(missing_confirmation.status.code(), Some(7));
    assert_eq!(
        json(&missing_confirmation)["error"]["code"],
        "CONFIRMATION_REQUIRED"
    );

    let installed = fixture.run(&[
        "integration",
        "install",
        "--agent",
        "codex",
        "--confirm",
        "--json",
    ]);
    let installed_json = assert_success_json(&installed);
    assert_eq!(
        installed_json["data"]["skill"]["agents"][0]["managed"],
        true
    );
    let codex_skill = fixture.home.join(".codex/skills/wecode-control");
    assert!(codex_skill.is_symlink());
    assert!(codex_skill.join("SKILL.md").is_file());

    fs::write(fixture.source.join("references/cli-contract.md"), "v2\n").unwrap();
    let before_update = fixture.run(&["integration", "status", "--json"]);
    assert_eq!(
        json(&before_update)["data"]["skill"]["updateAvailable"],
        true
    );
    let updated = fixture.run(&["integration", "update", "--confirm", "--json"]);
    assert_eq!(
        assert_success_json(&updated)["data"]["skill"]["updateAvailable"],
        false
    );

    let uninstalled = fixture.run(&[
        "integration",
        "uninstall",
        "--agent",
        "codex",
        "--confirm",
        "--json",
    ]);
    assert_success_json(&uninstalled);
    assert!(!codex_skill.exists());

    let _ = fs::remove_dir_all(&fixture.root);
}

#[test]
fn integration_cli_refuses_unmanaged_skill_with_conflict_exit() {
    let fixture = Fixture::new();
    let skill = fixture.home.join(".codex/skills/wecode-control");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "unmanaged\n").unwrap();

    let output = fixture.run(&[
        "integration",
        "install",
        "--agent",
        "codex",
        "--confirm",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(json(&output)["error"]["code"], "INTEGRATION_CONFLICT");
    assert_eq!(
        fs::read_to_string(skill.join("SKILL.md")).unwrap(),
        "unmanaged\n"
    );

    let _ = fs::remove_dir_all(&fixture.root);
}

#[test]
fn integration_cli_all_only_installs_detected_agents() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".codex")).unwrap();

    let output = fixture.run(&[
        "integration",
        "install",
        "--agent",
        "all",
        "--confirm",
        "--json",
    ]);
    assert_success_json(&output);
    assert!(
        fixture
            .home
            .join(".codex/skills/wecode-control")
            .is_symlink()
    );
    assert!(!fixture.home.join(".claude").exists());
    assert!(!fixture.home.join(".kiro").exists());

    let _ = fs::remove_dir_all(&fixture.root);
}

fn assert_success_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let value = json(output);
    assert_eq!(value["ok"], true);
    value
}

fn json(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 1);
    serde_json::from_slice(&output.stdout).unwrap()
}

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    source: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = Path::new("/tmp").join(format!(
            "wecode-integration-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = root.join("home");
        let source = root.join("bundle/skills/wecode-control");
        fs::create_dir_all(source.join("references")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: wecode-control\ndescription: test\n---\n",
        )
        .unwrap();
        fs::write(source.join("references/cli-contract.md"), "v1\n").unwrap();
        Self { root, home, source }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wecode-agent"))
            .args(args)
            .env("HOME", &self.home)
            .env("CODEX_HOME", self.home.join(".codex"))
            .env("CLAUDE_CONFIG_DIR", self.home.join(".claude"))
            .env("KIRO_HOME", self.home.join(".kiro"))
            .env("WECODE_CONTROL_SKILL_SOURCE", &self.source)
            .env("WECODE_INTEGRATION_SUPPORT_DIR", self.root.join("support"))
            .env(
                "WECODE_BUNDLED_CLI_PATH",
                env!("CARGO_BIN_EXE_wecode-agent"),
            )
            .env("WECODE_CLI_INSTALL_PATH", self.root.join("bin/wecode"))
            .output()
            .unwrap()
    }
}
