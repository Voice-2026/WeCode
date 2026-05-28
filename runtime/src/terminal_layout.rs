use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{fs, path::PathBuf};

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
            state_file: support_dir.join("state.json"),
        }
    }

    pub fn load(&self, project_id: Option<&str>) -> TerminalLayoutSummary {
        let Some(project_id) = project_id else {
            return TerminalLayoutSummary {
                error: Some("No selected project.".to_string()),
                ..Default::default()
            };
        };
        let raw = self.raw_snapshot();
        let Some(layout) = raw
            .get("terminalLayouts")
            .and_then(Value::as_object)
            .and_then(|layouts| layouts.get(project_id))
        else {
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
        let mut raw = self.raw_snapshot();
        let layouts = raw
            .entry("terminalLayouts".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let layouts = layouts
            .as_object_mut()
            .ok_or_else(|| "terminalLayouts is not an object.".to_string())?;
        let top_ratios = if top_panes.is_empty() {
            Vec::new()
        } else {
            vec![1.0 / top_panes.len() as f64; top_panes.len()]
        };
        let layout = json!({
            "tabs": tabs,
            "activeTabId": active_tab_id,
            "topPanes": top_panes,
            "topRatios": top_ratios,
            "bottomRatio": 0.32,
            "activeSlotId": active_slot_id,
        });
        layouts.insert(project_id.to_string(), layout);
        self.save_raw_snapshot(&raw)?;
        Ok(self.load(Some(project_id)))
    }

    fn raw_snapshot(&self) -> Map<String, Value> {
        fs::read_to_string(&self.state_file)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    fn save_raw_snapshot(&self, snapshot: &Map<String, Value>) -> Result<(), String> {
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let content = serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?;
        fs::write(&self.state_file, format!("{content}\n")).map_err(|error| error.to_string())
    }
}
