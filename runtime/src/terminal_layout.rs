use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const TERMINAL_LAYOUT_NAMESPACE: &str = "terminal-layout";

pub fn terminal_layout_storage_key(project_id: &str, worktree_id: &str) -> String {
    format!("{project_id}::{worktree_id}")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutSummary {
    #[serde(default)]
    pub active_terminal_id: String,
    pub top_panes: Vec<TerminalPaneSummary>,
    pub tabs: Vec<TerminalTabSummary>,
    #[serde(default, skip_serializing)]
    pub top_ratios: Vec<f64>,
    #[serde(default = "default_bottom_ratio", skip_serializing)]
    pub bottom_ratio: f64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalPaneSummary {
    pub title: String,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTabSummary {
    pub label: String,
    pub terminal_id: String,
}

pub struct TerminalLayoutService {
    support_dir: PathBuf,
}

impl TerminalLayoutService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self { support_dir }
    }

    pub fn load(&self, project_id: Option<&str>) -> TerminalLayoutSummary {
        let Some(project_id) = project_id else {
            return TerminalLayoutSummary {
                error: Some("No selected project.".to_string()),
                ..Default::default()
            };
        };
        if let Some(layout) = self.cache_layout(project_id) {
            return layout;
        }
        TerminalLayoutSummary {
            bottom_ratio: 0.32,
            error: Some("No terminal layout saved for selected project.".to_string()),
            ..Default::default()
        }
    }

    fn cache_layout(&self, project_id: &str) -> Option<TerminalLayoutSummary> {
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())
            .ok()?
            .get_json::<TerminalLayoutSummary>(TERMINAL_LAYOUT_NAMESPACE, project_id)
            .ok()
            .flatten()
    }

    pub fn load_many<'a, I>(
        &self,
        project_ids: I,
    ) -> std::collections::HashMap<String, TerminalLayoutSummary>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let cache =
            crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())
                .ok();
        project_ids
            .into_iter()
            .filter_map(|project_id| {
                let layout = cache
                    .as_ref()
                    .and_then(|cache| {
                        cache
                            .get_json::<TerminalLayoutSummary>(
                                TERMINAL_LAYOUT_NAMESPACE,
                                project_id,
                            )
                            .ok()
                            .flatten()
                    })?;
                Some((project_id.to_string(), layout))
            })
            .collect()
    }

    pub fn save_from_gpui(
        &self,
        project_id: &str,
        tabs: Vec<TerminalTabSummary>,
        active_terminal_id: String,
        top_panes: Vec<TerminalPaneSummary>,
    ) -> Result<TerminalLayoutSummary, String> {
        if tabs.is_empty() && top_panes.is_empty() {
            return Err("Terminal layout is empty.".to_string());
        }
        let top_ratios = if top_panes.is_empty() {
            Vec::new()
        } else {
            vec![1.0 / top_panes.len() as f64; top_panes.len()]
        };
        let layout = TerminalLayoutSummary {
            tabs,
            active_terminal_id,
            top_panes,
            top_ratios,
            bottom_ratio: 0.32,
            error: None,
        };
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())?
            .put_json(TERMINAL_LAYOUT_NAMESPACE, project_id, &layout)?;
        Ok(layout)
    }

    pub fn delete(&self, project_id: &str) -> Result<bool, String> {
        crate::persistent_cache::PersistentCacheStore::for_support_dir(self.support_dir.clone())?
            .delete_json(TERMINAL_LAYOUT_NAMESPACE, project_id)
    }
}

pub(crate) fn terminal_layout_cache_namespace() -> &'static str {
    TERMINAL_LAYOUT_NAMESPACE
}

fn default_bottom_ratio() -> f64 {
    0.32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_layout_serialization_omits_resizable_dimensions() {
        let layout = TerminalLayoutSummary {
            active_terminal_id: "terminal-1".to_string(),
            top_panes: vec![TerminalPaneSummary {
                title: "Split".to_string(),
                terminal_id: "terminal-1".to_string(),
            }],
            tabs: Vec::new(),
            top_ratios: vec![1.0],
            bottom_ratio: 0.72,
            error: None,
        };

        let value = serde_json::to_value(&layout).expect("serialize layout");
        assert!(value.get("topRatios").is_none());
        assert!(value.get("bottomRatio").is_none());
    }

    #[test]
    fn save_from_gpui_rejects_empty_layout_without_overwriting_existing_layout() {
        let support_dir = std::env::temp_dir().join(format!(
            "codux-terminal-layout-empty-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&support_dir).expect("create support dir");
        let service = TerminalLayoutService::new(support_dir.clone());

        service
            .save_from_gpui(
                "project-1::worktree-1",
                Vec::new(),
                "terminal-kept".to_string(),
                vec![TerminalPaneSummary {
                    title: "Shell".to_string(),
                    terminal_id: "terminal-kept".to_string(),
                }],
            )
            .expect("save initial layout");

        let error = service
            .save_from_gpui(
                "project-1::worktree-1",
                Vec::new(),
                String::new(),
                Vec::new(),
            )
            .expect_err("empty layout should be rejected");
        assert_eq!(error, "Terminal layout is empty.");

        let layout = service.load(Some("project-1::worktree-1"));
        assert_eq!(layout.active_terminal_id, "terminal-kept");
        assert_eq!(layout.top_panes.len(), 1);
        assert_eq!(layout.top_panes[0].terminal_id, "terminal-kept");

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
