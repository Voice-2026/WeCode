use crate::config::ConfigDocumentStore;
use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

const AUTOMATIONS_FILE: &str = "automations.json";
const SCHEMA_VERSION: u32 = 3;
const RUN_HISTORY_LIMIT: usize = 200;
const LEGACY_CATCH_UP_GRACE_SECONDS: i64 = 24 * 60 * 60;
pub const DEFAULT_CATCH_UP_GRACE_SECONDS: i64 = 12 * 60 * 60;
const MAX_CATCH_UP_GRACE_SECONDS: i64 = 7 * 24 * 60 * 60;
const DEFAULT_PRECHECK_TIMEOUT_SECONDS: u64 = 60;
const OUTPUT_SNAPSHOT_LIMIT: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDefinition {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub project_id: String,
    pub project_name: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub workspace_path: String,
    #[serde(default)]
    pub workspace_mode: AutomationWorkspaceMode,
    #[serde(default)]
    pub project_path: String,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub reuse_session: bool,
    pub host_device_id: Option<String>,
    pub agent: AutomationAgent,
    pub prompt: String,
    pub precheck_command: Option<String>,
    #[serde(default = "default_precheck_timeout_seconds")]
    pub precheck_timeout_seconds: u64,
    pub schedule: AutomationSchedule,
    pub timezone: String,
    pub missed_run_policy: MissedRunPolicy,
    #[serde(default = "legacy_catch_up_grace_seconds")]
    pub catch_up_grace_seconds: i64,
    pub next_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationWorkspaceMode {
    #[default]
    Existing,
    NewPerRun,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationAgent {
    Claude,
    Codex,
    Kiro,
}

impl AutomationAgent {
    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::Kiro => "Kiro",
        }
    }

    pub fn tool_name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Kiro => "kiro",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutomationSchedulePreset {
    Hourly,
    #[default]
    Daily,
    Weekdays,
    Weekly,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationSchedule {
    Once {
        at: i64,
    },
    Daily {
        hour: u32,
        minute: u32,
    },
    Weekly {
        weekdays: Vec<u32>,
        hour: u32,
        minute: u32,
    },
    Cron {
        expression: String,
    },
}

impl AutomationSchedule {
    pub fn parse(spec: &str, timezone: &str) -> Result<Self, String> {
        let spec = spec.trim();
        let timezone = parse_timezone(timezone)?;
        if let Some(value) = spec.strip_prefix("once:") {
            let value = value.trim().replace('T', " ");
            let local = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M")
                .map_err(|_| "单次任务格式应为 once:YYYY-MM-DD HH:MM".to_string())?;
            let at = timezone
                .from_local_datetime(&local)
                .earliest()
                .ok_or_else(|| "该本地时间在所选时区不存在".to_string())?
                .timestamp();
            return Ok(Self::Once { at });
        }
        if let Some(value) = spec.strip_prefix("daily:") {
            let (hour, minute) = parse_time(value)?;
            return Ok(Self::Daily { hour, minute });
        }
        if let Some(value) = spec.strip_prefix("weekly:") {
            let (days, time) = value
                .split_once('@')
                .ok_or_else(|| "每周任务格式应为 weekly:1,3,5@09:00".to_string())?;
            let mut weekdays = days
                .split(',')
                .map(|day| {
                    day.trim()
                        .parse::<u32>()
                        .map_err(|_| "星期必须使用 1-7，1 表示周一".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if weekdays.is_empty() || weekdays.iter().any(|day| !(1..=7).contains(day)) {
                return Err("星期必须使用 1-7，1 表示周一".to_string());
            }
            weekdays.sort_unstable();
            weekdays.dedup();
            let (hour, minute) = parse_time(time)?;
            return Ok(Self::Weekly {
                weekdays,
                hour,
                minute,
            });
        }
        if let Some(value) = spec.strip_prefix("cron:") {
            CronExpression::parse(value.trim())?;
            return Ok(Self::Cron {
                expression: value.trim().to_string(),
            });
        }
        Err("调度格式支持 once:、daily:、weekly: 或 cron:".to_string())
    }

    pub fn next_after(&self, after: i64, timezone: &str) -> Result<Option<i64>, String> {
        let timezone = parse_timezone(timezone)?;
        match self {
            Self::Once { at } => Ok((*at > after).then_some(*at)),
            Self::Daily { hour, minute } => scan_next_minute(after, timezone, |local| {
                local.hour() == *hour && local.minute() == *minute
            }),
            Self::Weekly {
                weekdays,
                hour,
                minute,
            } => scan_next_minute(after, timezone, |local| {
                weekdays.contains(&local.weekday().number_from_monday())
                    && local.hour() == *hour
                    && local.minute() == *minute
            }),
            Self::Cron { expression } => {
                let cron = CronExpression::parse(expression)?;
                scan_next_minute(after, timezone, |local| cron.matches(local))
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Once { at } => DateTimeLabel::from_timestamp(*at)
                .map(|value| format!("单次 · {value}"))
                .unwrap_or_else(|| "单次".to_string()),
            Self::Daily { hour, minute } => format!("每天 {hour:02}:{minute:02}"),
            Self::Weekly {
                weekdays,
                hour,
                minute,
            } => {
                let days = weekdays
                    .iter()
                    .map(|day| weekday_label(*day))
                    .collect::<Vec<_>>()
                    .join("、");
                format!("{days} {hour:02}:{minute:02}")
            }
            Self::Cron { expression } if expression == "0 * * * *" => "每小时".to_string(),
            Self::Cron { expression } => format!("Cron · {expression}"),
        }
    }

    pub fn preset(&self) -> AutomationSchedulePreset {
        match self {
            Self::Daily { .. } => AutomationSchedulePreset::Daily,
            Self::Weekly { weekdays, .. } if weekdays == &[1, 2, 3, 4, 5] => {
                AutomationSchedulePreset::Weekdays
            }
            Self::Weekly { .. } => AutomationSchedulePreset::Weekly,
            Self::Cron { expression } if expression == "0 * * * *" => {
                AutomationSchedulePreset::Hourly
            }
            Self::Once { .. } | Self::Cron { .. } => AutomationSchedulePreset::Custom,
        }
    }

    pub fn editor_value(&self) -> String {
        match self {
            Self::Daily { hour, minute } => format!("{hour:02}:{minute:02}"),
            Self::Weekly { hour, minute, .. } => format!("{hour:02}:{minute:02}"),
            Self::Cron { expression } if expression == "0 * * * *" => "09:00".to_string(),
            Self::Cron { expression } => expression.clone(),
            Self::Once { at } => DateTimeLabel::from_timestamp(*at).unwrap_or_default(),
        }
    }
}

struct DateTimeLabel;

impl DateTimeLabel {
    fn from_timestamp(timestamp: i64) -> Option<String> {
        chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
    }
}

fn weekday_label(day: u32) -> &'static str {
    match day {
        1 => "周一",
        2 => "周二",
        3 => "周三",
        4 => "周四",
        5 => "周五",
        6 => "周六",
        7 => "周日",
        _ => "未知",
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    CatchUpOnce,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRunTrigger {
    Scheduled,
    Manual,
    CatchUp,
    Rerun,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRunState {
    Scheduled,
    Preparing,
    Running,
    WaitingInput,
    Completed,
    Failed,
    Cancelled,
    SkippedOverlap,
    SkippedPrecheck,
}

impl AutomationRunState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Cancelled
                | Self::SkippedOverlap
                | Self::SkippedPrecheck
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationOutputSnapshot {
    pub content: String,
    pub captured_at: i64,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPrecheckResult {
    pub command: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
}

impl AutomationPrecheckResult {
    pub fn passed(&self) -> bool {
        !self.timed_out && self.error.is_none() && self.exit_code == Some(0)
    }

    pub fn failure_message(&self) -> String {
        if self.timed_out {
            return format!("执行前检查超时（{} ms）", self.duration_ms);
        }
        if let Some(error) = self.error.as_deref() {
            return format!("执行前检查失败：{error}");
        }
        let detail = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };
        if detail.is_empty() {
            format!("执行前检查未通过，退出码 {}", self.exit_code.unwrap_or(-1))
        } else {
            format!("执行前检查未通过：{detail}")
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRun {
    pub id: String,
    pub automation_id: String,
    pub trigger: AutomationRunTrigger,
    pub scheduled_for: i64,
    pub state: AutomationRunState,
    pub state_reason: Option<String>,
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub ai_session_id: Option<String>,
    #[serde(default)]
    pub resumed_from_session_id: Option<String>,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub workspace_name: String,
    #[serde(default)]
    pub workspace_path: String,
    #[serde(default)]
    pub workspace_mode: AutomationWorkspaceMode,
    #[serde(default)]
    pub project_path: String,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub reuse_session: bool,
    #[serde(default)]
    pub agent: Option<AutomationAgent>,
    #[serde(default)]
    pub output_snapshot: Option<AutomationOutputSnapshot>,
    #[serde(default)]
    pub precheck_result: Option<AutomationPrecheckResult>,
    #[serde(default)]
    pub run_number: u32,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationSnapshot {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub definitions: Vec<AutomationDefinition>,
    #[serde(default)]
    pub runs: Vec<AutomationRun>,
}

#[derive(Clone, Debug)]
pub struct AutomationCreateRequest {
    pub name: String,
    pub project_id: String,
    pub project_name: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub workspace_path: String,
    pub workspace_mode: AutomationWorkspaceMode,
    pub project_path: String,
    pub base_branch: Option<String>,
    pub reuse_session: bool,
    pub host_device_id: Option<String>,
    pub agent: AutomationAgent,
    pub prompt: String,
    pub precheck_command: Option<String>,
    pub precheck_timeout_seconds: u64,
    pub schedule_spec: String,
    pub timezone: String,
    pub catch_up_grace_seconds: i64,
}

#[derive(Clone, Debug)]
pub struct AutomationRunPlan {
    pub run_id: String,
    pub automation_id: String,
    pub automation_name: String,
    pub project_id: String,
    pub project_name: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub workspace_path: String,
    pub workspace_mode: AutomationWorkspaceMode,
    pub project_path: String,
    pub base_branch: Option<String>,
    pub resume_session_id: Option<String>,
    pub host_device_id: Option<String>,
    pub agent: AutomationAgent,
    pub prompt: String,
    pub precheck_command: Option<String>,
    pub precheck_timeout_seconds: u64,
}

#[derive(Clone)]
pub struct AutomationService {
    store: Arc<ConfigDocumentStore>,
}

impl AutomationService {
    pub fn for_support_dir(support_dir: impl Into<PathBuf>) -> Self {
        Self {
            store: ConfigDocumentStore::for_file(support_dir.into().join(AUTOMATIONS_FILE)),
        }
    }

    pub fn snapshot(&self) -> AutomationSnapshot {
        let mut snapshot: AutomationSnapshot = self.store.snapshot_as().unwrap_or_default();
        normalize_default_workspace_names(&mut snapshot);
        snapshot
    }

    pub fn create(
        &self,
        request: AutomationCreateRequest,
        now: i64,
    ) -> Result<AutomationDefinition, String> {
        let name = request.name.trim();
        let prompt = request.prompt.trim();
        if name.is_empty() {
            return Err("请输入任务名称".to_string());
        }
        if prompt.is_empty() {
            return Err("请输入任务提示词".to_string());
        }
        let (project_path, base_branch) = normalize_workspace_configuration(
            request.workspace_mode,
            &request.project_path,
            &request.workspace_path,
            request.base_branch.as_deref(),
        )?;
        let timezone = request.timezone.trim().to_string();
        let schedule = AutomationSchedule::parse(&request.schedule_spec, &timezone)?;
        let next_run_at = schedule.next_after(now, &timezone)?;
        if next_run_at.is_none() {
            return Err("调度时间已经过去，请选择未来时间".to_string());
        }
        let definition = AutomationDefinition {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            enabled: true,
            project_id: request.project_id,
            project_name: request.project_name,
            workspace_id: request.workspace_id,
            workspace_name: request.workspace_name,
            workspace_path: request.workspace_path,
            workspace_mode: request.workspace_mode,
            project_path,
            base_branch,
            reuse_session: request.workspace_mode == AutomationWorkspaceMode::Existing
                && request.reuse_session,
            host_device_id: request.host_device_id,
            agent: request.agent,
            prompt: prompt.to_string(),
            precheck_command: request
                .precheck_command
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            precheck_timeout_seconds: request.precheck_timeout_seconds.clamp(1, 30 * 60),
            schedule,
            timezone,
            missed_run_policy: MissedRunPolicy::CatchUpOnce,
            catch_up_grace_seconds: request
                .catch_up_grace_seconds
                .clamp(0, MAX_CATCH_UP_GRACE_SECONDS),
            next_run_at,
            created_at: now,
            updated_at: now,
        };
        self.update(|state| {
            state.definitions.push(definition.clone());
            Ok(())
        })?;
        Ok(definition)
    }

    pub fn update_definition(
        &self,
        id: &str,
        request: AutomationCreateRequest,
        now: i64,
    ) -> Result<AutomationDefinition, String> {
        let name = request.name.trim();
        let prompt = request.prompt.trim();
        if name.is_empty() {
            return Err("请输入任务名称".to_string());
        }
        if prompt.is_empty() {
            return Err("请输入任务提示词".to_string());
        }
        let (project_path, base_branch) = normalize_workspace_configuration(
            request.workspace_mode,
            &request.project_path,
            &request.workspace_path,
            request.base_branch.as_deref(),
        )?;
        let timezone = request.timezone.trim().to_string();
        let schedule = AutomationSchedule::parse(&request.schedule_spec, &timezone)?;
        let next_run_at = schedule.next_after(now, &timezone)?;
        self.update(|state| {
            let definition = state
                .definitions
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or_else(|| "自动任务不存在".to_string())?;
            definition.name = name.to_string();
            definition.project_id = request.project_id;
            definition.project_name = request.project_name;
            definition.workspace_id = request.workspace_id;
            definition.workspace_name = request.workspace_name;
            definition.workspace_path = request.workspace_path;
            definition.workspace_mode = request.workspace_mode;
            definition.project_path = project_path;
            definition.base_branch = base_branch;
            definition.reuse_session = request.workspace_mode == AutomationWorkspaceMode::Existing
                && request.reuse_session;
            definition.host_device_id = request.host_device_id;
            definition.agent = request.agent;
            definition.prompt = prompt.to_string();
            definition.precheck_command = request
                .precheck_command
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            definition.precheck_timeout_seconds =
                request.precheck_timeout_seconds.clamp(1, 30 * 60);
            definition.schedule = schedule;
            definition.timezone = timezone;
            definition.catch_up_grace_seconds = request
                .catch_up_grace_seconds
                .clamp(0, MAX_CATCH_UP_GRACE_SECONDS);
            definition.next_run_at = if definition.enabled {
                next_run_at
            } else {
                None
            };
            definition.updated_at = now;
            Ok(definition.clone())
        })
    }

    pub fn set_enabled(&self, id: &str, enabled: bool, now: i64) -> Result<(), String> {
        self.update(|state| {
            let definition = state
                .definitions
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or_else(|| "自动任务不存在".to_string())?;
            definition.enabled = enabled;
            definition.updated_at = now;
            definition.next_run_at = if enabled {
                definition.schedule.next_after(now, &definition.timezone)?
            } else {
                None
            };
            Ok(())
        })
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        self.update(|state| {
            if has_active_run(state, id) {
                return Err("任务正在运行，结束后才能删除".to_string());
            }
            let before = state.definitions.len();
            state.definitions.retain(|item| item.id != id);
            if state.definitions.len() == before {
                return Err("自动任务不存在".to_string());
            }
            state.runs.retain(|run| run.automation_id != id);
            Ok(())
        })
    }

    pub fn enqueue_manual(&self, id: &str, now: i64) -> Result<AutomationRunPlan, String> {
        self.update(|state| {
            let definition = state
                .definitions
                .iter()
                .find(|item| item.id == id)
                .cloned()
                .ok_or_else(|| "自动任务不存在".to_string())?;
            if has_active_run(state, id) {
                return Err("该任务已有一轮正在运行".to_string());
            }
            let resume_session_id = reusable_session_id(state, &definition);
            let run = new_run(
                &definition,
                AutomationRunTrigger::Manual,
                now,
                next_run_number(state, id),
                resume_session_id,
            );
            let plan = run_plan(&definition, &run);
            state.runs.push(run);
            trim_runs(state);
            Ok(plan)
        })
    }

    pub fn claim_due(&self, now: i64) -> Result<Vec<AutomationRunPlan>, String> {
        if !self.snapshot().definitions.iter().any(|definition| {
            definition.enabled && definition.next_run_at.is_some_and(|next| next <= now)
        }) {
            return Ok(Vec::new());
        }
        self.update(|state| {
            let mut plans = Vec::new();
            let due = state
                .definitions
                .iter()
                .filter(|definition| {
                    definition.enabled && definition.next_run_at.is_some_and(|next| next <= now)
                })
                .cloned()
                .collect::<Vec<_>>();
            for definition in due {
                let scheduled_for = definition.next_run_at.unwrap_or(now);
                let trigger = if scheduled_for < now {
                    AutomationRunTrigger::CatchUp
                } else {
                    AutomationRunTrigger::Scheduled
                };
                let too_old =
                    now.saturating_sub(scheduled_for) > definition.catch_up_grace_seconds.max(0);
                let should_skip_missed = scheduled_for < now
                    && (definition.missed_run_policy == MissedRunPolicy::Skip || too_old);
                let next_from = if should_skip_missed {
                    now
                } else {
                    scheduled_for
                };
                if let Some(saved) = state
                    .definitions
                    .iter_mut()
                    .find(|item| item.id == definition.id)
                {
                    saved.next_run_at = saved.schedule.next_after(next_from, &saved.timezone)?;
                    saved.updated_at = now;
                }
                if should_skip_missed {
                    continue;
                }
                if has_active_run(state, &definition.id) {
                    let mut run = new_run(
                        &definition,
                        AutomationRunTrigger::Scheduled,
                        scheduled_for,
                        next_run_number(state, &definition.id),
                        None,
                    );
                    run.state = AutomationRunState::SkippedOverlap;
                    run.state_reason = Some("上一轮仍在运行，本轮未启动".to_string());
                    run.finished_at = Some(now);
                    state.runs.push(run);
                    continue;
                }
                let resume_session_id = reusable_session_id(state, &definition);
                let run = new_run(
                    &definition,
                    trigger,
                    scheduled_for,
                    next_run_number(state, &definition.id),
                    resume_session_id,
                );
                plans.push(run_plan(&definition, &run));
                state.runs.push(run);
            }
            trim_runs(state);
            Ok(plans)
        })
    }

    pub fn mark_running(&self, run_id: &str, terminal_id: String, now: i64) -> Result<(), String> {
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::Running;
                run.terminal_id = Some(terminal_id);
                run.started_at = Some(now);
                run.state_reason = None;
            }
        })
    }

    pub fn record_run_workspace(
        &self,
        run_id: &str,
        workspace_id: String,
        workspace_name: String,
        workspace_path: String,
    ) -> Result<(), String> {
        if workspace_path.trim().is_empty() {
            return Err("任务工作目录为空".to_string());
        }
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.workspace_id = workspace_id;
                run.workspace_name = workspace_name;
                run.workspace_path = workspace_path;
            }
        })
    }

    pub fn record_run_ai_session(&self, run_id: &str, ai_session_id: String) -> Result<(), String> {
        let ai_session_id = ai_session_id.trim().to_string();
        if ai_session_id.is_empty() {
            return Err("会话 ID 为空".to_string());
        }
        self.update_run(run_id, |run| {
            run.ai_session_id = Some(ai_session_id);
        })
    }

    pub fn record_precheck(
        &self,
        run_id: &str,
        result: AutomationPrecheckResult,
    ) -> Result<(), String> {
        self.update_run(run_id, |run| {
            run.precheck_result = Some(result);
        })
    }

    pub fn mark_skipped_precheck(
        &self,
        run_id: &str,
        result: AutomationPrecheckResult,
        now: i64,
    ) -> Result<(), String> {
        let reason = result.failure_message();
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::SkippedPrecheck;
                run.state_reason = Some(reason);
                run.precheck_result = Some(result);
                run.finished_at = Some(now);
            }
        })
    }

    pub fn mark_failed(&self, run_id: &str, reason: String, now: i64) -> Result<(), String> {
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::Failed;
                run.state_reason = Some(reason);
                run.finished_at = Some(now);
            }
        })
    }

    pub fn mark_completed(&self, run_id: &str, now: i64) -> Result<(), String> {
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::Completed;
                run.state_reason = None;
                run.finished_at = Some(now);
            }
        })
    }

    pub fn mark_completed_with_output(
        &self,
        run_id: &str,
        output_snapshot: Option<AutomationOutputSnapshot>,
        now: i64,
    ) -> Result<(), String> {
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::Completed;
                run.state_reason = None;
                run.output_snapshot = output_snapshot;
                run.finished_at = Some(now);
            }
        })
    }

    pub fn mark_failed_with_output(
        &self,
        run_id: &str,
        reason: String,
        output_snapshot: Option<AutomationOutputSnapshot>,
        now: i64,
    ) -> Result<(), String> {
        self.update_run(run_id, |run| {
            if !run.state.is_terminal() {
                run.state = AutomationRunState::Failed;
                run.state_reason = Some(reason);
                run.output_snapshot = output_snapshot;
                run.finished_at = Some(now);
            }
        })
    }

    pub fn recover_interrupted(&self, now: i64) -> Result<bool, String> {
        self.update(|state| {
            let mut changed = false;
            for run in state.runs.iter_mut().filter(|run| !run.state.is_terminal()) {
                run.state = AutomationRunState::Failed;
                run.state_reason = Some("WeCode 重启，上一轮自动任务已中断".to_string());
                run.finished_at = Some(now);
                changed = true;
            }
            Ok(changed)
        })
    }

    pub fn reconcile_terminals(
        &self,
        terminals: &[(String, bool)],
        now: i64,
    ) -> Result<bool, String> {
        self.update(|state| {
            let mut changed = false;
            for run in state.runs.iter_mut().filter(|run| {
                matches!(
                    run.state,
                    AutomationRunState::Preparing
                        | AutomationRunState::Running
                        | AutomationRunState::WaitingInput
                )
            }) {
                let Some(terminal_id) = run.terminal_id.as_deref() else {
                    continue;
                };
                if terminals
                    .iter()
                    .find(|(id, _)| id == terminal_id)
                    .is_some_and(|(_, running)| !*running)
                {
                    run.state = AutomationRunState::Completed;
                    run.finished_at = Some(now);
                    changed = true;
                }
            }
            Ok(changed)
        })
    }

    fn update_run(
        &self,
        run_id: &str,
        update: impl FnOnce(&mut AutomationRun),
    ) -> Result<(), String> {
        self.update(|state| {
            let run = state
                .runs
                .iter_mut()
                .find(|run| run.id == run_id)
                .ok_or_else(|| "自动任务运行记录不存在".to_string())?;
            update(run);
            Ok(())
        })
    }

    fn update<R>(
        &self,
        update: impl FnOnce(&mut AutomationSnapshot) -> Result<R, String>,
    ) -> Result<R, String> {
        self.store.update(|value| {
            let mut state =
                serde_json::from_value::<AutomationSnapshot>(value.clone()).unwrap_or_default();
            state.schema_version = SCHEMA_VERSION;
            let result = update(&mut state)?;
            *value = serde_json::to_value(state).map_err(|error| error.to_string())?;
            Ok(result)
        })
    }
}

fn normalize_default_workspace_names(snapshot: &mut AutomationSnapshot) {
    for definition in &mut snapshot.definitions {
        if definition.project_path.trim().is_empty() {
            definition.project_path = definition.workspace_path.clone();
        }
        if definition.workspace_id == definition.project_id {
            definition.workspace_name = definition.project_name.clone();
        }
    }
    for run in &mut snapshot.runs {
        let Some(definition) = snapshot
            .definitions
            .iter()
            .find(|definition| definition.id == run.automation_id)
        else {
            continue;
        };
        if run.project_path.trim().is_empty() {
            run.project_path = definition.project_path.clone();
        }
        if run.workspace_id == definition.project_id {
            run.workspace_name = definition.project_name.clone();
        }
    }
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

fn legacy_catch_up_grace_seconds() -> i64 {
    LEGACY_CATCH_UP_GRACE_SECONDS
}

fn default_precheck_timeout_seconds() -> u64 {
    DEFAULT_PRECHECK_TIMEOUT_SECONDS
}

fn normalize_workspace_configuration(
    workspace_mode: AutomationWorkspaceMode,
    project_path: &str,
    workspace_path: &str,
    base_branch: Option<&str>,
) -> Result<(String, Option<String>), String> {
    let project_path = project_path.trim();
    let workspace_path = workspace_path.trim();
    let base_branch = base_branch.map(str::trim).filter(|value| !value.is_empty());
    match workspace_mode {
        AutomationWorkspaceMode::Existing => {
            if workspace_path.is_empty() {
                return Err("任务工作目录为空".to_string());
            }
            Ok((
                if project_path.is_empty() {
                    workspace_path.to_string()
                } else {
                    project_path.to_string()
                },
                base_branch.map(str::to_string),
            ))
        }
        AutomationWorkspaceMode::NewPerRun => {
            if project_path.is_empty() {
                return Err("项目目录为空，无法为每次运行创建 worktree".to_string());
            }
            let base_branch =
                base_branch.ok_or_else(|| "请选择新 worktree 的来源分支".to_string())?;
            Ok((project_path.to_string(), Some(base_branch.to_string())))
        }
    }
}

fn new_run(
    definition: &AutomationDefinition,
    trigger: AutomationRunTrigger,
    scheduled_for: i64,
    run_number: u32,
    resume_session_id: Option<String>,
) -> AutomationRun {
    let id = Uuid::new_v4().to_string();
    let ai_session_id = resume_session_id
        .clone()
        .or_else(|| (definition.agent == AutomationAgent::Claude).then(|| id.clone()));
    AutomationRun {
        id,
        automation_id: definition.id.clone(),
        trigger,
        scheduled_for,
        state: AutomationRunState::Preparing,
        state_reason: None,
        terminal_id: None,
        ai_session_id,
        resumed_from_session_id: resume_session_id,
        workspace_id: definition.workspace_id.clone(),
        workspace_name: definition.workspace_name.clone(),
        workspace_path: definition.workspace_path.clone(),
        workspace_mode: definition.workspace_mode,
        project_path: effective_project_path(definition),
        base_branch: definition.base_branch.clone(),
        reuse_session: definition.reuse_session,
        agent: Some(definition.agent),
        output_snapshot: None,
        precheck_result: None,
        run_number,
        started_at: None,
        finished_at: None,
    }
}

fn next_run_number(state: &AutomationSnapshot, automation_id: &str) -> u32 {
    state
        .runs
        .iter()
        .filter(|run| run.automation_id == automation_id)
        .map(|run| run.run_number)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn run_plan(definition: &AutomationDefinition, run: &AutomationRun) -> AutomationRunPlan {
    AutomationRunPlan {
        run_id: run.id.clone(),
        automation_id: definition.id.clone(),
        automation_name: definition.name.clone(),
        project_id: definition.project_id.clone(),
        project_name: definition.project_name.clone(),
        workspace_id: run.workspace_id.clone(),
        workspace_name: run.workspace_name.clone(),
        workspace_path: run.workspace_path.clone(),
        workspace_mode: run.workspace_mode,
        project_path: if run.project_path.trim().is_empty() {
            effective_project_path(definition)
        } else {
            run.project_path.clone()
        },
        base_branch: run
            .base_branch
            .clone()
            .or_else(|| definition.base_branch.clone()),
        resume_session_id: run.resumed_from_session_id.clone(),
        host_device_id: definition.host_device_id.clone(),
        agent: definition.agent,
        prompt: definition.prompt.clone(),
        precheck_command: definition.precheck_command.clone(),
        precheck_timeout_seconds: definition.precheck_timeout_seconds,
    }
}

fn reusable_session_id(
    state: &AutomationSnapshot,
    definition: &AutomationDefinition,
) -> Option<String> {
    if !definition.reuse_session || definition.workspace_mode != AutomationWorkspaceMode::Existing {
        return None;
    }
    state
        .runs
        .iter()
        .rev()
        .find(|run| {
            run.automation_id == definition.id
                && run.workspace_id == definition.workspace_id
                && run.agent == Some(definition.agent)
                && run
                    .ai_session_id
                    .as_deref()
                    .is_some_and(|id| !id.trim().is_empty())
        })
        .and_then(|run| run.ai_session_id.clone())
}

fn effective_project_path(definition: &AutomationDefinition) -> String {
    if definition.project_path.trim().is_empty() {
        definition.workspace_path.clone()
    } else {
        definition.project_path.clone()
    }
}

fn has_active_run(state: &AutomationSnapshot, automation_id: &str) -> bool {
    state
        .runs
        .iter()
        .any(|run| run.automation_id == automation_id && !run.state.is_terminal())
}

fn trim_runs(state: &mut AutomationSnapshot) {
    if state.runs.len() > RUN_HISTORY_LIMIT {
        state.runs.drain(..state.runs.len() - RUN_HISTORY_LIMIT);
    }
}

pub fn run_automation_precheck(
    command: &str,
    cwd: &str,
    timeout_seconds: u64,
) -> AutomationPrecheckResult {
    let started = Instant::now();
    let mut process = if cfg!(windows) {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    } else {
        let mut process = Command::new("sh");
        process.args(["-lc", command]);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            process.process_group(0);
        }
        process
    };
    process
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            return AutomationPrecheckResult {
                command: command.to_string(),
                exit_code: None,
                timed_out: false,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| spawn_tail_reader(pipe, stdout.clone()));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| spawn_tail_reader(pipe, stderr.clone()));
    let deadline = started + Duration::from_secs(timeout_seconds.clamp(1, 30 * 60));
    let mut timed_out = false;
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(40)),
            Ok(None) => {
                timed_out = true;
                terminate_process_tree(&mut child);
                break child.wait().ok().and_then(|status| status.code());
            }
            Err(error) => {
                terminate_process_tree(&mut child);
                if let Some(handle) = stdout_reader {
                    let _ = handle.join();
                }
                if let Some(handle) = stderr_reader {
                    let _ = handle.join();
                }
                return AutomationPrecheckResult {
                    command: command.to_string(),
                    exit_code: None,
                    timed_out: false,
                    duration_ms: started.elapsed().as_millis() as u64,
                    stdout: sanitize_terminal_output(&bytes_from_mutex(&stdout)),
                    stderr: sanitize_terminal_output(&bytes_from_mutex(&stderr)),
                    error: Some(error.to_string()),
                };
            }
        }
    };
    if let Some(handle) = stdout_reader {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_reader {
        let _ = handle.join();
    }
    AutomationPrecheckResult {
        command: command.to_string(),
        exit_code,
        timed_out,
        duration_ms: started.elapsed().as_millis() as u64,
        stdout: sanitize_terminal_output(&bytes_from_mutex(&stdout)),
        stderr: sanitize_terminal_output(&bytes_from_mutex(&stderr)),
        error: None,
    }
}

fn spawn_tail_reader(
    mut reader: impl Read + Send + 'static,
    output: Arc<Mutex<Vec<u8>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut chunk = [0_u8; 8192];
        loop {
            let count = match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            if let Ok(mut output) = output.lock() {
                output.extend_from_slice(&chunk[..count]);
                if output.len() > OUTPUT_SNAPSHOT_LIMIT {
                    let drain = output.len() - OUTPUT_SNAPSHOT_LIMIT;
                    output.drain(..drain);
                }
            }
        }
    })
}

fn bytes_from_mutex(value: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    value.lock().map(|value| value.clone()).unwrap_or_default()
}

fn terminate_process_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        let group = -(child.id() as i32);
        libc::kill(group, libc::SIGTERM);
        thread::sleep(Duration::from_millis(150));
        if child.try_wait().ok().flatten().is_none() {
            libc::kill(group, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
}

pub fn automation_output_snapshot(raw: &str, captured_at: i64) -> Option<AutomationOutputSnapshot> {
    let content = sanitize_terminal_output(raw.as_bytes());
    if content.trim().is_empty() {
        return None;
    }
    let truncated = raw.len() > OUTPUT_SNAPSHOT_LIMIT;
    let content = if content.len() > OUTPUT_SNAPSHOT_LIMIT {
        String::from_utf8_lossy(&content.as_bytes()[content.len() - OUTPUT_SNAPSHOT_LIMIT..])
            .into_owned()
    } else {
        content
    };
    Some(AutomationOutputSnapshot {
        content,
        captured_at,
        truncated,
    })
}

pub fn sanitize_terminal_output(bytes: &[u8]) -> String {
    let mut output = Vec::with_capacity(bytes.len().min(OUTPUT_SNAPSHOT_LIMIT));
    let mut index = bytes.len().saturating_sub(OUTPUT_SNAPSHOT_LIMIT);
    while index < bytes.len() {
        if bytes[index] == 0x1b {
            index += 1;
            match bytes.get(index).copied() {
                Some(b'[') => {
                    index += 1;
                    while index < bytes.len() {
                        let byte = bytes[index];
                        index += 1;
                        if (0x40..=0x7e).contains(&byte) {
                            break;
                        }
                    }
                }
                Some(b']') => {
                    index += 1;
                    while index < bytes.len() {
                        if bytes[index] == 0x07 {
                            index += 1;
                            break;
                        }
                        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
                            index += 2;
                            break;
                        }
                        index += 1;
                    }
                }
                Some(_) => index += 1,
                None => break,
            }
            continue;
        }
        let byte = bytes[index];
        index += 1;
        if byte == b'\r' {
            continue;
        }
        if byte == b'\n' || byte == b'\t' || byte >= 0x20 {
            output.push(byte);
        }
    }
    String::from_utf8_lossy(&output).trim().to_string()
}

fn parse_timezone(value: &str) -> Result<Tz, String> {
    value
        .trim()
        .parse::<Tz>()
        .map_err(|_| format!("无法识别时区：{}", value.trim()))
}

fn parse_time(value: &str) -> Result<(u32, u32), String> {
    let (hour, minute) = value
        .trim()
        .split_once(':')
        .ok_or_else(|| "时间格式应为 HH:MM".to_string())?;
    let hour = hour
        .parse::<u32>()
        .map_err(|_| "小时必须是 0-23".to_string())?;
    let minute = minute
        .parse::<u32>()
        .map_err(|_| "分钟必须是 0-59".to_string())?;
    if hour > 23 || minute > 59 {
        return Err("时间格式应为 HH:MM".to_string());
    }
    Ok((hour, minute))
}

fn scan_next_minute(
    after: i64,
    timezone: Tz,
    matches: impl Fn(chrono::DateTime<Tz>) -> bool,
) -> Result<Option<i64>, String> {
    let mut candidate = after.div_euclid(60).saturating_add(1).saturating_mul(60);
    const MAX_MINUTES: usize = 366 * 24 * 60 * 2;
    for _ in 0..MAX_MINUTES {
        let utc = Utc
            .timestamp_opt(candidate, 0)
            .single()
            .ok_or_else(|| "调度时间超出支持范围".to_string())?;
        if matches(utc.with_timezone(&timezone)) {
            return Ok(Some(candidate));
        }
        candidate = candidate.saturating_add(60);
    }
    Ok(None)
}

#[derive(Clone, Debug)]
struct CronExpression {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

impl CronExpression {
    fn parse(expression: &str) -> Result<Self, String> {
        let fields = expression.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err("Cron 必须是五段式：分 时 日 月 周".to_string());
        }
        Ok(Self {
            minute: CronField::parse(fields[0], 0, 59)?,
            hour: CronField::parse(fields[1], 0, 23)?,
            day_of_month: CronField::parse(fields[2], 1, 31)?,
            month: CronField::parse(fields[3], 1, 12)?,
            day_of_week: CronField::parse(fields[4], 0, 7)?,
        })
    }

    fn matches(&self, local: chrono::DateTime<Tz>) -> bool {
        let weekday = local.weekday().num_days_from_sunday();
        let day_of_month_matches = self.day_of_month.matches(local.day());
        let day_of_week_matches =
            self.day_of_week.matches(weekday) || (weekday == 0 && self.day_of_week.matches(7));
        let day_matches = match (
            self.day_of_month.unrestricted,
            self.day_of_week.unrestricted,
        ) {
            (true, true) => true,
            (true, false) => day_of_week_matches,
            (false, true) => day_of_month_matches,
            (false, false) => day_of_month_matches || day_of_week_matches,
        };
        self.minute.matches(local.minute())
            && self.hour.matches(local.hour())
            && self.month.matches(local.month())
            && day_matches
    }
}

#[derive(Clone, Debug)]
struct CronField {
    min: u32,
    allowed: Vec<bool>,
    unrestricted: bool,
}

impl CronField {
    fn parse(value: &str, min: u32, max: u32) -> Result<Self, String> {
        let mut allowed = vec![false; (max - min + 1) as usize];
        for part in value.split(',') {
            let (range, step) = part.split_once('/').unwrap_or((part, "1"));
            let step = step
                .parse::<u32>()
                .ok()
                .filter(|step| *step > 0)
                .ok_or_else(|| format!("无效 Cron 步长：{part}"))?;
            let (start, end) = if range == "*" {
                (min, max)
            } else if let Some((start, end)) = range.split_once('-') {
                (
                    start
                        .parse::<u32>()
                        .map_err(|_| format!("无效 Cron 字段：{part}"))?,
                    end.parse::<u32>()
                        .map_err(|_| format!("无效 Cron 字段：{part}"))?,
                )
            } else {
                let number = range
                    .parse::<u32>()
                    .map_err(|_| format!("无效 Cron 字段：{part}"))?;
                (number, number)
            };
            if start < min || end > max || start > end {
                return Err(format!("Cron 字段超出范围：{part}"));
            }
            for number in (start..=end).step_by(step as usize) {
                allowed[(number - min) as usize] = true;
            }
        }
        if !allowed.iter().any(|allowed| *allowed) {
            return Err(format!("Cron 字段为空：{value}"));
        }
        Ok(Self {
            min,
            allowed,
            unrestricted: value == "*",
        })
    }

    fn matches(&self, value: u32) -> bool {
        value
            .checked_sub(self.min)
            .and_then(|index| self.allowed.get(index as usize))
            .copied()
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn temp_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wecode-automation-{label}-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_supported_schedule_specs_and_computes_next_run() {
        let daily = AutomationSchedule::parse("daily:09:30", "Asia/Shanghai").unwrap();
        let weekly = AutomationSchedule::parse("weekly:1,3,5@09:30", "Asia/Shanghai").unwrap();
        let cron = AutomationSchedule::parse("cron:*/15 9-18 * * 1-5", "Asia/Shanghai").unwrap();
        assert!(
            daily
                .next_after(1_700_000_000, "Asia/Shanghai")
                .unwrap()
                .is_some()
        );
        assert!(
            weekly
                .next_after(1_700_000_000, "Asia/Shanghai")
                .unwrap()
                .is_some()
        );
        assert!(
            cron.next_after(1_700_000_000, "Asia/Shanghai")
                .unwrap()
                .is_some()
        );
        assert!(AutomationSchedule::parse("cron:* * *", "UTC").is_err());
    }

    #[test]
    fn store_claims_due_run_once_and_blocks_overlap() {
        let dir = temp_dir("claim");
        let service = AutomationService::for_support_dir(&dir);
        let now = 1_900_000_000;
        let definition = service
            .create(
                AutomationCreateRequest {
                    name: "Daily review".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Project".to_string(),
                    workspace_id: "worktree-1".to_string(),
                    workspace_name: "Project".to_string(),
                    workspace_path: dir.display().to_string(),
                    workspace_mode: AutomationWorkspaceMode::Existing,
                    project_path: dir.display().to_string(),
                    base_branch: None,
                    reuse_session: false,
                    host_device_id: None,
                    agent: AutomationAgent::Claude,
                    prompt: "Review changes".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "cron:* * * * *".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                now,
            )
            .unwrap();
        let due = service.claim_due(now + 60).unwrap();
        assert_eq!(due.len(), 1);
        service
            .mark_running(&due[0].run_id, "terminal-1".to_string(), now + 60)
            .unwrap();
        let second = service.claim_due(now + 120).unwrap();
        assert!(second.is_empty());
        let snapshot = service.snapshot();
        assert!(snapshot.runs.iter().any(|run| {
            run.automation_id == definition.id && run.state == AutomationRunState::SkippedOverlap
        }));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn invalid_task_is_rejected_without_mutating_store() {
        let dir = temp_dir("invalid");
        let service = AutomationService::for_support_dir(&dir);
        let result = service.create(
            AutomationCreateRequest {
                name: String::new(),
                project_id: "p".to_string(),
                project_name: "P".to_string(),
                workspace_id: "p".to_string(),
                workspace_name: "P".to_string(),
                workspace_path: dir.display().to_string(),
                workspace_mode: AutomationWorkspaceMode::Existing,
                project_path: dir.display().to_string(),
                base_branch: None,
                reuse_session: false,
                host_device_id: None,
                agent: AutomationAgent::Codex,
                prompt: "hello".to_string(),
                precheck_command: None,
                precheck_timeout_seconds: 60,
                schedule_spec: "daily:09:00".to_string(),
                timezone: "UTC".to_string(),
                catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
            },
            1_900_000_000,
        );
        assert!(result.is_err());
        assert!(service.snapshot().definitions.is_empty());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn snapshot_uses_project_name_for_default_workspace() {
        let dir = temp_dir("default-workspace-name");
        let service = AutomationService::for_support_dir(&dir);
        let now = 1_900_000_000;
        let definition = service
            .create(
                AutomationCreateRequest {
                    name: "Default workspace".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Forge".to_string(),
                    workspace_id: "project-1".to_string(),
                    workspace_name: "master".to_string(),
                    workspace_path: dir.display().to_string(),
                    workspace_mode: AutomationWorkspaceMode::Existing,
                    project_path: dir.display().to_string(),
                    base_branch: None,
                    reuse_session: false,
                    host_device_id: None,
                    agent: AutomationAgent::Codex,
                    prompt: "Review changes".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "daily:09:00".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                now,
            )
            .unwrap();
        let plan = service.enqueue_manual(&definition.id, now + 1).unwrap();

        let snapshot = service.snapshot();
        assert_eq!(snapshot.definitions[0].workspace_name, "Forge");
        assert_eq!(snapshot.runs[0].id, plan.run_id);
        assert_eq!(snapshot.runs[0].workspace_name, "Forge");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn restart_marks_non_terminal_runs_as_failed() {
        let dir = temp_dir("recover");
        let service = AutomationService::for_support_dir(&dir);
        let now = 1_900_000_000;
        let definition = service
            .create(
                AutomationCreateRequest {
                    name: "Recover".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Project".to_string(),
                    workspace_id: "worktree-1".to_string(),
                    workspace_name: "Project".to_string(),
                    workspace_path: dir.display().to_string(),
                    workspace_mode: AutomationWorkspaceMode::Existing,
                    project_path: dir.display().to_string(),
                    base_branch: None,
                    reuse_session: false,
                    host_device_id: None,
                    agent: AutomationAgent::Codex,
                    prompt: "Review changes".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "daily:09:00".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                now,
            )
            .unwrap();
        let plan = service.enqueue_manual(&definition.id, now + 1).unwrap();
        service
            .mark_running(&plan.run_id, "terminal-1".to_string(), now + 1)
            .unwrap();

        assert!(service.recover_interrupted(now + 2).unwrap());
        let run = service
            .snapshot()
            .runs
            .into_iter()
            .find(|run| run.id == plan.run_id)
            .unwrap();
        assert_eq!(run.state, AutomationRunState::Failed);
        assert!(run.state_reason.unwrap().contains("重启"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn precheck_records_output_and_failure() {
        let dir = temp_dir("precheck");
        let result = run_automation_precheck("printf ready", &dir.display().to_string(), 2);
        assert!(result.passed());
        assert_eq!(result.stdout, "ready");

        let failed =
            run_automation_precheck("printf nope >&2; exit 7", &dir.display().to_string(), 2);
        assert!(!failed.passed());
        assert_eq!(failed.exit_code, Some(7));
        assert!(failed.failure_message().contains("nope"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn output_snapshot_strips_terminal_control_sequences() {
        let snapshot = automation_output_snapshot("\u{1b}[31mhello\u{1b}[0m\r\nworld", 42).unwrap();
        assert_eq!(snapshot.content, "hello\nworld");
        assert!(!snapshot.truncated);
    }

    #[test]
    fn legacy_definition_defaults_to_existing_workspace() {
        let value = serde_json::json!({
            "id": "automation-1",
            "name": "Legacy",
            "enabled": true,
            "projectId": "project-1",
            "projectName": "Project",
            "workspaceId": "workspace-1",
            "workspaceName": "main",
            "workspacePath": "/tmp/project",
            "hostDeviceId": null,
            "agent": "codex",
            "prompt": "Review changes",
            "precheckCommand": null,
            "schedule": { "kind": "daily", "hour": 9, "minute": 0 },
            "timezone": "UTC",
            "missedRunPolicy": "catch_up_once",
            "nextRunAt": 1_900_000_000_i64,
            "createdAt": 1_800_000_000_i64,
            "updatedAt": 1_800_000_000_i64
        });

        let definition: AutomationDefinition = serde_json::from_value(value).unwrap();
        assert_eq!(definition.workspace_mode, AutomationWorkspaceMode::Existing);
        assert!(definition.project_path.is_empty());
        assert_eq!(definition.base_branch, None);
        assert!(!definition.reuse_session);
        assert_eq!(
            definition.catch_up_grace_seconds,
            LEGACY_CATCH_UP_GRACE_SECONDS
        );

        let run = new_run(
            &definition,
            AutomationRunTrigger::Manual,
            1_900_000_000,
            1,
            None,
        );
        let plan = run_plan(&definition, &run);
        assert_eq!(plan.workspace_mode, AutomationWorkspaceMode::Existing);
        assert_eq!(plan.project_path, "/tmp/project");
        assert_eq!(plan.base_branch, None);
    }

    #[test]
    fn new_per_run_requires_source_and_records_created_workspace() {
        let dir = temp_dir("new-per-run");
        let service = AutomationService::for_support_dir(&dir);
        let now = 1_900_000_000;
        let missing_branch = service.create(
            AutomationCreateRequest {
                name: "Isolated review".to_string(),
                project_id: "project-1".to_string(),
                project_name: "Project".to_string(),
                workspace_id: "project-1".to_string(),
                workspace_name: "Project".to_string(),
                workspace_path: dir.display().to_string(),
                workspace_mode: AutomationWorkspaceMode::NewPerRun,
                project_path: dir.display().to_string(),
                base_branch: None,
                reuse_session: false,
                host_device_id: None,
                agent: AutomationAgent::Codex,
                prompt: "Review changes".to_string(),
                precheck_command: None,
                precheck_timeout_seconds: 60,
                schedule_spec: "daily:09:00".to_string(),
                timezone: "UTC".to_string(),
                catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
            },
            now,
        );
        assert!(missing_branch.unwrap_err().contains("来源分支"));

        let definition = service
            .create(
                AutomationCreateRequest {
                    name: "Isolated review".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Project".to_string(),
                    workspace_id: "project-1".to_string(),
                    workspace_name: "Project".to_string(),
                    workspace_path: dir.display().to_string(),
                    workspace_mode: AutomationWorkspaceMode::NewPerRun,
                    project_path: dir.display().to_string(),
                    base_branch: Some("  main  ".to_string()),
                    reuse_session: true,
                    host_device_id: None,
                    agent: AutomationAgent::Codex,
                    prompt: "Review changes".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "daily:09:00".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                now,
            )
            .unwrap();
        assert_eq!(definition.base_branch.as_deref(), Some("main"));
        assert!(!definition.reuse_session);

        let plan = service.enqueue_manual(&definition.id, now + 1).unwrap();
        assert_eq!(plan.workspace_mode, AutomationWorkspaceMode::NewPerRun);
        assert_eq!(plan.project_path, dir.display().to_string());
        assert_eq!(plan.base_branch.as_deref(), Some("main"));

        let created_path = dir.join("automation-worktrees/run-1");
        service
            .record_run_workspace(
                &plan.run_id,
                "run-worktree-1".to_string(),
                "automation/run-1".to_string(),
                created_path.display().to_string(),
            )
            .unwrap();
        let run = service
            .snapshot()
            .runs
            .into_iter()
            .find(|run| run.id == plan.run_id)
            .unwrap();
        assert_eq!(run.workspace_id, "run-worktree-1");
        assert_eq!(run.workspace_name, "automation/run-1");
        assert_eq!(run.workspace_path, created_path.display().to_string());
        assert_eq!(run.workspace_mode, AutomationWorkspaceMode::NewPerRun);
        assert_eq!(run.project_path, dir.display().to_string());
        assert_eq!(run.base_branch.as_deref(), Some("main"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn existing_workspace_can_reuse_the_previous_ai_session() {
        let dir = temp_dir("reuse-session");
        let service = AutomationService::for_support_dir(&dir);
        let now = 1_900_000_000;
        let definition = service
            .create(
                AutomationCreateRequest {
                    name: "Continue research".to_string(),
                    project_id: "project-1".to_string(),
                    project_name: "Project".to_string(),
                    workspace_id: "worktree-1".to_string(),
                    workspace_name: "Project".to_string(),
                    workspace_path: dir.display().to_string(),
                    workspace_mode: AutomationWorkspaceMode::Existing,
                    project_path: dir.display().to_string(),
                    base_branch: None,
                    reuse_session: true,
                    host_device_id: None,
                    agent: AutomationAgent::Codex,
                    prompt: "Continue the report".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "daily:09:00".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                now,
            )
            .unwrap();

        let first = service.enqueue_manual(&definition.id, now + 1).unwrap();
        assert_eq!(first.resume_session_id, None);
        service
            .record_run_ai_session(&first.run_id, "codex-session-1".to_string())
            .unwrap();
        service.mark_completed(&first.run_id, now + 2).unwrap();

        let second = service.enqueue_manual(&definition.id, now + 3).unwrap();
        assert_eq!(second.resume_session_id.as_deref(), Some("codex-session-1"));
        let run = service
            .snapshot()
            .runs
            .into_iter()
            .find(|run| run.id == second.run_id)
            .unwrap();
        assert_eq!(
            run.resumed_from_session_id.as_deref(),
            Some("codex-session-1")
        );
        assert_eq!(run.ai_session_id.as_deref(), Some("codex-session-1"));
        fs::remove_dir_all(dir).ok();
    }
}
