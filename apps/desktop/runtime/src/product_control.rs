use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    ai_history::AISessionSummary, project_store::ProjectSummary,
    runtime_bridge::staged_runtime_root_path, runtime_state::RuntimeService,
    terminal_pty::TerminalPtyConfig, tool_permissions::ToolPermissionsSummary,
};

const MAX_PROMPT_CHARS: usize = 32 * 1024;
const SESSION_ENTER_DELAY: Duration = Duration::from_millis(30);
const MAX_TERMINAL_INPUT_CHARS: usize = 32 * 1024;
const DEFAULT_TERMINAL_SNAPSHOT_CHARS: usize = 8 * 1024;
const MAX_TERMINAL_SNAPSHOT_CHARS: usize = 64 * 1024;
const MAX_TERMINAL_SNAPSHOT_SOURCE_CHARS: usize = 256 * 1024;
const MAX_TERMINAL_COMMAND_CHARS: usize = 8 * 1024;
const AGENT_VERSION_TIMEOUT: Duration = Duration::from_millis(1500);
const AGENT_VERSION_OUTPUT_BYTES: usize = 4 * 1024;
const AGENT_VERSION_MAX_CHARS: usize = 160;

#[derive(Debug)]
pub struct ProductControlError {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
}

impl ProductControlError {
    fn new(code: &'static str, message: impl Into<String>, details: Value) -> Self {
        Self {
            code,
            message: message.into(),
            details,
        }
    }
}

#[derive(Clone, Copy)]
struct AgentSpec {
    id: &'static str,
    name: &'static str,
    command: &'static str,
    aliases: &'static [&'static str],
    supports_model: bool,
    supports_resume: bool,
    supports_full_access: bool,
}

const AGENTS: [AgentSpec; 10] = [
    AgentSpec {
        id: "codex",
        name: "Codex",
        command: "codex",
        aliases: &["codex"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "claude",
        name: "Claude Code",
        command: "claude",
        aliases: &["claude"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "kiro-claude",
        name: "Kiro Claude",
        command: "claude",
        aliases: &["claude"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "kiro-codex",
        name: "Kiro Codex",
        command: "codex",
        aliases: &["codex"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "agy",
        name: "Agy",
        command: "agy",
        aliases: &["agy"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "opencode",
        name: "OpenCode",
        command: "opencode",
        aliases: &["opencode"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "kiro",
        name: "Kiro",
        command: "kiro-cli",
        aliases: &["kiro-cli"],
        supports_model: false,
        supports_resume: true,
        supports_full_access: false,
    },
    AgentSpec {
        id: "codewhale",
        name: "CodeWhale",
        command: "codewhale",
        aliases: &["codewhale"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
    AgentSpec {
        id: "kimi",
        name: "Kimi Code",
        command: "kimi",
        aliases: &["kimi"],
        supports_model: true,
        supports_resume: false,
        supports_full_access: false,
    },
    AgentSpec {
        id: "mimo",
        name: "MiMo-Code",
        command: "mimo",
        aliases: &["mimo"],
        supports_model: true,
        supports_resume: true,
        supports_full_access: true,
    },
];

pub fn agent_list(service: &RuntimeService) -> Value {
    let settings = service.sync_tool_permissions();
    let gateway_settings = crate::gateway_service::GatewaySettings::load(service.support_dir());
    let gateway_status = crate::gateway_service::GatewayService::global_status();
    let executables = AGENTS
        .iter()
        .map(|agent| find_executable(agent.aliases))
        .collect::<Vec<_>>();
    let version_probes = executables
        .iter()
        .map(|executable| {
            executable.as_ref().map(|path| {
                let path = path.clone();
                thread::spawn(move || probe_agent_version(&path))
            })
        })
        .collect::<Vec<_>>();
    let versions = version_probes
        .into_iter()
        .map(|probe| probe.and_then(|probe| probe.join().ok().flatten()))
        .collect::<Vec<_>>();
    let agents = AGENTS
        .iter()
        .zip(executables)
        .zip(versions)
        .map(|((agent, executable), version)| {
            let gateway_required = gateway_client(agent).is_some();
            let gateway_online = gateway_status.addr.is_some();
            json!({
                "id": agent.id,
                "name": agent.name,
                "installed": executable.is_some(),
                "available": executable.is_some() && (!gateway_required || gateway_online),
                "executable": executable.map(|path| path.display().to_string()),
                "version": version,
                "configuredModel": configured_agent_model(&settings, &gateway_settings, agent.id),
                "permissionMode": configured_permission(&settings, agent.id),
                "gateway": {
                    "required": gateway_required,
                    "enabled": gateway_status.enabled,
                    "online": gateway_online,
                    "address": gateway_status.addr.map(|addr| addr.to_string()),
                    "error": gateway_status.error.clone(),
                },
                "capabilities": {
                    "create": true,
                    "resume": agent.supports_resume,
                    "send": true,
                    "status": true,
                    "stop": true,
                    "modelOverride": agent.supports_model,
                    "fullAccess": agent.supports_full_access,
                    "hotModelSwitch": false,
                },
            })
        })
        .collect::<Vec<_>>();
    json!({ "agents": agents })
}

pub fn model_list(service: &RuntimeService, agent_id: &str) -> Result<Value, ProductControlError> {
    let agent = agent_spec(agent_id)?;
    if let Some(client) = gateway_client(agent) {
        let catalog = crate::gateway_service::current_gateway_model_catalog();
        let gateway_settings = crate::gateway_service::GatewaySettings::load(service.support_dir());
        let default_model = gateway_default_model(&gateway_settings, client);
        let models = catalog
            .models
            .iter()
            .filter(|model| gateway_model_compatible(model, client))
            .map(|model| {
                json!({
                    "id": model.id,
                    "name": model.name,
                    "description": model.description,
                    "source": catalog.source,
                    "configured": model.id == default_model,
                    "contextWindowTokens": model.context_window_tokens,
                    "rateMultiplier": model.rate_multiplier,
                    "rateUnit": model.rate_unit,
                    "experimental": model.description.to_ascii_lowercase().contains("experimental"),
                })
            })
            .collect::<Vec<_>>();
        return Ok(json!({
            "agentId": agent.id,
            "models": models,
            "defaultModel": default_model,
            "supportsModelOverride": true,
            "discovery": "kiro-catalog",
            "source": catalog.source,
            "refreshedAt": catalog.refreshed_at.to_rfc3339(),
            "stale": catalog.is_stale_now(),
            "schemaVersion": catalog.schema_version,
        }));
    }
    let settings = service.sync_tool_permissions();
    let model = configured_model(&settings, agent.id);
    let models = model
        .as_deref()
        .filter(|model| !model.is_empty())
        .map(|model| {
            vec![json!({
                "id": model,
                "name": model,
                "source": "wecode-config",
                "configured": true,
            })]
        })
        .unwrap_or_default();
    Ok(json!({
        "agentId": agent.id,
        "models": models,
        "defaultModel": model,
        "supportsModelOverride": agent.supports_model,
        "discovery": "configured-only",
    }))
}

pub fn session_list(
    service: &RuntimeService,
    project_id: Option<&str>,
    worktree_id: Option<&str>,
) -> Result<Value, ProductControlError> {
    let _ = service.poll_ai_runtime_state();
    let runtime = service.ai_runtime_state_snapshot();
    let terminals = service.terminal_manager().list();
    let mut sessions = Vec::new();
    let mut active_external_ids = HashSet::new();
    let mut history_ids = HashSet::new();

    for terminal in terminals.iter().filter(|terminal| terminal.tool.is_some()) {
        if project_id.is_some_and(|id| terminal.project_id != id) {
            continue;
        }
        if worktree_id.is_some_and(|id| terminal.worktree_id.as_deref() != Some(id)) {
            continue;
        }
        let runtime_session = runtime
            .sessions
            .iter()
            .find(|session| session.terminal_id == terminal.id);
        if let Some(external_id) = runtime_session.and_then(|session| session.ai_session_id.clone())
        {
            active_external_ids.insert(external_id);
        }
        sessions.push(active_session_value(
            terminal,
            runtime_session,
            session_ready(service, &terminal.id, runtime_session),
            active_session_status(service, terminal, runtime_session),
        ));
    }

    for project in selected_projects(service, project_id)? {
        if !PathBuf::from(&project.path).is_dir() {
            continue;
        }
        let history = service.reload_project_ai_history(&project.path);
        for session in history.sessions {
            if !history_ids.insert(session.id.clone()) {
                continue;
            }
            if session
                .external_session_id
                .as_ref()
                .is_some_and(|id| active_external_ids.contains(id))
            {
                continue;
            }
            sessions.push(json!({
                "id": session.id,
                "kind": "history",
                "projectId": project.id,
                "projectName": project.name,
                "worktreeId": Value::Null,
                "terminalId": Value::Null,
                "externalSessionId": session.external_session_id,
                "title": session.title,
                "agentId": history_agent_id(&session.source, session.last_model.as_deref()),
                "model": session.last_model,
                "status": "completed",
                "updatedAt": session.last_seen_at,
                "canSend": false,
                "canResume": history_session_can_resume(&session),
            }));
        }
    }
    Ok(json!({ "sessions": sessions }))
}

pub fn session_create(
    service: &RuntimeService,
    project_id: &str,
    worktree_id: Option<&str>,
    agent_id: &str,
    model: Option<&str>,
    permission_mode: Option<&str>,
) -> Result<Value, ProductControlError> {
    let project = resolve_project(service, project_id)?;
    ensure_local_project(service, &project)?;
    let (cwd, resolved_worktree_id) = resolve_workspace(service, &project, worktree_id)?;
    let agent = agent_spec(agent_id)?;
    ensure_agent_installed(agent)?;
    let settings = service.sync_tool_permissions();
    let permission = validate_permission(agent, &settings, permission_mode)?;
    let (command, env, model) = if let Some(client) = gateway_client(agent) {
        prepare_gateway_session(service, agent, client, model, None, &permission)?
    } else {
        let model = validate_model(agent, &settings, model)?;
        (
            create_command(agent, model.as_deref(), &permission),
            None,
            model,
        )
    };
    launch_session(
        service,
        &project,
        &resolved_worktree_id,
        &cwd,
        agent,
        command,
        env,
        model.as_deref(),
    )
}

pub fn session_resume(
    service: &RuntimeService,
    session_id: &str,
    project_id: Option<&str>,
) -> Result<Value, ProductControlError> {
    let (project, session) = find_history_session(service, session_id, project_id)?;
    ensure_local_project(service, &project)?;
    let agent_id = history_agent_id(&session.source, session.last_model.as_deref());
    let agent = agent_spec(&agent_id)?;
    if !agent.supports_resume {
        return Err(ProductControlError::new(
            "UNSUPPORTED_CAPABILITY",
            "this Agent does not expose reliable session resume semantics",
            json!({ "agentId": agent.id, "sessionId": session_id }),
        ));
    }
    if !history_session_can_resume(&session) {
        return Err(ProductControlError::new(
            "UNSUPPORTED_CAPABILITY",
            "this indexed session does not contain a reliable CLI resume identifier",
            json!({ "agentId": agent.id, "sessionId": session_id }),
        ));
    }
    ensure_agent_installed(agent)?;
    let permission = configured_permission(&service.sync_tool_permissions(), agent.id);
    let (command, env, model) = if let Some(client) = gateway_client(agent) {
        let resume_id = session
            .external_session_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or(&session.session_key);
        prepare_gateway_session(
            service,
            agent,
            client,
            session.last_model.as_deref(),
            Some(resume_id),
            &permission,
        )?
    } else {
        (
            crate::ai_history::session_restore_command(&session),
            None,
            session.last_model.clone(),
        )
    };
    launch_session(
        service,
        &project,
        &project.id,
        &project.path,
        agent,
        command,
        env,
        model.as_deref(),
    )
}

pub fn session_send(
    service: &RuntimeService,
    session_id: &str,
    prompt: &str,
) -> Result<Value, ProductControlError> {
    if prompt.trim().is_empty() {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "session prompt cannot be empty",
            Value::Null,
        ));
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "session prompt exceeds the 32768 character limit",
            json!({ "maxChars": MAX_PROMPT_CHARS }),
        ));
    }
    let manager = service.terminal_manager();
    let terminal = manager
        .list()
        .into_iter()
        .find(|terminal| terminal.id == session_id && terminal.tool.is_some())
        .ok_or_else(|| session_not_active(session_id))?;
    let _ = service.poll_ai_runtime_state();
    let runtime = service.ai_runtime_state_snapshot();
    let runtime_session = runtime
        .sessions
        .iter()
        .find(|session| session.terminal_id == session_id);
    if !session_ready(service, session_id, runtime_session) {
        return Err(ProductControlError::new(
            "SESSION_NOT_READY",
            "the Agent runtime binding is not ready; poll session status and retry",
            json!({ "sessionId": session_id, "status": "starting" }),
        ));
    }
    write_session_prompt(prompt, SESSION_ENTER_DELAY, |input| {
        manager.write(session_id, input)
    })
    .map_err(|error| {
        ProductControlError::new(
            "OPERATION_FAILED",
            "failed to send or submit input to the Agent session",
            json!({ "reason": error.to_string() }),
        )
    })?;
    Ok(json!({
        "id": terminal.id,
        "terminalId": terminal.id,
        "accepted": true,
        "status": "running",
    }))
}

pub fn session_status(
    service: &RuntimeService,
    session_id: &str,
) -> Result<Value, ProductControlError> {
    let _ = service.poll_ai_runtime_state();
    let runtime = service.ai_runtime_state_snapshot();
    if let Some(terminal) = service
        .terminal_manager()
        .list()
        .into_iter()
        .find(|terminal| terminal.id == session_id && terminal.tool.is_some())
    {
        let runtime_session = runtime
            .sessions
            .iter()
            .find(|session| session.terminal_id == terminal.id);
        return Ok(active_session_value(
            &terminal,
            runtime_session,
            session_ready(service, &terminal.id, runtime_session),
            active_session_status(service, &terminal, runtime_session),
        ));
    }
    if let Some(runtime_session) = runtime.sessions.iter().find(|session| {
        session.ai_session_id.as_deref() == Some(session_id) || session.terminal_id == session_id
    }) {
        return Ok(runtime_only_session_value(runtime_session));
    }
    let (project, session) = find_history_session(service, session_id, None)?;
    Ok(json!({
        "id": session.id,
        "kind": "history",
        "projectId": project.id,
        "projectName": project.name,
        "externalSessionId": session.external_session_id,
        "title": session.title,
        "agentId": history_agent_id(&session.source, session.last_model.as_deref()),
        "model": session.last_model,
        "status": "completed",
        "updatedAt": session.last_seen_at,
        "canSend": false,
        "canResume": true,
    }))
}

pub fn session_stop(
    service: &RuntimeService,
    session_id: &str,
    confirmed: bool,
) -> Result<Value, ProductControlError> {
    let status = session_status(service, session_id)?;
    if status.get("kind").and_then(Value::as_str) != Some("active") {
        return Err(session_not_active(session_id));
    }
    if !confirmed {
        return Err(ProductControlError::new(
            "CONFIRMATION_REQUIRED",
            "review the active Agent session and retry with --confirm",
            status,
        ));
    }
    service
        .terminal_manager()
        .kill(session_id)
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to stop the Agent session",
                json!({ "reason": error.to_string() }),
            )
        })?;
    Ok(json!({ "id": session_id, "terminalId": session_id, "status": "stopped" }))
}

pub fn terminal_list(
    service: &RuntimeService,
    project_id: Option<&str>,
    worktree_id: Option<&str>,
) -> Result<Value, ProductControlError> {
    if let Some(project_id) = project_id {
        resolve_project(service, project_id)?;
    }
    let terminals = service
        .terminal_manager()
        .list()
        .into_iter()
        .filter(|terminal| terminal.tool.is_none())
        .filter(|terminal| project_id.is_none_or(|id| terminal.project_id == id))
        .filter(|terminal| worktree_id.is_none_or(|id| terminal.worktree_id.as_deref() == Some(id)))
        .map(terminal_value)
        .collect::<Vec<_>>();
    Ok(json!({ "terminals": terminals }))
}

pub fn terminal_create(
    service: &RuntimeService,
    project_id: &str,
    worktree_id: Option<&str>,
    command: Option<&str>,
    title: Option<&str>,
) -> Result<Value, ProductControlError> {
    let project = resolve_project(service, project_id)?;
    ensure_local_project(service, &project)?;
    let (cwd, resolved_worktree_id) = resolve_workspace(service, &project, worktree_id)?;
    let command = normalized_limited(command, MAX_TERMINAL_COMMAND_CHARS, "terminal command")?;
    let title = title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Terminal");
    if title.chars().count() > 128 {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "terminal title exceeds the 128 character limit",
            json!({ "maxChars": 128 }),
        ));
    }
    let terminal_id = service
        .terminal_manager()
        .create(
            TerminalPtyConfig {
                cwd: Some(cwd),
                command,
                project_id: Some(project.id.clone()),
                project_name: Some(project.name.clone()),
                worktree_id: Some(resolved_worktree_id),
                title: Some(title.to_string()),
                support_dir: Some(service.support_dir().to_path_buf()),
                runtime_root: Some(staged_runtime_root_path()),
                ..Default::default()
            },
            |_| {},
        )
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to create the terminal",
                json!({ "reason": error.to_string() }),
            )
        })?;
    terminal_by_id(service, &terminal_id).map(terminal_value)
}

pub fn terminal_send(
    service: &RuntimeService,
    terminal_id: &str,
    text: &str,
    enter: bool,
) -> Result<Value, ProductControlError> {
    let terminal = terminal_by_id(service, terminal_id)?;
    if text.is_empty() && !enter {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "terminal input cannot be empty unless --enter is set",
            Value::Null,
        ));
    }
    if text.chars().count() > MAX_TERMINAL_INPUT_CHARS {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "terminal input exceeds the 32768 character limit",
            json!({ "maxChars": MAX_TERMINAL_INPUT_CHARS }),
        ));
    }
    if !terminal.is_running {
        return Err(ProductControlError::new(
            "TERMINAL_NOT_RUNNING",
            "the requested terminal is not running",
            json!({ "terminalId": terminal_id, "status": terminal.status }),
        ));
    }
    if terminal.command.trim().is_empty() && terminal.buffer_characters < 8 {
        return Err(ProductControlError::new(
            "TERMINAL_NOT_READY",
            "the interactive shell is still starting; poll terminal list and retry",
            json!({ "terminalId": terminal_id, "status": "starting" }),
        ));
    }
    let mut input = text.as_bytes().to_vec();
    if enter {
        input.push(b'\r');
    }
    service
        .terminal_manager()
        .write(terminal_id, &input)
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to send terminal input",
                json!({ "reason": error.to_string() }),
            )
        })?;
    Ok(json!({
        "id": terminal.id,
        "terminalId": terminal.id,
        "accepted": true,
        "entered": enter,
        "status": terminal.status,
    }))
}

pub fn terminal_snapshot(
    service: &RuntimeService,
    terminal_id: &str,
    tail: Option<usize>,
) -> Result<Value, ProductControlError> {
    let terminal = terminal_by_id(service, terminal_id)?;
    let requested = tail.unwrap_or(DEFAULT_TERMINAL_SNAPSHOT_CHARS);
    if requested == 0 || requested > MAX_TERMINAL_SNAPSHOT_CHARS {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "terminal snapshot tail must be between 1 and 65536 characters",
            json!({ "maxChars": MAX_TERMINAL_SNAPSHOT_CHARS }),
        ));
    }
    let source_limit = requested
        .saturating_mul(4)
        .min(MAX_TERMINAL_SNAPSHOT_SOURCE_CHARS);
    let (source, start_offset) = service
        .terminal_manager()
        .snapshot_tail(terminal_id, source_limit)
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to read the terminal snapshot",
                json!({ "reason": error.to_string() }),
            )
        })?;
    let plain = strip_terminal_control_sequences(&source);
    let plain_chars = plain.chars().count();
    let text = if plain_chars > requested {
        plain.chars().skip(plain_chars - requested).collect()
    } else {
        plain
    };
    Ok(json!({
        "id": terminal.id,
        "terminalId": terminal.id,
        "status": terminal.status,
        "text": text,
        "startOffset": start_offset,
        "bufferCharacters": terminal.buffer_characters,
        "truncated": start_offset > 0 || plain_chars > requested,
        "maxChars": requested,
    }))
}

pub fn terminal_close(
    service: &RuntimeService,
    terminal_id: &str,
    confirmed: bool,
) -> Result<Value, ProductControlError> {
    let terminal = terminal_by_id(service, terminal_id)?;
    let details = terminal_value(terminal);
    if !confirmed {
        return Err(ProductControlError::new(
            "CONFIRMATION_REQUIRED",
            "review the terminal target and retry with --confirm",
            details,
        ));
    }
    service
        .terminal_manager()
        .kill(terminal_id)
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to close the terminal",
                json!({ "reason": error.to_string() }),
            )
        })?;
    Ok(json!({ "id": terminal_id, "terminalId": terminal_id, "status": "closed" }))
}

fn terminal_by_id(
    service: &RuntimeService,
    terminal_id: &str,
) -> Result<crate::terminal_pty::TerminalSessionSnapshot, ProductControlError> {
    service
        .terminal_manager()
        .list()
        .into_iter()
        .find(|terminal| terminal.id == terminal_id && terminal.tool.is_none())
        .ok_or_else(|| {
            ProductControlError::new(
                "TERMINAL_NOT_FOUND",
                "the requested ordinary terminal was not found",
                json!({ "terminalId": terminal_id }),
            )
        })
}

fn terminal_value(terminal: crate::terminal_pty::TerminalSessionSnapshot) -> Value {
    json!({
        "kind": "terminal",
        "id": terminal.id,
        "terminalId": terminal.id,
        "title": terminal.title,
        "projectId": terminal.project_id,
        "projectName": terminal.project_name,
        "worktreeId": terminal.worktree_id,
        "cwd": terminal.cwd,
        "shell": terminal.shell,
        "command": terminal.command,
        "cols": terminal.cols,
        "rows": terminal.rows,
        "status": terminal.status,
        "isRunning": terminal.is_running,
        "createdAt": terminal.created_at,
        "lastActiveAt": terminal.last_active_at,
        "bufferCharacters": terminal.buffer_characters,
        "hasBuffer": terminal.has_buffer,
        "canSend": terminal.is_running
            && (!terminal.command.trim().is_empty() || terminal.buffer_characters >= 8),
    })
}

fn normalized_limited(
    value: Option<&str>,
    max_chars: usize,
    label: &str,
) -> Result<Option<String>, ProductControlError> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            format!("{label} exceeds the {max_chars} character limit"),
            json!({ "maxChars": max_chars }),
        ));
    }
    Ok(value.map(str::to_string))
}

pub fn automation_list(service: &RuntimeService) -> Value {
    let snapshot =
        crate::automation::AutomationService::for_support_dir(service.support_dir()).snapshot();
    let automations = snapshot
        .definitions
        .iter()
        .map(|definition| automation_value(definition, &snapshot.runs))
        .collect::<Vec<_>>();
    json!({ "automations": automations })
}

pub fn automation_create(
    service: &RuntimeService,
    params: &wecode_protocol::LocalControlAutomationCreateParams,
) -> Result<Value, ProductControlError> {
    let project = resolve_project(service, &params.project_id)?;
    ensure_local_project(service, &project)?;
    let workspace_mode = automation_workspace_mode(params.workspace_mode.as_deref())?;
    let (workspace_id, workspace_name, workspace_path) = resolve_automation_workspace(
        service,
        &project,
        workspace_mode,
        params.worktree_id.as_deref(),
    )?;
    let agent = automation_agent(params.agent_id.as_deref())?;
    let model = if agent.uses_gateway() {
        Some(validate_automation_gateway_model(
            service,
            agent,
            params.model.as_deref(),
        )?)
    } else {
        None
    };
    let request = crate::automation::AutomationCreateRequest {
        name: params.name.clone(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        workspace_id,
        workspace_name,
        workspace_path,
        workspace_mode,
        project_path: project.path.clone(),
        base_branch: params.base_branch.clone(),
        reuse_session: params.reuse_session,
        host_device_id: None,
        agent,
        model,
        prompt: params.prompt.clone(),
        precheck_command: params.precheck_command.clone(),
        precheck_timeout_seconds: params.precheck_timeout_seconds.unwrap_or(60),
        schedule_spec: params
            .schedule
            .clone()
            .unwrap_or_else(|| "daily:09:00".to_string()),
        timezone: params
            .timezone
            .clone()
            .unwrap_or_else(|| "Asia/Shanghai".to_string()),
        catch_up_grace_seconds: params.catch_up_grace_seconds.unwrap_or(43_200),
    };
    let automation_service =
        crate::automation::AutomationService::for_support_dir(service.support_dir());
    let definition = automation_service
        .create(request, chrono::Utc::now().timestamp())
        .map_err(|error| automation_operation_error("new", error))?;
    let snapshot = automation_service.snapshot();
    Ok(automation_value(&definition, &snapshot.runs))
}

pub fn automation_update(
    service: &RuntimeService,
    params: &wecode_protocol::LocalControlAutomationUpdateParams,
) -> Result<Value, ProductControlError> {
    let automation_service =
        crate::automation::AutomationService::for_support_dir(service.support_dir());
    let snapshot = automation_service.snapshot();
    let current = snapshot
        .definitions
        .iter()
        .find(|definition| definition.id == params.automation_id)
        .cloned()
        .ok_or_else(|| automation_not_found(&params.automation_id))?;
    let project_id = params.project_id.as_deref().unwrap_or(&current.project_id);
    let project = resolve_project(service, project_id)?;
    ensure_local_project(service, &project)?;
    let workspace_mode = params
        .workspace_mode
        .as_deref()
        .map_or(Ok(current.workspace_mode), |mode| {
            automation_workspace_mode(Some(mode))
        })?;
    let retained_worktree_id = (params.project_id.is_none()
        && params.worktree_id.is_none()
        && workspace_mode == crate::automation::AutomationWorkspaceMode::Existing
        && current.workspace_mode == crate::automation::AutomationWorkspaceMode::Existing)
        .then_some(current.workspace_id.as_str())
        .filter(|id| *id != project.id);
    let worktree_id = params.worktree_id.as_deref().or(retained_worktree_id);
    let (workspace_id, workspace_name, workspace_path) =
        resolve_automation_workspace(service, &project, workspace_mode, worktree_id)?;
    let agent = params
        .agent_id
        .as_deref()
        .map_or(Ok(current.agent), |agent| automation_agent(Some(agent)))?;
    let model = if agent.uses_gateway() {
        Some(validate_automation_gateway_model(
            service,
            agent,
            params.model.as_deref().or(current.model.as_deref()),
        )?)
    } else {
        None
    };
    let runtime_changed = agent != current.agent || model != current.model;
    let request = crate::automation::AutomationCreateRequest {
        name: params.name.clone().unwrap_or(current.name),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        workspace_id,
        workspace_name,
        workspace_path,
        workspace_mode,
        project_path: project.path.clone(),
        base_branch: params.base_branch.clone().or(current.base_branch),
        reuse_session: params.reuse_session.unwrap_or(if runtime_changed {
            false
        } else {
            current.reuse_session
        }),
        host_device_id: None,
        agent,
        model,
        prompt: params.prompt.clone().unwrap_or(current.prompt),
        precheck_command: params.precheck_command.clone().or(current.precheck_command),
        precheck_timeout_seconds: params
            .precheck_timeout_seconds
            .unwrap_or(current.precheck_timeout_seconds),
        schedule_spec: params
            .schedule
            .clone()
            .unwrap_or_else(|| current.schedule.spec()),
        timezone: params.timezone.clone().unwrap_or(current.timezone),
        catch_up_grace_seconds: params
            .catch_up_grace_seconds
            .unwrap_or(current.catch_up_grace_seconds),
    };
    let definition = automation_service
        .update_definition(
            &params.automation_id,
            request,
            chrono::Utc::now().timestamp(),
        )
        .map_err(|error| automation_operation_error(&params.automation_id, error))?;
    let updated = automation_service.snapshot();
    Ok(automation_value(&definition, &updated.runs))
}

fn validate_automation_gateway_model(
    service: &RuntimeService,
    agent: crate::automation::AutomationAgent,
    requested: Option<&str>,
) -> Result<String, ProductControlError> {
    let settings = crate::gateway_service::GatewaySettings::load(service.support_dir());
    let (client, default_model) = match agent {
        crate::automation::AutomationAgent::KiroCodex => {
            ("codex", settings.default_codex_model.as_str())
        }
        _ => ("claude", settings.default_claude_model.as_str()),
    };
    let requested = requested
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .unwrap_or(default_model);
    let catalog = crate::gateway_service::current_gateway_model_catalog();
    let Some(model) = catalog.model(requested) else {
        return Err(ProductControlError::new(
            "MODEL_NOT_AVAILABLE",
            "the requested automation model is not present in the current Kiro catalog",
            json!({ "agentId": agent.id(), "model": requested }),
        ));
    };
    if !gateway_model_compatible(model, client) {
        return Err(ProductControlError::new(
            "MODEL_CLIENT_INCOMPATIBLE",
            "the requested automation model is not compatible with this Agent",
            json!({ "agentId": agent.id(), "model": requested }),
        ));
    }
    Ok(model.id.clone())
}

fn automation_agent(
    agent_id: Option<&str>,
) -> Result<crate::automation::AutomationAgent, ProductControlError> {
    match agent_id.unwrap_or("kiro_gateway_claude").trim() {
        "claude" => Ok(crate::automation::AutomationAgent::Claude),
        "kiro_gateway_claude" | "claude+kiro" | "claude-kiro" => {
            Ok(crate::automation::AutomationAgent::KiroGatewayClaude)
        }
        "kiro_gateway_codex" | "kiro-codex" | "codex+kiro" | "codex-kiro" => {
            Ok(crate::automation::AutomationAgent::KiroCodex)
        }
        "codex" => Ok(crate::automation::AutomationAgent::Codex),
        "kiro" => Ok(crate::automation::AutomationAgent::Kiro),
        value => Err(ProductControlError::new(
            "INVALID_PARAMS",
            "automation Agent must be claude, kiro_gateway_claude, kiro_gateway_codex, codex, or kiro",
            json!({ "agentId": value }),
        )),
    }
}

fn automation_workspace_mode(
    value: Option<&str>,
) -> Result<crate::automation::AutomationWorkspaceMode, ProductControlError> {
    match value.unwrap_or("existing").trim() {
        "existing" => Ok(crate::automation::AutomationWorkspaceMode::Existing),
        "new" | "new_per_run" => Ok(crate::automation::AutomationWorkspaceMode::NewPerRun),
        value => Err(ProductControlError::new(
            "INVALID_PARAMS",
            "automation workspace mode must be existing or new",
            json!({ "workspaceMode": value }),
        )),
    }
}

fn resolve_automation_workspace(
    service: &RuntimeService,
    project: &ProjectSummary,
    mode: crate::automation::AutomationWorkspaceMode,
    worktree_id: Option<&str>,
) -> Result<(String, String, String), ProductControlError> {
    if mode == crate::automation::AutomationWorkspaceMode::NewPerRun {
        return Ok((
            project.id.clone(),
            project.name.clone(),
            project.path.clone(),
        ));
    }
    let Some(worktree_id) = worktree_id.filter(|id| !id.trim().is_empty()) else {
        return Ok((
            project.id.clone(),
            project.name.clone(),
            project.path.clone(),
        ));
    };
    if worktree_id == project.id {
        return Ok((
            project.id.clone(),
            project.name.clone(),
            project.path.clone(),
        ));
    }
    let snapshot = service.worktree_snapshot(project.id.clone(), project.path.clone());
    let worktree = snapshot
        .worktrees
        .into_iter()
        .find(|worktree| worktree.id == worktree_id)
        .ok_or_else(|| {
            ProductControlError::new(
                "WORKTREE_NOT_FOUND",
                "the requested automation worktree does not belong to the project",
                json!({ "projectId": project.id, "worktreeId": worktree_id }),
            )
        })?;
    let name = if worktree.name.trim().is_empty() {
        worktree.branch.clone()
    } else {
        worktree.name.clone()
    };
    Ok((worktree.id, name, worktree.path))
}

pub fn automation_run(
    service: &RuntimeService,
    automation_id: &str,
) -> Result<Value, ProductControlError> {
    let automation_service =
        crate::automation::AutomationService::for_support_dir(service.support_dir());
    let snapshot = automation_service.snapshot();
    let definition = snapshot
        .definitions
        .iter()
        .find(|definition| definition.id == automation_id)
        .ok_or_else(|| automation_not_found(automation_id))?;
    if let Some(run) = snapshot
        .runs
        .iter()
        .rev()
        .find(|run| run.automation_id == automation_id && !run.state.is_terminal())
    {
        return Err(ProductControlError::new(
            "AUTOMATION_ACTIVE_RUN",
            "the automation already has an active run",
            json!({ "automationId": automation_id, "runId": run.id, "state": run.state }),
        ));
    }
    let now = chrono::Utc::now().timestamp();
    let plan = automation_service
        .enqueue_manual(automation_id, now)
        .map_err(|error| automation_operation_error(automation_id, error))?;
    let run_id = plan.run_id.clone();
    if let Err(error) = service.dispatch_automation_plan(plan) {
        let _ = automation_service.mark_failed(&run_id, error.clone(), now);
        return Err(ProductControlError::new(
            "AUTOMATION_DISPATCH_UNAVAILABLE",
            "the Desktop automation executor is unavailable",
            json!({ "automationId": automation_id, "runId": run_id, "reason": error }),
        ));
    }
    Ok(json!({
        "id": definition.id,
        "automationId": definition.id,
        "name": definition.name,
        "runId": run_id,
        "state": "scheduled",
        "accepted": true,
    }))
}

pub fn automation_set_enabled(
    service: &RuntimeService,
    automation_id: &str,
    enabled: bool,
) -> Result<Value, ProductControlError> {
    let automation_service =
        crate::automation::AutomationService::for_support_dir(service.support_dir());
    let snapshot = automation_service.snapshot();
    let definition = snapshot
        .definitions
        .iter()
        .find(|definition| definition.id == automation_id)
        .ok_or_else(|| automation_not_found(automation_id))?;
    let changed = definition.enabled != enabled;
    if changed {
        automation_service
            .set_enabled(automation_id, enabled, chrono::Utc::now().timestamp())
            .map_err(|error| automation_operation_error(automation_id, error))?;
    }
    let updated = automation_service.snapshot();
    let definition = updated
        .definitions
        .iter()
        .find(|definition| definition.id == automation_id)
        .ok_or_else(|| automation_not_found(automation_id))?;
    let mut value = automation_value(definition, &updated.runs);
    value["changed"] = json!(changed);
    Ok(value)
}

fn automation_value(
    definition: &crate::automation::AutomationDefinition,
    runs: &[crate::automation::AutomationRun],
) -> Value {
    let latest_run = runs
        .iter()
        .rev()
        .find(|run| run.automation_id == definition.id)
        .map(automation_run_value)
        .unwrap_or(Value::Null);
    json!({
        "id": definition.id,
        "automationId": definition.id,
        "name": definition.name,
        "enabled": definition.enabled,
        "projectId": definition.project_id,
        "projectName": definition.project_name,
        "workspaceId": definition.workspace_id,
        "workspaceName": definition.workspace_name,
        "workspaceMode": definition.workspace_mode,
        "agentId": definition.agent.id(),
        "model": definition.model,
        "schedule": definition.schedule,
        "scheduleLabel": definition.schedule.display(),
        "timezone": definition.timezone,
        "nextRunAt": definition.next_run_at,
        "latestRun": latest_run,
    })
}

fn automation_run_value(run: &crate::automation::AutomationRun) -> Value {
    json!({
        "id": run.id,
        "runId": run.id,
        "trigger": run.trigger,
        "state": run.state,
        "stateReason": run.state_reason,
        "terminalId": run.terminal_id,
        "aiSessionId": run.ai_session_id,
        "workspaceId": run.workspace_id,
        "workspaceName": run.workspace_name,
        "runNumber": run.run_number,
        "scheduledFor": run.scheduled_for,
        "startedAt": run.started_at,
        "finishedAt": run.finished_at,
    })
}

fn automation_not_found(automation_id: &str) -> ProductControlError {
    ProductControlError::new(
        "AUTOMATION_NOT_FOUND",
        "the requested automation was not found",
        json!({ "automationId": automation_id }),
    )
}

fn automation_operation_error(automation_id: &str, error: String) -> ProductControlError {
    let code = if error.contains("正在运行") {
        "AUTOMATION_ACTIVE_RUN"
    } else if error.contains("不存在") {
        "AUTOMATION_NOT_FOUND"
    } else {
        "OPERATION_FAILED"
    };
    ProductControlError::new(
        code,
        "automation operation failed",
        json!({ "automationId": automation_id, "reason": error }),
    )
}

fn strip_terminal_control_sequences(value: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Text,
        Escape,
        Csi,
        Osc,
        OscEscape,
    }

    let mut state = State::Text;
    let mut plain = String::with_capacity(value.len());
    for ch in value.chars() {
        state = match state {
            State::Text => match ch {
                '\u{1b}' => State::Escape,
                '\r' => State::Text,
                '\n' | '\t' => {
                    plain.push(ch);
                    State::Text
                }
                ch if ch.is_control() => State::Text,
                _ => {
                    plain.push(ch);
                    State::Text
                }
            },
            State::Escape => match ch {
                '[' => State::Csi,
                ']' => State::Osc,
                _ => State::Text,
            },
            State::Csi => {
                if ('@'..='~').contains(&ch) {
                    State::Text
                } else {
                    State::Csi
                }
            }
            State::Osc => match ch {
                '\u{7}' => State::Text,
                '\u{1b}' => State::OscEscape,
                _ => State::Osc,
            },
            State::OscEscape => {
                if ch == '\\' {
                    State::Text
                } else {
                    State::Osc
                }
            }
        };
    }
    plain
}

fn launch_session(
    service: &RuntimeService,
    project: &ProjectSummary,
    worktree_id: &str,
    cwd: &str,
    agent: &AgentSpec,
    command: String,
    env: Option<HashMap<String, String>>,
    model: Option<&str>,
) -> Result<Value, ProductControlError> {
    let permissions = service.sync_tool_permissions();
    let terminal_id = service
        .terminal_manager()
        .create(
            TerminalPtyConfig {
                cwd: Some(cwd.to_string()),
                command: Some(command),
                env,
                project_id: Some(project.id.clone()),
                project_name: Some(project.name.clone()),
                worktree_id: Some(worktree_id.to_string()),
                title: Some(agent.name.to_string()),
                tool: Some(agent.id.to_string()),
                support_dir: Some(service.support_dir().to_path_buf()),
                runtime_root: Some(staged_runtime_root_path()),
                tool_permissions_file: Some(PathBuf::from(permissions.path)),
                ..Default::default()
            },
            |_| {},
        )
        .map_err(|error| {
            ProductControlError::new(
                "OPERATION_FAILED",
                "failed to create the Agent session terminal",
                json!({ "reason": error.to_string() }),
            )
        })?;
    Ok(json!({
        "id": terminal_id,
        "kind": "active",
        "terminalId": terminal_id,
        "projectId": project.id,
        "projectName": project.name,
        "worktreeId": worktree_id,
        "agentId": agent.id,
        "model": model,
        "status": "starting",
        "canSend": false,
        "canResume": false,
    }))
}

fn resolve_project(
    service: &RuntimeService,
    project_id: &str,
) -> Result<ProjectSummary, ProductControlError> {
    service
        .project_list()
        .projects
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            ProductControlError::new(
                "PROJECT_NOT_FOUND",
                "the requested project is not registered in WeCode Desktop",
                json!({ "projectId": project_id }),
            )
        })
}

fn ensure_local_project(
    service: &RuntimeService,
    project: &ProjectSummary,
) -> Result<(), ProductControlError> {
    if service
        .host_device_for_project_path(&project.path)
        .is_some()
    {
        return Err(ProductControlError::new(
            "UNSUPPORTED_CAPABILITY",
            "P0 Agent sessions only support projects hosted by this Desktop",
            json!({ "projectId": project.id }),
        ));
    }
    if !PathBuf::from(&project.path).is_dir() {
        return Err(ProductControlError::new(
            "PROJECT_NOT_FOUND",
            "the local project directory is unavailable",
            json!({ "projectId": project.id }),
        ));
    }
    Ok(())
}

fn resolve_workspace(
    service: &RuntimeService,
    project: &ProjectSummary,
    worktree_id: Option<&str>,
) -> Result<(String, String), ProductControlError> {
    let Some(worktree_id) = worktree_id.filter(|id| !id.trim().is_empty()) else {
        return Ok((project.path.clone(), project.id.clone()));
    };
    let snapshot = service.worktree_snapshot(project.id.clone(), project.path.clone());
    let worktree = snapshot
        .worktrees
        .into_iter()
        .find(|worktree| worktree.id == worktree_id)
        .ok_or_else(|| {
            ProductControlError::new(
                "WORKTREE_NOT_FOUND",
                "the requested worktree does not belong to the project",
                json!({ "projectId": project.id, "worktreeId": worktree_id }),
            )
        })?;
    Ok((worktree.path, worktree.id))
}

fn find_history_session(
    service: &RuntimeService,
    session_id: &str,
    project_id: Option<&str>,
) -> Result<(ProjectSummary, AISessionSummary), ProductControlError> {
    let mut matches = Vec::new();
    for project in selected_projects(service, project_id)? {
        if !PathBuf::from(&project.path).is_dir() {
            continue;
        }
        let history = service.reload_project_ai_history(&project.path);
        if let Some(session) = history.sessions.into_iter().find(|session| {
            session.id == session_id
                || session.session_key == session_id
                || session.external_session_id.as_deref() == Some(session_id)
        }) {
            matches.push((project, session));
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(ProductControlError::new(
            "SESSION_NOT_FOUND",
            "the requested Agent session was not found",
            json!({ "sessionId": session_id }),
        )),
        _ => Err(ProductControlError::new(
            "AMBIGUOUS_TARGET",
            "the session identifier matches more than one project; provide --project",
            json!({ "sessionId": session_id }),
        )),
    }
}

fn selected_projects(
    service: &RuntimeService,
    project_id: Option<&str>,
) -> Result<Vec<ProjectSummary>, ProductControlError> {
    let projects = service.project_list().projects;
    if let Some(project_id) = project_id {
        let project = projects
            .into_iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| {
                ProductControlError::new(
                    "PROJECT_NOT_FOUND",
                    "the requested project is not registered in WeCode Desktop",
                    json!({ "projectId": project_id }),
                )
            })?;
        Ok(vec![project])
    } else {
        Ok(projects)
    }
}

fn active_session_value(
    terminal: &crate::terminal_pty::TerminalSessionSnapshot,
    runtime: Option<&crate::ai_runtime::AISessionSnapshot>,
    ready: bool,
    status: &'static str,
) -> Value {
    json!({
        "id": terminal.id,
        "kind": "active",
        "projectId": terminal.project_id,
        "projectName": terminal.project_name,
        "worktreeId": terminal.worktree_id,
        "terminalId": terminal.id,
        "externalSessionId": runtime.and_then(|session| session.ai_session_id.clone()),
        "title": runtime.map(|session| session.session_title.clone()).unwrap_or_else(|| terminal.title.clone()),
        "agentId": terminal.tool.as_deref().map(canonical_agent_id).or_else(|| runtime.map(|session| canonical_agent_id(&session.tool))),
        "model": runtime.and_then(|session| session.model.clone()),
        "status": status,
        "updatedAt": runtime.map(|session| session.updated_at),
        "latestAssistantPreview": runtime.and_then(|session| session.latest_assistant_preview.clone()),
        "message": runtime.and_then(|session| session.message.clone()),
        "canSend": terminal.is_running && ready,
        "canResume": false,
    })
}

fn active_session_status(
    service: &RuntimeService,
    terminal: &crate::terminal_pty::TerminalSessionSnapshot,
    runtime: Option<&crate::ai_runtime::AISessionSnapshot>,
) -> &'static str {
    if let Ok(screen) = service.terminal_manager().snapshot(&terminal.id) {
        if let Some(status) = agent_screen_runtime_status(&screen) {
            return status;
        }
    }
    runtime
        .map(runtime_status)
        .unwrap_or(if terminal.is_running {
            "starting"
        } else {
            "stopped"
        })
}

fn agent_screen_runtime_status(screen: &str) -> Option<&'static str> {
    use crate::ai_runtime::{
        ScreenSignal, screen_signal::detect_screen_signal, tool_driver::COMMON_SCREEN_PATTERNS,
    };

    match detect_screen_signal(screen, &[COMMON_SCREEN_PATTERNS]) {
        ScreenSignal::Waiting => Some("waiting_input"),
        ScreenSignal::Running => Some("running"),
        ScreenSignal::Unknown => None,
    }
}

fn session_ready(
    service: &RuntimeService,
    session_id: &str,
    runtime: Option<&crate::ai_runtime::AISessionSnapshot>,
) -> bool {
    if runtime.is_none() {
        return false;
    }
    service
        .terminal_manager()
        .snapshot(session_id)
        .map(|screen| agent_screen_accepts_prompt(&screen))
        .unwrap_or(false)
}

fn agent_screen_accepts_prompt(screen: &str) -> bool {
    let normalized = screen
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if normalized.chars().count() < 8 {
        return false;
    }
    const STARTUP_GATES: [&str; 11] = [
        "press enter to continue",
        "press enter to confirm or esc to go back",
        "continue anyway? [y/n]",
        "sign in with chatgpt",
        "finish signing in via your browser",
        "do you trust the contents of this directory?",
        "trust this folder",
        "starting mcp server",
        "starting mcp servers",
        "booting mcp server",
        "hooks need review",
    ];
    !STARTUP_GATES.iter().any(|gate| normalized.contains(gate))
}

fn write_session_prompt<E>(
    prompt: &str,
    enter_delay: Duration,
    mut write: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    write(prompt.as_bytes())?;
    // TUI Agents distinguish pasted text from a physical Enter key. Writing
    // both in one PTY frame leaves Codex's prompt in the composer without
    // submitting it, so preserve the key boundary with a short pacing gap.
    if !enter_delay.is_zero() {
        thread::sleep(enter_delay);
    }
    write(b"\r")
}

fn runtime_only_session_value(session: &crate::ai_runtime::AISessionSnapshot) -> Value {
    json!({
        "id": session.terminal_id,
        "kind": "runtime",
        "projectId": session.project_id,
        "projectName": session.project_name,
        "terminalId": session.terminal_id,
        "externalSessionId": session.ai_session_id,
        "title": session.session_title,
        "agentId": canonical_agent_id(&session.tool),
        "model": session.model,
        "status": runtime_status(session),
        "updatedAt": session.updated_at,
        "latestAssistantPreview": session.latest_assistant_preview,
        "message": session.message,
        "canSend": false,
        "canResume": false,
    })
}

fn runtime_status(session: &crate::ai_runtime::AISessionSnapshot) -> &'static str {
    match session.state.as_str() {
        "responding" => "running",
        "needsInput" => "waiting_input",
        "idle" if session.was_interrupted => "failed",
        "idle" if session.has_completed_turn => "completed",
        "idle" => "starting",
        _ => "unknown",
    }
}

fn gateway_client(agent: &AgentSpec) -> Option<&'static str> {
    match agent.id {
        "kiro-claude" => Some("claude"),
        "kiro-codex" => Some("codex"),
        _ => None,
    }
}

fn gateway_default_model<'a>(
    settings: &'a crate::gateway_service::GatewaySettings,
    client: &str,
) -> &'a str {
    match client {
        "codex" => &settings.default_codex_model,
        _ => &settings.default_claude_model,
    }
}

fn gateway_model_compatible(model: &crate::gateway_service::GatewayModel, client: &str) -> bool {
    match client {
        "codex" => model.compatibility.codex_cli,
        _ => model.compatibility.claude_code,
    }
}

fn configured_agent_model(
    tool_settings: &ToolPermissionsSummary,
    gateway_settings: &crate::gateway_service::GatewaySettings,
    agent_id: &str,
) -> Option<String> {
    match agent_id {
        "kiro-claude" => Some(gateway_settings.default_claude_model.clone()),
        "kiro-codex" => Some(gateway_settings.default_codex_model.clone()),
        _ => configured_model(tool_settings, agent_id),
    }
}

fn prepare_gateway_session(
    service: &RuntimeService,
    agent: &AgentSpec,
    client: &str,
    requested_model: Option<&str>,
    resume_id: Option<&str>,
    permission: &str,
) -> Result<(String, Option<HashMap<String, String>>, Option<String>), ProductControlError> {
    let status = crate::gateway_service::GatewayService::global_status();
    let Some(addr) = status.addr else {
        return Err(ProductControlError::new(
            "GATEWAY_OFFLINE",
            "Kiro Gateway is not listening",
            json!({
                "agentId": agent.id,
                "enabled": status.enabled,
                "reason": status.error,
            }),
        ));
    };
    let settings = crate::gateway_service::GatewaySettings::load(service.support_dir());
    let requested = requested_model
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .unwrap_or_else(|| gateway_default_model(&settings, client));
    let catalog = crate::gateway_service::current_gateway_model_catalog();
    let Some(model) = catalog.model(requested) else {
        return Err(ProductControlError::new(
            "MODEL_NOT_AVAILABLE",
            "the requested model is not present in the current Kiro catalog",
            json!({ "agentId": agent.id, "model": requested }),
        ));
    };
    if !gateway_model_compatible(model, client) {
        return Err(ProductControlError::new(
            "MODEL_CLIENT_INCOMPATIBLE",
            "the requested model is not enabled for this Kiro Gateway Agent",
            json!({ "agentId": agent.id, "model": requested }),
        ));
    }
    let (mut command, env) = match client {
        "codex" => {
            let command = crate::gateway_service::gateway_codex_command(
                &model.id,
                &format!("http://{addr}/v1"),
                model.context_window_tokens,
            );
            let mut command = apply_gateway_permission(command, client, permission);
            command.push_str(" --disable hooks");
            (
                command,
                crate::gateway_service::gateway_codex_environment(
                    &settings.config.api_key,
                    &model.id,
                ),
            )
        }
        _ => {
            let command = crate::gateway_service::gateway_claude_command(&model.id, resume_id);
            (
                apply_gateway_permission(command, client, permission),
                crate::gateway_service::gateway_claude_environment(
                    &format!("http://{addr}"),
                    &settings.config.api_key,
                    &model.id,
                ),
            )
        }
    };
    if client == "codex" {
        if let Some(resume_id) = resume_id.filter(|id| !id.trim().is_empty()) {
            command.push_str(" resume ");
            command.push_str(&shell_quote(resume_id));
        }
    }
    Ok((command, Some(env), Some(model.id.clone())))
}

fn apply_gateway_permission(mut command: String, client: &str, permission: &str) -> String {
    if client == "codex" && permission == "fullAccess" {
        command.push_str(" --dangerously-bypass-approvals-and-sandbox");
    } else if client == "claude" && permission != "fullAccess" {
        command = command.replace(" --permission-mode bypassPermissions", "");
    }
    command
}

fn create_command(agent: &AgentSpec, model: Option<&str>, permission: &str) -> String {
    let mut command = agent.command.to_string();
    if let Some(model) = model {
        command.push_str(" --model ");
        command.push_str(&shell_quote(model));
    }
    if permission == "fullAccess" {
        match agent.id {
            "codex" => command.push_str(" --dangerously-bypass-approvals-and-sandbox"),
            "claude" => command.push_str(" --permission-mode bypassPermissions"),
            _ => {}
        }
    }
    command
}

fn validate_model(
    agent: &AgentSpec,
    settings: &ToolPermissionsSummary,
    requested: Option<&str>,
) -> Result<Option<String>, ProductControlError> {
    let configured = configured_model(settings, agent.id).filter(|model| !model.is_empty());
    let Some(requested) = requested.map(str::trim).filter(|model| !model.is_empty()) else {
        return Ok(agent.supports_model.then_some(configured).flatten());
    };
    if !agent.supports_model || configured.as_deref() != Some(requested) {
        return Err(ProductControlError::new(
            "UNSUPPORTED_CAPABILITY",
            "the requested model is not exposed by WeCode for this Agent",
            json!({ "agentId": agent.id, "model": requested, "configuredModel": configured }),
        ));
    }
    Ok(Some(requested.to_string()))
}

fn validate_permission(
    agent: &AgentSpec,
    settings: &ToolPermissionsSummary,
    requested: Option<&str>,
) -> Result<String, ProductControlError> {
    let configured = configured_permission(settings, agent.id);
    let requested = requested.unwrap_or(&configured).trim();
    if !matches!(requested, "default" | "fullAccess") {
        return Err(ProductControlError::new(
            "INVALID_PARAMS",
            "permission mode must be default or fullAccess",
            json!({ "permissionMode": requested }),
        ));
    }
    if requested != configured || (requested == "fullAccess" && !agent.supports_full_access) {
        return Err(ProductControlError::new(
            "UNAUTHORIZED",
            "the requested permission mode exceeds the Agent permissions configured in WeCode",
            json!({ "agentId": agent.id, "permissionMode": requested }),
        ));
    }
    Ok(requested.to_string())
}

fn configured_model(settings: &ToolPermissionsSummary, agent_id: &str) -> Option<String> {
    let model = match agent_id {
        "codex" => &settings.codex_model,
        "claude" => &settings.claude_code_model,
        "agy" => &settings.agy_model,
        "opencode" => &settings.opencode_model,
        "kiro" => &settings.kiro_model,
        "codewhale" => &settings.codewhale_model,
        "kimi" => &settings.kimi_model,
        "mimo" => &settings.mimo_model,
        _ => return None,
    };
    (!model.trim().is_empty()).then(|| model.clone())
}

fn configured_permission(settings: &ToolPermissionsSummary, agent_id: &str) -> String {
    match agent_id {
        "codex" | "kiro-codex" => &settings.codex,
        "claude" | "kiro-claude" => &settings.claude_code,
        "agy" => &settings.agy,
        "opencode" => &settings.opencode,
        "kiro" => &settings.kiro,
        "codewhale" => &settings.codewhale,
        "kimi" => &settings.kimi,
        "mimo" => &settings.mimo,
        _ => "default",
    }
    .to_string()
}

fn agent_spec(agent_id: &str) -> Result<&'static AgentSpec, ProductControlError> {
    AGENTS
        .iter()
        .find(|agent| agent.id == canonical_agent_id(agent_id))
        .ok_or_else(|| {
            ProductControlError::new(
                "AGENT_NOT_FOUND",
                "the requested Agent is not adapted by WeCode",
                json!({ "agentId": agent_id }),
            )
        })
}

fn ensure_agent_installed(agent: &AgentSpec) -> Result<(), ProductControlError> {
    if find_executable(agent.aliases).is_some() {
        return Ok(());
    }
    Err(ProductControlError::new(
        "AGENT_UNAVAILABLE",
        "the requested Agent executable is not installed or visible to WeCode Desktop",
        json!({ "agentId": agent.id, "command": agent.command }),
    ))
}

fn find_executable(names: &[&str]) -> Option<PathBuf> {
    let mut directories =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    let home = crate::runtime_paths::home_dir();
    directories.extend([
        home.join(".local/bin"),
        home.join("bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ]);
    for directory in directories {
        for name in names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(windows)]
            for extension in ["exe", "cmd", "bat"] {
                let candidate = directory.join(format!("{name}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn probe_agent_version(executable: &Path) -> Option<String> {
    let mut child = Command::new(executable)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let stdout_reader = thread::spawn(move || read_version_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_version_pipe(stderr));
    let started_at = Instant::now();
    let completed = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if started_at.elapsed() < AGENT_VERSION_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if !completed {
        return None;
    }
    parse_agent_version(&stdout).or_else(|| parse_agent_version(&stderr))
}

fn read_version_pipe(mut pipe: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = pipe
        .by_ref()
        .take(AGENT_VERSION_OUTPUT_BYTES as u64)
        .read_to_end(&mut bytes);
    bytes
}

fn parse_agent_version(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| {
            line.chars()
                .filter(|character| !character.is_control())
                .take(AGENT_VERSION_MAX_CHARS)
                .collect::<String>()
        })
        .filter(|line| !line.is_empty())
}

fn canonical_agent_id(source: &str) -> String {
    let source = source.to_ascii_lowercase();
    if matches!(source.as_str(), "kiro-claude" | "kiro_gateway_claude") {
        "kiro-claude"
    } else if matches!(source.as_str(), "kiro-codex" | "kiro_gateway_codex") {
        "kiro-codex"
    } else if source.contains("claude") {
        "claude"
    } else if source.contains("opencode") {
        "opencode"
    } else if source.contains("kiro") {
        "kiro"
    } else if source.contains("codewhale") || source.contains("deepseek") {
        "codewhale"
    } else if source.contains("kimi") {
        "kimi"
    } else if source.contains("mimo") {
        "mimo"
    } else if source.contains("agy") || source.contains("antigravity") {
        "agy"
    } else {
        "codex"
    }
    .to_string()
}

fn history_agent_id(source: &str, model: Option<&str>) -> String {
    let canonical = canonical_agent_id(source);
    let catalog = crate::gateway_service::current_gateway_model_catalog();
    if canonical == "codex"
        && model
            .and_then(|model| catalog.model(model))
            .is_some_and(|model| model.compatibility.codex_cli)
    {
        "kiro-codex".to_string()
    } else {
        canonical
    }
}

fn history_session_can_resume(session: &AISessionSummary) -> bool {
    let agent_id = canonical_agent_id(&session.source);
    let supports_resume = agent_spec(&agent_id)
        .map(|agent| agent.supports_resume)
        .unwrap_or(false);
    if !supports_resume {
        return false;
    }
    let identifier = session
        .external_session_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or(&session.session_key);
    resume_identifier_is_reliable(&agent_id, identifier)
}

fn resume_identifier_is_reliable(agent_id: &str, identifier: &str) -> bool {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return false;
    }
    // Some Codex history rows retain the rollout JSONL path instead of the
    // conversation UUID. `codex resume` does not accept that path as an ID.
    !matches!(agent_id, "codex" | "kiro-codex")
        || (!identifier.contains('/')
            && !identifier.contains('\\')
            && !identifier.ends_with(".jsonl"))
}

fn session_not_active(session_id: &str) -> ProductControlError {
    ProductControlError::new(
        "SESSION_NOT_ACTIVE",
        "the requested session is not attached to a live WeCode terminal",
        json!({ "sessionId": session_id }),
    )
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn command_builder_only_enables_verified_model_and_permission_flags() {
        let codex = agent_spec("codex").unwrap();
        assert_eq!(
            create_command(codex, Some("gpt-5.4"), "fullAccess"),
            "codex --model gpt-5.4 --dangerously-bypass-approvals-and-sandbox"
        );
        let claude = agent_spec("claude-code").unwrap();
        assert_eq!(
            create_command(claude, Some("claude-sonnet-4-6"), "default"),
            "claude --model claude-sonnet-4-6"
        );
    }

    #[test]
    fn kiro_gateway_agents_remain_distinct_from_personal_agents() {
        let claude = agent_spec("kiro-claude").unwrap();
        let codex = agent_spec("kiro-codex").unwrap();
        assert_eq!(gateway_client(claude), Some("claude"));
        assert_eq!(gateway_client(codex), Some("codex"));
        assert_eq!(canonical_agent_id("kiro_gateway_claude"), "kiro-claude");
        assert_eq!(canonical_agent_id("kiro_gateway_codex"), "kiro-codex");

        let catalog = crate::gateway_service::GatewayModelCatalog::fallback();
        assert!(gateway_model_compatible(
            catalog.model("claude-sonnet-5").unwrap(),
            "claude"
        ));
        assert!(!gateway_model_compatible(
            catalog.model("claude-sonnet-5").unwrap(),
            "codex"
        ));
        assert!(gateway_model_compatible(
            catalog.model("gpt-5.6-terra").unwrap(),
            "codex"
        ));

        assert_eq!(
            apply_gateway_permission(
                "claude --permission-mode bypassPermissions --model claude-sonnet-5".to_string(),
                "claude",
                "default",
            ),
            "claude --model claude-sonnet-5"
        );
        assert!(
            apply_gateway_permission(
                "codex --model gpt-5.6-terra".to_string(),
                "codex",
                "fullAccess",
            )
            .ends_with("--dangerously-bypass-approvals-and-sandbox")
        );
    }

    #[test]
    fn codex_resume_rejects_rollout_paths_but_accepts_session_ids() {
        assert!(resume_identifier_is_reliable(
            "codex",
            "3e56270f-527b-4d4f-b447-718fbfb98d64"
        ));
        assert!(!resume_identifier_is_reliable(
            "codex",
            "/Users/example/.codex/sessions/rollout.jsonl"
        ));
        assert!(!resume_identifier_is_reliable(
            "kiro-codex",
            "/Users/example/.codex/sessions/rollout.jsonl"
        ));
    }

    #[test]
    fn legacy_codex_history_uses_kiro_provider_only_for_compatible_models() {
        assert_eq!(
            history_agent_id("codex", Some("gpt-5.6-terra")),
            "kiro-codex"
        );
        assert_eq!(history_agent_id("codex", Some("gpt-5.5")), "codex");
        assert_eq!(
            history_agent_id("kiro-codex", Some("gpt-5.6-terra")),
            "kiro-codex"
        );
    }

    #[test]
    fn terminal_snapshot_strips_ansi_and_control_sequences() {
        assert_eq!(
            strip_terminal_control_sequences("a\u{1b}[31mred\u{1b}[0m\u{1b}]0;title\u{7}\r\nb"),
            "ared\nb"
        );
    }

    #[test]
    fn agent_screen_readiness_rejects_startup_gates() {
        assert!(!agent_screen_accepts_prompt("Press enter to continue"));
        assert!(!agent_screen_accepts_prompt(
            "Do you trust the contents of this directory?"
        ));
        assert!(!agent_screen_accepts_prompt(
            "Starting MCP servers (1/2): codex_apps"
        ));
        assert!(!agent_screen_accepts_prompt(
            "Hooks need review\n5 hooks are new or changed.\nPress enter to confirm or esc to go back"
        ));
        assert!(!agent_screen_accepts_prompt(
            "Finish signing in via your browser"
        ));
        assert!(agent_screen_accepts_prompt(
            "OpenAI Codex\n› Ask anything\ngpt-5.6-sol"
        ));
    }

    #[test]
    fn agent_screen_status_prefers_codex_approval_over_busy_footer() {
        assert_eq!(
            agent_screen_runtime_status(
                "Would you like to run this command?\nPress enter to confirm or esc to cancel"
            ),
            Some("waiting_input")
        );
        assert_eq!(
            agent_screen_runtime_status("Working (42s · esc to interrupt)"),
            Some("running")
        );
        assert_eq!(agent_screen_runtime_status("Ready for input"), None);
    }

    #[test]
    fn agent_prompt_and_enter_are_written_as_separate_pty_frames() {
        let mut frames = Vec::new();
        write_session_prompt("hello agent", Duration::ZERO, |input| {
            frames.push(input.to_vec());
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(frames, vec![b"hello agent".to_vec(), b"\r".to_vec()]);
    }

    #[test]
    fn agent_version_parser_is_bounded_and_uses_first_nonempty_line() {
        let version = parse_agent_version(b"\nCodex CLI 1.2.3\nextra details\n").unwrap();
        assert_eq!(version, "Codex CLI 1.2.3");
        assert!(version.chars().count() <= AGENT_VERSION_MAX_CHARS);
        assert!(parse_agent_version(b"\n\r\n").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn agent_version_probe_executes_the_resolved_binary_without_a_shell() {
        use std::os::unix::fs::PermissionsExt;

        let root =
            std::env::temp_dir().join(format!("wecode-agent-version-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("agent-version");
        fs::write(&executable, "#!/bin/sh\nprintf 'agent 9.8.7\\n'\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            probe_agent_version(&executable).as_deref(),
            Some("agent 9.8.7")
        );
        let _ = fs::remove_dir_all(root);
    }
}
