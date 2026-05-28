use super::{
    ProjectStore, TerminalLayoutRecord, TerminalLayoutsSnapshot, helpers::is_known_workspace_id,
    terminal_layout::sanitize_terminal_layout,
};
use serde_json::Value;

impl ProjectStore {
    pub fn terminal_layout(&self, project_id: &str) -> Option<TerminalLayoutRecord> {
        self.snapshot().terminal_layouts.get(project_id).cloned()
    }

    pub fn terminal_layouts_snapshot(&self) -> TerminalLayoutsSnapshot {
        TerminalLayoutsSnapshot {
            layouts: self.snapshot().terminal_layouts,
        }
    }

    pub fn save_terminal_layout(
        &self,
        project_id: String,
        layout: TerminalLayoutRecord,
    ) -> Result<TerminalLayoutRecord, String> {
        let snapshot = self.snapshot();
        if !is_known_workspace_id(&snapshot, &project_id) {
            return Err("Project workspace not found.".to_string());
        }
        let layout = sanitize_terminal_layout(layout)
            .ok_or_else(|| "Terminal layout is empty.".to_string())?;
        let mut raw = self.raw_snapshot();
        if !matches!(raw.get("terminalLayouts"), Some(Value::Object(_))) {
            raw.insert(
                "terminalLayouts".to_string(),
                Value::Object(Default::default()),
            );
        }
        raw.get_mut("terminalLayouts")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "terminalLayouts is not an object.".to_string())?
            .insert(
                project_id,
                serde_json::to_value(&layout).map_err(|error| error.to_string())?,
            );
        self.save_raw_snapshot(&raw)?;
        Ok(layout)
    }
}
