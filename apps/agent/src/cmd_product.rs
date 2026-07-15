use serde_json::{Value, json};
use wecode_protocol::{
    LOCAL_CONTROL_METHOD_AGENT_LIST, LOCAL_CONTROL_METHOD_AUTOMATION_CREATE,
    LOCAL_CONTROL_METHOD_AUTOMATION_LIST, LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE,
    LOCAL_CONTROL_METHOD_AUTOMATION_RESUME, LOCAL_CONTROL_METHOD_AUTOMATION_RUN,
    LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE, LOCAL_CONTROL_METHOD_MODEL_LIST,
    LOCAL_CONTROL_METHOD_PROJECT_LIST, LOCAL_CONTROL_METHOD_SESSION_CREATE,
    LOCAL_CONTROL_METHOD_SESSION_LIST, LOCAL_CONTROL_METHOD_SESSION_RESUME,
    LOCAL_CONTROL_METHOD_SESSION_SEND, LOCAL_CONTROL_METHOD_SESSION_STATUS,
    LOCAL_CONTROL_METHOD_SESSION_STOP, LOCAL_CONTROL_METHOD_TERMINAL_CLOSE,
    LOCAL_CONTROL_METHOD_TERMINAL_CREATE, LOCAL_CONTROL_METHOD_TERMINAL_LIST,
    LOCAL_CONTROL_METHOD_TERMINAL_SEND, LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT,
    LOCAL_CONTROL_METHOD_WORKTREE_CREATE, LOCAL_CONTROL_METHOD_WORKTREE_LIST,
    LOCAL_CONTROL_METHOD_WORKTREE_MERGE, LOCAL_CONTROL_METHOD_WORKTREE_REMOVE,
    LocalControlAutomationCreateParams, LocalControlAutomationUpdateParams, LocalControlResponse,
};

use crate::cmd_app;

pub fn project_list(json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_PROJECT_LIST,
        json!({}),
        json_output,
        print_projects,
    )
}

pub fn agent_list(json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_AGENT_LIST,
        json!({}),
        json_output,
        print_agents,
    )
}

pub fn model_list(agent_id: String, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_MODEL_LIST,
        json!({ "agentId": agent_id }),
        json_output,
        print_models,
    )
}

pub fn session_list(
    project_id: Option<String>,
    worktree_id: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_LIST,
        json!({ "projectId": project_id, "worktreeId": worktree_id }),
        json_output,
        print_sessions,
    )
}

pub fn session_create(
    project_id: String,
    worktree_id: Option<String>,
    agent_id: String,
    model: Option<String>,
    permission_mode: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_CREATE,
        json!({
            "projectId": project_id,
            "worktreeId": worktree_id,
            "agentId": agent_id,
            "model": model,
            "permissionMode": permission_mode,
        }),
        json_output,
        print_session,
    )
}

pub fn session_resume(
    session_id: String,
    project_id: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_RESUME,
        json!({ "sessionId": session_id, "projectId": project_id }),
        json_output,
        print_session,
    )
}

pub fn session_send(session_id: String, prompt: String, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_SEND,
        json!({ "sessionId": session_id, "prompt": prompt }),
        json_output,
        print_session,
    )
}

pub fn session_status(session_id: String, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_STATUS,
        json!({ "sessionId": session_id }),
        json_output,
        print_session,
    )
}

pub fn session_stop(session_id: String, confirmed: bool, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_SESSION_STOP,
        json!({ "sessionId": session_id, "confirmed": confirmed }),
        json_output,
        print_session,
    )
}

pub fn terminal_list(
    project_id: Option<String>,
    worktree_id: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_TERMINAL_LIST,
        json!({ "projectId": project_id, "worktreeId": worktree_id }),
        json_output,
        print_terminals,
    )
}

pub fn terminal_create(
    project_id: String,
    worktree_id: Option<String>,
    command: Option<String>,
    title: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_TERMINAL_CREATE,
        json!({
            "projectId": project_id,
            "worktreeId": worktree_id,
            "command": command,
            "title": title,
        }),
        json_output,
        print_terminal,
    )
}

pub fn terminal_send(
    terminal_id: String,
    text: String,
    enter: bool,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_TERMINAL_SEND,
        json!({ "terminalId": terminal_id, "text": text, "enter": enter }),
        json_output,
        print_terminal,
    )
}

pub fn terminal_snapshot(
    terminal_id: String,
    tail: Option<usize>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT,
        json!({ "terminalId": terminal_id, "tail": tail }),
        json_output,
        print_terminal_snapshot,
    )
}

pub fn terminal_close(
    terminal_id: String,
    confirmed: bool,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_TERMINAL_CLOSE,
        json!({ "terminalId": terminal_id, "confirmed": confirmed }),
        json_output,
        print_terminal,
    )
}

pub fn automation_list(json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_AUTOMATION_LIST,
        json!({}),
        json_output,
        print_automations,
    )
}

pub fn automation_create(
    params: LocalControlAutomationCreateParams,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_AUTOMATION_CREATE,
        serde_json::to_value(params).map_err(|error| error.to_string())?,
        json_output,
        print_automation,
    )
}

pub fn automation_update(
    params: LocalControlAutomationUpdateParams,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE,
        serde_json::to_value(params).map_err(|error| error.to_string())?,
        json_output,
        print_automation,
    )
}

pub fn automation_run(automation_id: String, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_AUTOMATION_RUN,
        json!({ "automationId": automation_id }),
        json_output,
        print_automation,
    )
}

pub fn automation_set_enabled(
    method: &str,
    automation_id: String,
    json_output: bool,
) -> Result<(), String> {
    debug_assert!(
        method == LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE
            || method == LOCAL_CONTROL_METHOD_AUTOMATION_RESUME
    );
    run(
        method,
        json!({ "automationId": automation_id }),
        json_output,
        print_automation,
    )
}

pub fn worktree_list(project_id: String, json_output: bool) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_WORKTREE_LIST,
        json!({ "projectId": project_id }),
        json_output,
        print_worktrees,
    )
}

pub fn worktree_create(
    project_id: String,
    branch_name: String,
    base_branch: Option<String>,
    task_title: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    run(
        LOCAL_CONTROL_METHOD_WORKTREE_CREATE,
        json!({
            "projectId": project_id,
            "branchName": branch_name,
            "baseBranch": base_branch,
            "taskTitle": task_title,
        }),
        json_output,
        print_worktrees,
    )
}

pub fn worktree_mutate(
    method: &str,
    project_id: String,
    worktree_id: String,
    base_branch: Option<String>,
    remove_branch: bool,
    confirmed: bool,
    json_output: bool,
) -> Result<(), String> {
    debug_assert!(
        method == LOCAL_CONTROL_METHOD_WORKTREE_MERGE
            || method == LOCAL_CONTROL_METHOD_WORKTREE_REMOVE
    );
    run(
        method,
        json!({
            "projectId": project_id,
            "worktreeId": worktree_id,
            "baseBranch": base_branch,
            "removeBranch": remove_branch,
            "confirmed": confirmed,
        }),
        json_output,
        print_worktrees,
    )
}

fn run(
    method: &str,
    params: Value,
    json_output: bool,
    human_printer: fn(&Value),
) -> Result<(), String> {
    match cmd_app::request(method, params) {
        Ok(response) if response.ok => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string(&response)
                        .map_err(|error| format!("encode local control response: {error}"))?
                );
            } else if let Some(data) = response.data.as_ref() {
                human_printer(data);
            }
            Ok(())
        }
        Ok(response) => {
            if !json_output {
                print_confirmation_preview(&response);
            }
            cmd_app::handle_response_error(response, json_output)
        }
        Err(error) => cmd_app::offline(error, json_output),
    }
}

fn print_projects(data: &Value) {
    let projects = data
        .get("projects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if projects.is_empty() {
        println!("No projects registered in WeCode Desktop.");
        return;
    }
    for project in projects {
        println!(
            "{}\t{}\t{}\t{}",
            project.get("id").and_then(Value::as_str).unwrap_or("-"),
            project.get("name").and_then(Value::as_str).unwrap_or("-"),
            project.get("branch").and_then(Value::as_str).unwrap_or("-"),
            project.get("path").and_then(Value::as_str).unwrap_or("-"),
        );
    }
}

fn print_agents(data: &Value) {
    for agent in data
        .get("agents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        println!(
            "{}\t{}\t{}\t{}",
            agent.get("id").and_then(Value::as_str).unwrap_or("-"),
            if agent
                .get("installed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "installed"
            } else {
                "unavailable"
            },
            agent
                .get("configuredModel")
                .and_then(Value::as_str)
                .unwrap_or("default"),
            agent
                .get("permissionMode")
                .and_then(Value::as_str)
                .unwrap_or("default"),
        );
    }
}

fn print_models(data: &Value) {
    let models = data
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if models.is_empty() {
        println!("No explicit model is configured; omit --model to use the Agent default.");
        return;
    }
    for model in models {
        println!(
            "{}\t{}",
            model.get("id").and_then(Value::as_str).unwrap_or("-"),
            model.get("source").and_then(Value::as_str).unwrap_or("-")
        );
    }
}

fn print_sessions(data: &Value) {
    let sessions = data
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if sessions.is_empty() {
        println!("No Agent sessions found.");
        return;
    }
    for session in sessions {
        print_session(&session);
    }
}

fn print_session(data: &Value) {
    println!(
        "{}\t{}\t{}\t{}\t{}",
        data.get("id").and_then(Value::as_str).unwrap_or("-"),
        data.get("agentId").and_then(Value::as_str).unwrap_or("-"),
        data.get("model")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        data.get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        data.get("title").and_then(Value::as_str).unwrap_or("-"),
    );
}

fn print_terminals(data: &Value) {
    let terminals = data
        .get("terminals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if terminals.is_empty() {
        println!("No ordinary terminals found.");
        return;
    }
    for terminal in terminals {
        print_terminal(&terminal);
    }
}

fn print_terminal(data: &Value) {
    println!(
        "{}\t{}\t{}\t{}",
        data.get("terminalId")
            .or_else(|| data.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("-"),
        data.get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        data.get("title").and_then(Value::as_str).unwrap_or("-"),
        data.get("cwd").and_then(Value::as_str).unwrap_or("-"),
    );
}

fn print_terminal_snapshot(data: &Value) {
    if let Some(text) = data.get("text").and_then(Value::as_str) {
        print!("{text}");
        if !text.is_empty() && !text.ends_with('\n') {
            println!();
        }
    }
}

fn print_automations(data: &Value) {
    let automations = data
        .get("automations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if automations.is_empty() {
        println!("No automations configured in WeCode Desktop.");
        return;
    }
    for automation in automations {
        print_automation(&automation);
    }
}

fn print_automation(data: &Value) {
    let latest_state = data
        .get("latestRun")
        .and_then(|run| run.get("state"))
        .or_else(|| data.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("never");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        data.get("automationId")
            .or_else(|| data.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("-"),
        if data.get("enabled").and_then(Value::as_bool).unwrap_or(true) {
            "enabled"
        } else {
            "paused"
        },
        latest_state,
        data.get("agentId").and_then(Value::as_str).unwrap_or("-"),
        data.get("model").and_then(Value::as_str).unwrap_or("-"),
        data.get("name").and_then(Value::as_str).unwrap_or("-"),
    );
}

fn print_worktrees(data: &Value) {
    let worktrees = data
        .get("worktrees")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if worktrees.is_empty() {
        println!("No worktrees found.");
        return;
    }
    for worktree in worktrees {
        println!(
            "{}\t{}\t{}\t{}",
            worktree.get("id").and_then(Value::as_str).unwrap_or("-"),
            worktree
                .get("branch")
                .and_then(Value::as_str)
                .unwrap_or("-"),
            worktree
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("-"),
            worktree.get("path").and_then(Value::as_str).unwrap_or("-"),
        );
    }
}

fn print_confirmation_preview(response: &LocalControlResponse) {
    let Some(error) = response.error.as_ref() else {
        return;
    };
    if error.code != "CONFIRMATION_REQUIRED" {
        return;
    }
    let details = &error.details;
    if details.get("kind").and_then(Value::as_str) == Some("active") {
        eprintln!("Confirmation required:");
        eprintln!(
            "  session: {} [{}]",
            details.get("id").and_then(Value::as_str).unwrap_or("-"),
            details
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
        eprintln!(
            "  agent: {}, project: {}",
            details
                .get("agentId")
                .and_then(Value::as_str)
                .unwrap_or("-"),
            details
                .get("projectName")
                .and_then(Value::as_str)
                .unwrap_or("-")
        );
        return;
    }
    if details.get("kind").and_then(Value::as_str) == Some("terminal") {
        eprintln!("Confirmation required:");
        eprintln!(
            "  terminal: {} [{}]",
            details
                .get("terminalId")
                .and_then(Value::as_str)
                .unwrap_or("-"),
            details
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
        eprintln!(
            "  title: {}, cwd: {}",
            details.get("title").and_then(Value::as_str).unwrap_or("-"),
            details.get("cwd").and_then(Value::as_str).unwrap_or("-")
        );
        return;
    }
    eprintln!("Confirmation required:");
    eprintln!(
        "  project: {} ({})",
        details
            .get("projectName")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        details
            .get("projectId")
            .and_then(Value::as_str)
            .unwrap_or("-")
    );
    eprintln!(
        "  worktree: {} [{}]",
        details
            .get("worktreeName")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        details.get("branch").and_then(Value::as_str).unwrap_or("-")
    );
    if let Some(summary) = details.get("gitSummary") {
        eprintln!(
            "  changes: {}, incoming: {}, outgoing: {}",
            summary.get("changes").and_then(Value::as_u64).unwrap_or(0),
            summary.get("incoming").and_then(Value::as_i64).unwrap_or(0),
            summary.get("outgoing").and_then(Value::as_i64).unwrap_or(0),
        );
    }
}
