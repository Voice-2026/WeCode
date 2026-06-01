use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutSummary {
    pub active_slot_id: String,
    pub active_tab_id: String,
    pub top_panes: Vec<TerminalPaneSummary>,
    pub tabs: Vec<TerminalTabSummary>,
    pub top_ratios: Vec<f64>,
    pub bottom_ratio: f64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalPaneSummary {
    pub id: String,
    pub title: String,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTabSummary {
    pub id: String,
    pub label: String,
    pub terminal_id: String,
}

pub struct TerminalLayoutService {
    state_file: PathBuf,
}

impl TerminalLayoutService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            state_file: crate::config::state_file_path(support_dir),
        }
    }

    pub fn load(&self, project_id: Option<&str>) -> TerminalLayoutSummary {
        let Some(project_id) = project_id else {
            return TerminalLayoutSummary {
                error: Some("No selected project.".to_string()),
                ..Default::default()
            };
        };
        let store = crate::config::ConfigStore::for_file(self.state_file.clone());
        let Some(layout) = store.get_path(&["terminalLayouts", project_id]) else {
            return TerminalLayoutSummary {
                bottom_ratio: 0.32,
                error: Some("No terminal layout saved for selected project.".to_string()),
                ..Default::default()
            };
        };
        serde_json::from_value::<TerminalLayoutSummary>(layout.clone()).unwrap_or_else(|error| {
            TerminalLayoutSummary {
                bottom_ratio: 0.32,
                error: Some(error.to_string()),
                ..Default::default()
            }
        })
    }

    pub fn save_from_gpui(
        &self,
        project_id: &str,
        tabs: Vec<TerminalTabSummary>,
        active_tab_id: String,
        top_panes: Vec<TerminalPaneSummary>,
        active_slot_id: String,
    ) -> Result<TerminalLayoutSummary, String> {
        let top_ratios = if top_panes.is_empty() {
            Vec::new()
        } else {
            vec![1.0 / top_panes.len() as f64; top_panes.len()]
        };
        crate::config::ConfigStore::for_file(self.state_file.clone()).set_path(&[
            "terminalLayouts",
            project_id,
        ], json!({
            "tabs": tabs,
            "activeTabId": active_tab_id,
            "topPanes": top_panes,
            "topRatios": top_ratios,
            "bottomRatio": 0.32,
            "activeSlotId": active_slot_id,
        }))?;
        Ok(self.load(Some(project_id)))
    }
}
