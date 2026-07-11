use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

const AI_SESSION_METADATA_NAMESPACE: &str = "ai-session-metadata";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AISessionMetadata {
    pub session_id: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default = "default_retention")]
    pub retention: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub updated_at: i64,
}

pub struct AISessionMetadataService {
    support_dir: PathBuf,
}

impl AISessionMetadataService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self { support_dir }
    }

    pub fn list(&self) -> HashMap<String, AISessionMetadata> {
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())
            .and_then(|cache| cache.scan_json(AI_SESSION_METADATA_NAMESPACE))
            .unwrap_or_default()
            .into_iter()
            .map(|(session_id, metadata)| (session_id, sanitize_metadata(metadata)))
            .collect()
    }

    pub fn set_pinned(&self, session_id: &str, pinned: bool) -> Result<AISessionMetadata, String> {
        self.update(session_id, |metadata| metadata.pinned = pinned)
    }

    pub fn set_retention(
        &self,
        session_id: &str,
        retention: &str,
    ) -> Result<AISessionMetadata, String> {
        let retention = normalized_retention(retention).to_string();
        self.update(session_id, move |metadata| metadata.retention = retention)
    }

    pub fn set_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<AISessionMetadata, String> {
        self.update(session_id, |metadata| metadata.archived = archived)
    }

    fn update(
        &self,
        session_id: &str,
        update: impl FnOnce(&mut AISessionMetadata),
    ) -> Result<AISessionMetadata, String> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err("AI session id is required.".to_string());
        }
        let cache = crate::persistent_cache::PersistentCacheStore::for_support_dir(
            self.support_dir.clone(),
        )?;
        let mut metadata = cache
            .get_json::<AISessionMetadata>(AI_SESSION_METADATA_NAMESPACE, session_id)?
            .unwrap_or_else(|| AISessionMetadata {
                session_id: session_id.to_string(),
                retention: default_retention(),
                ..AISessionMetadata::default()
            });
        update(&mut metadata);
        metadata.session_id = session_id.to_string();
        metadata.retention = normalized_retention(&metadata.retention).to_string();
        metadata.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();
        cache.put_json(AI_SESSION_METADATA_NAMESPACE, session_id, &metadata)?;
        Ok(metadata)
    }
}

fn sanitize_metadata(mut metadata: AISessionMetadata) -> AISessionMetadata {
    metadata.retention = normalized_retention(&metadata.retention).to_string();
    metadata
}

fn normalized_retention(retention: &str) -> &'static str {
    if retention.trim() == "longTerm" {
        "longTerm"
    } else {
        "temporary"
    }
}

fn default_retention() -> String {
    "temporary".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_metadata_round_trips_independent_flags() {
        let support_dir = std::env::temp_dir().join(format!(
            "wecode-ai-session-meta-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = AISessionMetadataService::new(support_dir.clone());
        service.set_pinned("session-1", true).unwrap();
        service.set_retention("session-1", "longTerm").unwrap();
        service.set_archived("session-1", true).unwrap();

        let metadata = service.list().remove("session-1").unwrap();
        assert!(metadata.pinned);
        assert_eq!(metadata.retention, "longTerm");
        assert!(metadata.archived);
        let _ = std::fs::remove_dir_all(support_dir);
    }
}
