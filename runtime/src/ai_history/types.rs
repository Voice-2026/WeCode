use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIHistorySummary {
    pub indexed: bool,
    pub indexed_at: Option<f64>,
    pub project_total_tokens: i64,
    pub project_cached_input_tokens: i64,
    pub today_total_tokens: i64,
    pub today_cached_input_tokens: i64,
    pub session_count: usize,
    pub sessions: Vec<AISessionSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIGlobalHistorySummary {
    pub indexed_project_count: usize,
    pub session_count: usize,
    pub total_tokens: i64,
    pub cached_input_tokens: i64,
    pub today_total_tokens: i64,
    pub today_cached_input_tokens: i64,
    pub project_totals: Vec<AIProjectUsageSummary>,
    pub recent_sessions: Vec<AISessionSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProjectUsageSummary {
    pub project_path: String,
    pub project_name: String,
    pub session_count: usize,
    pub total_tokens: i64,
    pub cached_input_tokens: i64,
    pub today_total_tokens: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AISessionSummary {
    pub id: String,
    pub session_key: String,
    pub external_session_id: Option<String>,
    pub title: String,
    pub source: String,
    pub last_model: Option<String>,
    pub last_seen_at: f64,
    pub total_tokens: i64,
    pub cached_input_tokens: i64,
    pub request_count: i64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AISessionDetail {
    pub id: String,
    pub title: String,
    pub source: String,
    pub session_key: String,
    pub external_session_id: Option<String>,
    pub first_seen_at: Option<f64>,
    pub last_seen_at: Option<f64>,
    pub active_duration_seconds: i64,
    pub total_tokens: i64,
    pub cached_input_tokens: i64,
    pub request_count: i64,
    pub files: Vec<AISessionFileSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AISessionFileSummary {
    pub file_path: String,
    pub model: String,
    pub first_seen_at: Option<f64>,
    pub last_seen_at: Option<f64>,
    pub total_tokens: i64,
    pub cached_input_tokens: i64,
    pub request_count: i64,
}

#[derive(Clone, Debug)]
pub(super) struct SessionLink {
    pub(super) source: String,
    pub(super) session_key: String,
    pub(super) external_session_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct SessionDetailLink {
    pub(super) source: String,
    pub(super) file_path: String,
    pub(super) session_key: String,
    pub(super) external_session_id: Option<String>,
    pub(super) title: String,
    pub(super) first_seen_at: Option<f64>,
    pub(super) last_seen_at: Option<f64>,
    pub(super) last_model: Option<String>,
    pub(super) active_duration_seconds: i64,
}
