use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEditorLayoutSummary {
    #[serde(default)]
    pub tabs: Vec<FileEditorTabSummary>,
    #[serde(default)]
    pub active_path: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEditorTabSummary {
    pub path: String,
    pub label: String,
    #[serde(default)]
    pub language: String,
}

pub struct FileEditorLayoutService {
    state_file: PathBuf,
}

impl FileEditorLayoutService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            state_file: crate::config::state_file_path(support_dir),
        }
    }

    pub fn load(&self, owner_id: Option<&str>) -> FileEditorLayoutSummary {
        let Some(owner_id) = owner_id else {
            return FileEditorLayoutSummary {
                error: Some("No selected project workspace.".to_string()),
                ..Default::default()
            };
        };
        let store = crate::config::ConfigStore::for_file(self.state_file.clone());
        let Some(layout) = store.get_path(&["fileEditorLayouts", owner_id]) else {
            return FileEditorLayoutSummary::default();
        };
        serde_json::from_value::<FileEditorLayoutSummary>(layout.clone()).unwrap_or_else(|error| {
            FileEditorLayoutSummary {
                error: Some(error.to_string()),
                ..Default::default()
            }
        })
    }

    pub fn save_from_gpui(
        &self,
        owner_id: &str,
        tabs: Vec<FileEditorTabSummary>,
        active_path: Option<String>,
    ) -> Result<FileEditorLayoutSummary, String> {
        let store = crate::config::ConfigStore::for_file(self.state_file.clone());
        if tabs.is_empty() {
            store.del_path(&["fileEditorLayouts", owner_id])?;
        } else {
            let active_path = active_path
                .filter(|active| tabs.iter().any(|tab| tab.path == *active))
                .or_else(|| tabs.first().map(|tab| tab.path.clone()));
            store.set_path(&["fileEditorLayouts", owner_id], json!({
                "tabs": tabs,
                "activePath": active_path,
            }))?;
        }
        Ok(self.load(Some(owner_id)))
    }
}
