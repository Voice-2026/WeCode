use serde::Serialize;
use serde_json::json;
use wecode_integrations::{AgentKind, IntegrationManager, IntegrationSnapshot};

const INTEGRATION_PROTOCOL_VERSION: &str = "1";
const EXIT_CONFIRMATION_REQUIRED: i32 = 7;
const EXIT_CONFLICT: i32 = 6;
const EXIT_INTERNAL: i32 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SuccessEnvelope<T> {
    ok: bool,
    request_id: String,
    protocol_version: &'static str,
    data: T,
}

pub fn status(json_output: bool) -> Result<(), String> {
    let manager = IntegrationManager::discover()?;
    print_snapshot(manager.snapshot(), manager.cli_status(), json_output)
}

pub fn install(agents: &[AgentKind], confirmed: bool, json_output: bool) -> Result<(), String> {
    require_confirmation(
        confirmed,
        json_output,
        "Installing integrations writes the wecode-control Skill into external Agent directories",
    );
    let manager = IntegrationManager::discover()?;
    let detected;
    let agents = if agents == AgentKind::ALL.as_slice() {
        detected = manager.detected_agents();
        if detected.is_empty() {
            exit_error(
                "AGENT_NOT_FOUND",
                "No supported Agent installation was detected".to_string(),
                json_output,
                EXIT_CONFLICT,
            );
        }
        detected.as_slice()
    } else {
        agents
    };
    match manager.install(agents) {
        Ok(snapshot) => print_snapshot(snapshot, manager.cli_status(), json_output),
        Err(error) => exit_integration_error("INTEGRATION_INSTALL_FAILED", error, json_output),
    }
}

pub fn update(confirmed: bool, json_output: bool) -> Result<(), String> {
    require_confirmation(
        confirmed,
        json_output,
        "Updating integrations replaces the WeCode-managed canonical Skill copy",
    );
    let manager = IntegrationManager::discover()?;
    match manager.update() {
        Ok(snapshot) => print_snapshot(snapshot, manager.cli_status(), json_output),
        Err(error) => exit_integration_error("INTEGRATION_UPDATE_FAILED", error, json_output),
    }
}

pub fn uninstall(agents: &[AgentKind], confirmed: bool, json_output: bool) -> Result<(), String> {
    require_confirmation(
        confirmed,
        json_output,
        "Uninstalling integrations removes WeCode-managed Skill links from external Agents",
    );
    let manager = IntegrationManager::discover()?;
    match manager.uninstall(agents) {
        Ok(snapshot) => print_snapshot(snapshot, manager.cli_status(), json_output),
        Err(error) => exit_integration_error("INTEGRATION_UNINSTALL_FAILED", error, json_output),
    }
}

fn require_confirmation(confirmed: bool, json_output: bool, message: &str) {
    if confirmed {
        return;
    }
    exit_error(
        "CONFIRMATION_REQUIRED",
        format!("{message}; pass --confirm to continue"),
        json_output,
        EXIT_CONFIRMATION_REQUIRED,
    );
}

fn print_snapshot(
    snapshot: IntegrationSnapshot,
    cli_status: wecode_integrations::CliStatus,
    json_output: bool,
) -> Result<(), String> {
    if json_output {
        let data = json!({
            "cli": cli_status,
            "skill": snapshot,
        });
        println!(
            "{}",
            serde_json::to_string(&SuccessEnvelope {
                ok: true,
                request_id: crate::cmd_app::uuid_request_id(),
                protocol_version: INTEGRATION_PROTOCOL_VERSION,
                data,
            })
            .map_err(|error| format!("encode integration status: {error}"))?
        );
        return Ok(());
    }

    println!(
        "CLI: {} ({})",
        if cli_status.installed {
            "installed"
        } else if cli_status.bundled {
            "bundled, not linked"
        } else {
            "unavailable"
        },
        cli_status.command_path.display()
    );
    println!(
        "Skill: {}{}",
        if snapshot.installed_fingerprint.is_some() {
            "installed"
        } else if snapshot.source_available {
            "bundled, not installed"
        } else {
            "unavailable"
        },
        if snapshot.update_available {
            ", update available"
        } else {
            ""
        }
    );
    for agent in snapshot.agents {
        println!(
            "{}: {} ({})",
            agent.display_name,
            if agent.installed {
                "installed"
            } else {
                "not installed"
            },
            agent.path.display()
        );
    }
    Ok(())
}

fn exit_integration_error(default_code: &str, message: String, json_output: bool) -> ! {
    let conflict =
        message.contains("refusing to replace") || message.contains("refusing to remove");
    let code = if conflict {
        "INTEGRATION_CONFLICT"
    } else {
        default_code
    };
    exit_error(
        code,
        message,
        json_output,
        if conflict {
            EXIT_CONFLICT
        } else {
            EXIT_INTERNAL
        },
    )
}

fn exit_error(code: &str, message: String, json_output: bool, exit_code: i32) -> ! {
    if json_output {
        println!(
            "{}",
            json!({
                "ok": false,
                "requestId": crate::cmd_app::uuid_request_id(),
                "protocolVersion": INTEGRATION_PROTOCOL_VERSION,
                "error": {
                    "code": code,
                    "message": message,
                }
            })
        );
    } else {
        eprintln!("error: {message}");
    }
    std::process::exit(exit_code);
}
