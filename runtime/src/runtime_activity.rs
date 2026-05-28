use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeActivitySummary {
    pub support_dir: String,
    pub runtime_temp_dir: String,
    pub runtime_root_dir: String,
    pub runtime_support_dir: String,
    pub runtime_log_present: bool,
    pub runtime_log_bytes: u64,
    pub runtime_log_last_modified: Option<String>,
    pub live_log_present: bool,
    pub live_log_bytes: u64,
    pub live_log_last_modified: Option<String>,
    pub runtime_event_count: usize,
    pub runtime_support_files: usize,
    pub running_ai_processes: Vec<RuntimeProcessSummary>,
    pub recent_runtime_lines: Vec<String>,
    pub recent_live_lines: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProcessSummary {
    pub pid: u32,
    pub command: String,
}

pub struct RuntimeActivityService {
    support_dir: PathBuf,
    runtime_temp_dir: PathBuf,
}

impl RuntimeActivityService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            support_dir,
            runtime_temp_dir: runtime_temp_dir(),
        }
    }

    pub fn summary(&self) -> RuntimeActivitySummary {
        let runtime_log = self.support_dir.join("runtime.log");
        let live_log = self.runtime_temp_dir.join("live.log");
        let runtime_root = self.runtime_temp_dir.join("runtime-root");
        let runtime_support = self.support_dir.join("runtime-support");
        let runtime_events = self.runtime_temp_dir.join("runtime-events");

        RuntimeActivitySummary {
            support_dir: self.support_dir.display().to_string(),
            runtime_temp_dir: self.runtime_temp_dir.display().to_string(),
            runtime_root_dir: runtime_root.display().to_string(),
            runtime_support_dir: runtime_support.display().to_string(),
            runtime_log_present: runtime_log.is_file(),
            runtime_log_bytes: file_size(&runtime_log),
            runtime_log_last_modified: modified_label(&runtime_log),
            live_log_present: live_log.is_file(),
            live_log_bytes: file_size(&live_log),
            live_log_last_modified: modified_label(&live_log),
            runtime_event_count: count_files_shallow(&runtime_events),
            runtime_support_files: count_files_recursive(&runtime_support),
            running_ai_processes: running_ai_processes().unwrap_or_default(),
            recent_runtime_lines: tail_lines(&runtime_log, 5).unwrap_or_default(),
            recent_live_lines: tail_lines(&live_log, 5).unwrap_or_default(),
            error: None,
        }
    }
}

fn runtime_temp_dir() -> PathBuf {
    std::env::temp_dir().join(app_slug())
}

fn app_slug() -> &'static str {
    if cfg!(debug_assertions) {
        "codux-dev"
    } else {
        "codux"
    }
}

fn file_size(path: &Path) -> u64 {
    path.metadata().map(|metadata| metadata.len()).unwrap_or(0)
}

fn modified_label(path: &Path) -> Option<String> {
    let modified = path.metadata().ok()?.modified().ok()?;
    let seconds = modified.duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
    Some(format!("{seconds:.3}"))
}

fn count_files_shallow(path: &Path) -> usize {
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        })
        .count()
}

fn count_files_recursive(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                count_files_recursive(&path)
            } else {
                1
            }
        })
        .sum()
}

fn tail_lines(path: &Path, limit: usize) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut lines = content
        .lines()
        .rev()
        .take(limit)
        .map(|line| line.chars().take(140).collect::<String>())
        .collect::<Vec<_>>();
    lines.reverse();
    Ok(lines)
}

fn running_ai_processes() -> Result<Vec<RuntimeProcessSummary>, String> {
    #[cfg(target_os = "windows")]
    {
        return Ok(Vec::new());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,command="])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text
            .lines()
            .filter_map(parse_process_line)
            .filter(|process| is_ai_runtime_process(&process.command))
            .take(12)
            .collect())
    }
}

fn parse_process_line(line: &str) -> Option<RuntimeProcessSummary> {
    let trimmed = line.trim();
    let (pid, command) = trimmed.split_once(' ')?;
    Some(RuntimeProcessSummary {
        pid: pid.trim().parse().ok()?,
        command: command.trim().chars().take(180).collect(),
    })
}

fn is_ai_runtime_process(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    ["codex", "claude", "gemini", "opencode", "kiro", "agy"]
        .iter()
        .any(|needle| lower.contains(needle))
        && !lower.contains("codux-gpui-terminal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn summary_reads_runtime_logs_and_support_files() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-gpui-runtime-test-{}", Uuid::new_v4()));
        fs::create_dir_all(support_dir.join("runtime-support/runtime-hooks")).unwrap();
        fs::write(support_dir.join("runtime.log"), "one\ntwo\nthree\n").unwrap();
        fs::write(
            support_dir.join("runtime-support/runtime-hooks/dmux-ai-state.sh"),
            "#!/bin/sh\n",
        )
        .unwrap();

        let service = RuntimeActivityService {
            support_dir: support_dir.clone(),
            runtime_temp_dir: support_dir.join("tmp"),
        };
        fs::create_dir_all(service.runtime_temp_dir.join("runtime-events")).unwrap();
        fs::write(
            service.runtime_temp_dir.join("live.log"),
            "live-one\nlive-two\n",
        )
        .unwrap();
        fs::write(
            service.runtime_temp_dir.join("runtime-events/event.json"),
            "{}",
        )
        .unwrap();

        let summary = service.summary();

        assert!(summary.runtime_log_present);
        assert!(summary.live_log_present);
        assert_eq!(summary.runtime_event_count, 1);
        assert_eq!(summary.runtime_support_files, 1);
        assert_eq!(summary.recent_runtime_lines, vec!["one", "two", "three"]);
        assert_eq!(summary.recent_live_lines, vec!["live-one", "live-two"]);

        fs::remove_dir_all(support_dir).unwrap();
    }

    #[test]
    fn parses_ai_process_lines() {
        let process = parse_process_line(" 1234 /usr/bin/codex --foo").unwrap();
        assert_eq!(process.pid, 1234);
        assert!(is_ai_runtime_process(&process.command));
        assert!(!is_ai_runtime_process("target/debug/codux-gpui-terminal"));
    }
}
