use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalStatusState {
    Idle,
    Working,
    Waiting,
    Completed,
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStatusEvent {
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_instance_id: Option<String>,
    pub state: TerminalStatusState,
    pub updated_at: f64,
    pub source: String,
}
