use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const LOCAL_CONTROL_PROTOCOL_VERSION: &str = "1";
pub const LOCAL_CONTROL_MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const LOCAL_CONTROL_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const LOCAL_CONTROL_MAX_REQUEST_ID_CHARS: usize = 128;
pub const LOCAL_CONTROL_CAPABILITY_APP_STATUS: &str = "app.status.v1";
pub const LOCAL_CONTROL_CAPABILITY_PROJECT_LIST: &str = "project.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_WORKTREE_LIST: &str = "worktree.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_WORKTREE_CREATE: &str = "worktree.create.v1";
pub const LOCAL_CONTROL_CAPABILITY_WORKTREE_MERGE: &str = "worktree.merge.v1";
pub const LOCAL_CONTROL_CAPABILITY_WORKTREE_REMOVE: &str = "worktree.remove.v1";
pub const LOCAL_CONTROL_CAPABILITY_AGENT_LIST: &str = "agent.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_MODEL_LIST: &str = "model.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_LIST: &str = "session.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_CREATE: &str = "session.create.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_RESUME: &str = "session.resume.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_SEND: &str = "session.send.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_STATUS: &str = "session.status.v1";
pub const LOCAL_CONTROL_CAPABILITY_SESSION_STOP: &str = "session.stop.v1";
pub const LOCAL_CONTROL_CAPABILITY_TERMINAL_LIST: &str = "terminal.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_TERMINAL_CREATE: &str = "terminal.create.v1";
pub const LOCAL_CONTROL_CAPABILITY_TERMINAL_SEND: &str = "terminal.send.v1";
pub const LOCAL_CONTROL_CAPABILITY_TERMINAL_SNAPSHOT: &str = "terminal.snapshot.v1";
pub const LOCAL_CONTROL_CAPABILITY_TERMINAL_CLOSE: &str = "terminal.close.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_LIST: &str = "automation.list.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_CREATE: &str = "automation.create.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_UPDATE: &str = "automation.update.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_RUN: &str = "automation.run.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_PAUSE: &str = "automation.pause.v1";
pub const LOCAL_CONTROL_CAPABILITY_AUTOMATION_RESUME: &str = "automation.resume.v1";
pub const LOCAL_CONTROL_METHOD_APP_STATUS: &str = "app.status";
pub const LOCAL_CONTROL_METHOD_PROJECT_LIST: &str = "project.list";
pub const LOCAL_CONTROL_METHOD_WORKTREE_LIST: &str = "worktree.list";
pub const LOCAL_CONTROL_METHOD_WORKTREE_CREATE: &str = "worktree.create";
pub const LOCAL_CONTROL_METHOD_WORKTREE_MERGE: &str = "worktree.merge";
pub const LOCAL_CONTROL_METHOD_WORKTREE_REMOVE: &str = "worktree.remove";
pub const LOCAL_CONTROL_METHOD_AGENT_LIST: &str = "agent.list";
pub const LOCAL_CONTROL_METHOD_MODEL_LIST: &str = "model.list";
pub const LOCAL_CONTROL_METHOD_SESSION_LIST: &str = "session.list";
pub const LOCAL_CONTROL_METHOD_SESSION_CREATE: &str = "session.create";
pub const LOCAL_CONTROL_METHOD_SESSION_RESUME: &str = "session.resume";
pub const LOCAL_CONTROL_METHOD_SESSION_SEND: &str = "session.send";
pub const LOCAL_CONTROL_METHOD_SESSION_STATUS: &str = "session.status";
pub const LOCAL_CONTROL_METHOD_SESSION_STOP: &str = "session.stop";
pub const LOCAL_CONTROL_METHOD_TERMINAL_LIST: &str = "terminal.list";
pub const LOCAL_CONTROL_METHOD_TERMINAL_CREATE: &str = "terminal.create";
pub const LOCAL_CONTROL_METHOD_TERMINAL_SEND: &str = "terminal.send";
pub const LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT: &str = "terminal.snapshot";
pub const LOCAL_CONTROL_METHOD_TERMINAL_CLOSE: &str = "terminal.close";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_LIST: &str = "automation.list";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_CREATE: &str = "automation.create";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE: &str = "automation.update";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_RUN: &str = "automation.run";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE: &str = "automation.pause";
pub const LOCAL_CONTROL_METHOD_AUTOMATION_RESUME: &str = "automation.resume";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlRequest {
    pub protocol_version: String,
    pub request_id: String,
    pub auth_token: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlResponse {
    pub ok: bool,
    pub request_id: String,
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LocalControlError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub details: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlAppStatus {
    pub product: String,
    pub version: String,
    pub protocol_version: String,
    pub instance_id: String,
    pub pid: u32,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlProjectParams {
    pub project_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlWorktreeCreateParams {
    pub project_id: String,
    pub branch_name: String,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub task_title: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlWorktreeMutationParams {
    pub project_id: String,
    pub worktree_id: String,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub remove_branch: bool,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlModelListParams {
    pub agent_id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlSessionListParams {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlSessionCreateParams {
    pub project_id: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
    pub agent_id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub permission_mode: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlSessionResumeParams {
    pub session_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlSessionSendParams {
    pub session_id: String,
    pub prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlSessionTargetParams {
    pub session_id: String,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlTerminalListParams {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlTerminalCreateParams {
    pub project_id: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlTerminalSendParams {
    pub terminal_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub enter: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlTerminalSnapshotParams {
    pub terminal_id: String,
    #[serde(default)]
    pub tail: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlTerminalTargetParams {
    pub terminal_id: String,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlAutomationTargetParams {
    pub automation_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlAutomationCreateParams {
    pub name: String,
    pub project_id: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub workspace_mode: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub reuse_session: bool,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub precheck_command: Option<String>,
    #[serde(default)]
    pub precheck_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub catch_up_grace_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalControlAutomationUpdateParams {
    pub automation_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub workspace_mode: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub reuse_session: Option<bool>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub precheck_command: Option<String>,
    #[serde(default)]
    pub precheck_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub catch_up_grace_seconds: Option<i64>,
}

impl LocalControlResponse {
    pub fn success(request_id: impl Into<String>, data: Value) -> Self {
        Self {
            ok: true,
            request_id: request_id.into(),
            protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            ok: false,
            request_id: request_id.into(),
            protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
            data: None,
            error: Some(LocalControlError {
                code: code.into(),
                message: message.into(),
                details,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn response_envelope_omits_the_unused_branch() {
        let success = serde_json::to_value(LocalControlResponse::success("r1", json!({}))).unwrap();
        assert_eq!(success["ok"], true);
        assert!(success.get("error").is_none());

        let failure = serde_json::to_value(LocalControlResponse::failure(
            "r2",
            "DESKTOP_OFFLINE",
            "offline",
            Value::Null,
        ))
        .unwrap();
        assert_eq!(failure["ok"], false);
        assert!(failure.get("data").is_none());
    }

    #[test]
    fn protocol_limits_remain_bounded() {
        assert!(LOCAL_CONTROL_MAX_REQUEST_BYTES < LOCAL_CONTROL_MAX_RESPONSE_BYTES);
        assert!(LOCAL_CONTROL_MAX_REQUEST_ID_CHARS <= 128);
    }
}
