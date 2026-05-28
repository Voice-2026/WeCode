use crate::ai_runtime::{constants::RUNTIME_EVENT_FILE_MAX_AGE_SECONDS, log::runtime_log_line};
use std::{fs, path::Path};

pub fn drain_runtime_event_dir(dir: &Path, now: f64) -> Vec<Vec<u8>> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let age = fs::metadata(&path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| now - duration.as_secs_f64())
            .unwrap_or(0.0);
        let data = fs::read(&path).ok();
        let _ = fs::remove_file(&path);
        if age > RUNTIME_EVENT_FILE_MAX_AGE_SECONDS {
            runtime_log_line(
                "hook-file",
                &format!(
                    "drop event-file reason=stale age={age:.1}s file={}",
                    path.display()
                ),
            );
            continue;
        }
        if let Some(data) = data.filter(|value| !value.is_empty()) {
            runtime_log_line(
                "hook-file",
                &format!(
                    "drain event-file bytes={} file={}",
                    data.len(),
                    path.display()
                ),
            );
            frames.push(data);
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn drains_runtime_event_files_and_removes_them() {
        let dir = std::env::temp_dir().join(format!("codux-event-drain-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("one.json"), br#"{"kind":"ai-hook"}"#).unwrap();
        fs::write(dir.join("skip.tmp"), b"ignored").unwrap();

        let frames = drain_runtime_event_dir(&dir, now_seconds());

        assert_eq!(frames, vec![br#"{"kind":"ai-hook"}"#.to_vec()]);
        assert!(!dir.join("one.json").exists());
        assert!(dir.join("skip.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    fn now_seconds() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0)
    }
}
