use crate::ai_runtime::{
    constants::TRANSCRIPT_POLL_MINIMUM_SECONDS, snapshot::AISessionSnapshot,
    tool_driver::runtime_resource_paths,
};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptSignature {
    size: u64,
    modified_millis: u128,
}

#[derive(Debug, Clone)]
pub struct TranscriptMonitor {
    paths: Vec<String>,
    signatures: Vec<Option<TranscriptSignature>>,
    signature: Option<TranscriptSignature>,
    last_poll_at: Option<f64>,
}

pub type TranscriptMonitorMap = Arc<Mutex<HashMap<String, TranscriptMonitor>>>;

pub fn refresh_transcript_monitors(
    monitors: &TranscriptMonitorMap,
    sessions: &[AISessionSnapshot],
) {
    let Ok(mut monitors) = monitors.lock() else {
        return;
    };
    let desired = sessions
        .iter()
        .filter_map(|session| {
            let paths = runtime_resource_paths(session)
                .into_iter()
                .map(|path| path.display().to_string())
                .filter(|path| !path.trim().is_empty())
                .collect::<Vec<_>>();
            if paths.is_empty() {
                return None;
            }
            Some((session.terminal_id.clone(), paths))
        })
        .collect::<HashMap<_, _>>();
    monitors.retain(|terminal_id, _| desired.contains_key(terminal_id));
    for (terminal_id, paths) in desired {
        if monitors
            .get(&terminal_id)
            .map(|monitor| monitor.paths == paths)
            .unwrap_or(false)
        {
            continue;
        }
        let signatures = paths
            .iter()
            .map(|path| transcript_signature(Path::new(path)))
            .collect::<Vec<_>>();
        monitors.insert(
            terminal_id,
            TranscriptMonitor {
                signature: signatures.first().cloned().unwrap_or(None),
                signatures,
                paths,
                last_poll_at: None,
            },
        );
    }
}

pub fn scan_transcript_monitors(
    monitors: &mut HashMap<String, TranscriptMonitor>,
    now: f64,
) -> Vec<String> {
    let mut changed = Vec::new();
    for (terminal_id, monitor) in monitors.iter_mut() {
        let signatures = monitor
            .paths
            .iter()
            .map(|path| transcript_signature(Path::new(path)))
            .collect::<Vec<_>>();
        if signatures == monitor.signatures {
            continue;
        }
        if monitor
            .last_poll_at
            .map(|last_poll_at| now - last_poll_at < TRANSCRIPT_POLL_MINIMUM_SECONDS)
            .unwrap_or(false)
        {
            continue;
        }
        monitor.signature = signatures.first().cloned().unwrap_or(None);
        monitor.signatures = signatures;
        monitor.last_poll_at = Some(now);
        changed.push(terminal_id.clone());
    }
    changed
}

pub fn transcript_signature(path: &Path) -> Option<TranscriptSignature> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.is_dir() {
        return directory_signature(path, metadata);
    }
    let modified_millis = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(TranscriptSignature {
        size: metadata.len(),
        modified_millis,
    })
}

fn directory_signature(path: &Path, metadata: fs::Metadata) -> Option<TranscriptSignature> {
    let mut size = 0_u64;
    let mut modified_millis = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    for entry in fs::read_dir(path).ok()?.filter_map(Result::ok) {
        let Ok(child_metadata) = entry.metadata() else {
            continue;
        };
        size = size.saturating_add(1);
        size = size.saturating_add(child_metadata.len());
        if let Ok(modified) = child_metadata.modified().and_then(|time| {
            time.duration_since(std::time::UNIX_EPOCH)
                .map_err(std::io::Error::other)
        }) {
            modified_millis = modified_millis.max(modified.as_millis());
        }
    }
    Some(TranscriptSignature {
        size,
        modified_millis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn scan_transcript_monitor_detects_file_changes_with_cooldown() {
        let dir = std::env::temp_dir().join(format!("codux-transcript-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.jsonl");
        fs::write(&path, "one\n").unwrap();
        let mut monitors = HashMap::from([(
            "term-1".to_string(),
            TranscriptMonitor {
                paths: vec![path.display().to_string()],
                signatures: vec![transcript_signature(&path)],
                signature: transcript_signature(&path),
                last_poll_at: None,
            },
        )]);

        fs::write(&path, "one\ntwo\n").unwrap();
        assert_eq!(
            scan_transcript_monitors(&mut monitors, 100.0),
            vec!["term-1".to_string()]
        );
        fs::write(&path, "one\ntwo\nthree\n").unwrap();
        assert!(scan_transcript_monitors(&mut monitors, 101.0).is_empty());
        assert_eq!(
            scan_transcript_monitors(&mut monitors, 103.0),
            vec!["term-1".to_string()]
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn scan_transcript_monitor_detects_directory_child_changes() {
        let dir = std::env::temp_dir().join(format!("codux-resource-dir-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let mut monitors = HashMap::from([(
            "term-1".to_string(),
            TranscriptMonitor {
                paths: vec![dir.display().to_string()],
                signatures: vec![transcript_signature(&dir)],
                signature: transcript_signature(&dir),
                last_poll_at: None,
            },
        )]);

        fs::write(dir.join("turn-1.json"), "{}").unwrap();
        assert_eq!(
            scan_transcript_monitors(&mut monitors, 100.0),
            vec!["term-1".to_string()]
        );

        fs::remove_dir_all(dir).unwrap();
    }
}
