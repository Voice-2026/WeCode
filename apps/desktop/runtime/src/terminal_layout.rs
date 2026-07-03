use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const TERMINAL_LAYOUT_NAMESPACE: &str = "terminal-layout";
const DEFAULT_BOTTOM_RATIO: f64 = 0.24;

pub fn terminal_layout_storage_key(project_id: &str, worktree_id: &str) -> String {
    codux_terminal_core::runtime_scope_key(project_id, Some(worktree_id))
}

/// Max columns a desktop user can stack in the main terminal grid.
pub const TERMINAL_GRID_MAX_COLUMNS: usize = 6;
/// Max rows a desktop user can stack inside one terminal grid column.
pub const TERMINAL_GRID_MAX_ROWS: usize = 6;
/// Max split panes in the main terminal grid.
pub const TERMINAL_SPLIT_CAP: usize = TERMINAL_GRID_MAX_COLUMNS * TERMINAL_GRID_MAX_ROWS;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutSummary {
    #[serde(default, skip_serializing)]
    pub active_terminal_id: String,
    pub top_panes: Vec<TerminalPaneSummary>,
    pub tabs: Vec<TerminalTabSummary>,
    /// Legacy column fallback for old layouts; `top_grid` is authoritative for current layouts.
    #[serde(default)]
    pub top_ratios: Vec<f64>,
    #[serde(default)]
    pub top_grid: TerminalTopGrid,
    #[serde(default = "default_bottom_ratio")]
    pub bottom_ratio: f64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTopGrid {
    #[serde(default)]
    pub columns: Vec<TerminalGridColumn>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalGridColumn {
    pub ratio: f64,
    pub rows: usize,
    #[serde(default)]
    pub row_ratios: Vec<f64>,
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
            return sanitize_terminal_layout(layout).unwrap_or_default();
        }
        TerminalLayoutSummary {
            bottom_ratio: default_bottom_ratio(),
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
        let cache = crate::persistent_cache::PersistentCacheStore::for_support_dir(
            self.support_dir.clone(),
        )
        .ok();
        project_ids
            .into_iter()
            .filter_map(|project_id| {
                let layout = cache.as_ref().and_then(|cache| {
                    cache
                        .get_json::<TerminalLayoutSummary>(TERMINAL_LAYOUT_NAMESPACE, project_id)
                        .ok()
                        .flatten()
                })?;
                sanitize_terminal_layout(layout).map(|layout| (project_id.to_string(), layout))
            })
            .collect()
    }

    pub fn save_from_gpui(
        &self,
        project_id: &str,
        tabs: Vec<TerminalTabSummary>,
        top_panes: Vec<TerminalPaneSummary>,
        top_ratios: Vec<f64>,
        bottom_ratio: f64,
    ) -> Result<TerminalLayoutSummary, String> {
        self.save_from_gpui_with_grid(
            project_id,
            tabs,
            top_panes,
            top_ratios,
            TerminalTopGrid::default(),
            bottom_ratio,
        )
    }

    pub fn save_from_gpui_with_grid(
        &self,
        project_id: &str,
        tabs: Vec<TerminalTabSummary>,
        top_panes: Vec<TerminalPaneSummary>,
        top_ratios: Vec<f64>,
        top_grid: TerminalTopGrid,
        bottom_ratio: f64,
    ) -> Result<TerminalLayoutSummary, String> {
        if tabs.is_empty() && top_panes.is_empty() {
            return Err("Terminal layout is empty.".to_string());
        }
        let layout = TerminalLayoutSummary {
            tabs,
            active_terminal_id: String::new(),
            top_panes,
            top_ratios,
            top_grid,
            bottom_ratio,
            error: None,
        };
        self.save_summary(project_id, layout)
    }

    pub fn save_summary(
        &self,
        project_id: &str,
        layout: TerminalLayoutSummary,
    ) -> Result<TerminalLayoutSummary, String> {
        let layout = sanitize_terminal_layout(layout)
            .ok_or_else(|| "Terminal layout is empty.".to_string())?;
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
    DEFAULT_BOTTOM_RATIO
}

fn sanitize_terminal_layout(mut layout: TerminalLayoutSummary) -> Option<TerminalLayoutSummary> {
    if layout.tabs.is_empty() && layout.top_panes.is_empty() {
        return None;
    }
    layout.top_ratios = normalize_ratios(layout.top_ratios, layout.top_panes.len());
    layout.top_grid =
        normalize_top_grid(layout.top_grid, &layout.top_ratios, layout.top_panes.len());
    layout.bottom_ratio = clamp_ratio(layout.bottom_ratio, 0.16, 0.58, default_bottom_ratio());
    Some(layout)
}

pub fn normalize_top_grid(
    grid: TerminalTopGrid,
    top_ratios: &[f64],
    pane_count: usize,
) -> TerminalTopGrid {
    if pane_count == 0 {
        return TerminalTopGrid::default();
    }
    if grid.columns.is_empty() {
        return single_row_top_grid(top_ratios.to_vec(), pane_count);
    }
    if grid.columns.iter().any(|column| column.rows == 0) {
        return single_row_top_grid(top_ratios.to_vec(), pane_count);
    }
    let total_rows = grid.columns.iter().map(|column| column.rows).sum::<usize>();
    if total_rows != pane_count {
        return single_row_top_grid(top_ratios.to_vec(), pane_count);
    }
    let column_count = grid.columns.len();
    let column_ratios = normalize_ratios(
        grid.columns
            .iter()
            .map(|column| column.ratio)
            .collect::<Vec<_>>(),
        column_count,
    );
    let columns = grid
        .columns
        .into_iter()
        .zip(column_ratios)
        .map(|(column, ratio)| {
            let rows = column.rows.max(1);
            TerminalGridColumn {
                ratio,
                rows,
                row_ratios: normalize_ratios(column.row_ratios, rows),
            }
        })
        .collect::<Vec<_>>();
    TerminalTopGrid { columns }
}

pub fn single_row_top_grid(ratios: Vec<f64>, pane_count: usize) -> TerminalTopGrid {
    if pane_count == 0 {
        return TerminalTopGrid::default();
    }
    let ratios = normalize_ratios(ratios, pane_count);
    TerminalTopGrid {
        columns: ratios
            .into_iter()
            .map(|ratio| TerminalGridColumn {
                ratio,
                rows: 1,
                row_ratios: vec![1.0],
            })
            .collect(),
    }
}

fn normalize_ratios(ratios: Vec<f64>, count: usize) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    let mut values = ratios
        .into_iter()
        .take(count)
        .map(|value| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    while values.len() < count {
        values.push(1.0 / count as f64);
    }
    let total = values.iter().sum::<f64>();
    if total <= 0.0 {
        return vec![1.0 / count as f64; count];
    }
    values.into_iter().map(|value| value / total).collect()
}

fn clamp_ratio(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_layout_serialization_keeps_resizable_dimensions() {
        let layout = TerminalLayoutSummary {
            active_terminal_id: "terminal-1".to_string(),
            top_panes: vec![TerminalPaneSummary {
                title: "Split".to_string(),
                terminal_id: "terminal-1".to_string(),
            }],
            tabs: Vec::new(),
            top_ratios: vec![1.0],
            top_grid: TerminalTopGrid::default(),
            bottom_ratio: 0.72,
            error: None,
        };

        let value = serde_json::to_value(&layout).expect("serialize layout");
        assert!(value.get("activeTerminalId").is_none());
        assert_eq!(value["topRatios"][0].as_f64(), Some(1.0));
        assert_eq!(value["bottomRatio"].as_f64(), Some(0.72));
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
                vec![TerminalPaneSummary {
                    title: "Shell".to_string(),
                    terminal_id: "terminal-kept".to_string(),
                }],
                vec![1.0],
                0.24,
            )
            .expect("save initial layout");

        let error = service
            .save_from_gpui(
                "project-1::worktree-1",
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0.24,
            )
            .expect_err("empty layout should be rejected");
        assert_eq!(error, "Terminal layout is empty.");

        let layout = service.load(Some("project-1::worktree-1"));
        assert_eq!(layout.active_terminal_id, "");
        assert_eq!(layout.top_panes.len(), 1);
        assert_eq!(layout.top_panes[0].terminal_id, "terminal-kept");

        let _ = std::fs::remove_dir_all(support_dir);
    }

    #[test]
    fn legacy_top_ratios_migrate_to_single_row_grid() {
        let layout = sanitize_terminal_layout(TerminalLayoutSummary {
            top_panes: vec![
                TerminalPaneSummary {
                    title: "One".to_string(),
                    terminal_id: "terminal-1".to_string(),
                },
                TerminalPaneSummary {
                    title: "Two".to_string(),
                    terminal_id: "terminal-2".to_string(),
                },
            ],
            top_ratios: vec![1.0, 2.0],
            tabs: Vec::new(),
            active_terminal_id: String::new(),
            top_grid: TerminalTopGrid::default(),
            bottom_ratio: 0.24,
            error: None,
        })
        .expect("layout should sanitize");

        assert_eq!(layout.top_ratios, vec![1.0 / 3.0, 2.0 / 3.0]);
        assert_eq!(layout.top_grid.columns.len(), 2);
        assert_eq!(layout.top_grid.columns[0].rows, 1);
        assert_eq!(layout.top_grid.columns[1].ratio, 2.0 / 3.0);
    }

    #[test]
    fn invalid_grid_rows_rebuild_from_top_ratios() {
        let layout = sanitize_terminal_layout(TerminalLayoutSummary {
            top_panes: vec![
                TerminalPaneSummary {
                    title: "One".to_string(),
                    terminal_id: "terminal-1".to_string(),
                },
                TerminalPaneSummary {
                    title: "Two".to_string(),
                    terminal_id: "terminal-2".to_string(),
                },
            ],
            top_ratios: vec![0.25, 0.75],
            top_grid: TerminalTopGrid {
                columns: vec![TerminalGridColumn {
                    ratio: 1.0,
                    rows: 3,
                    row_ratios: vec![1.0, 1.0, 1.0],
                }],
            },
            tabs: Vec::new(),
            active_terminal_id: String::new(),
            bottom_ratio: 0.24,
            error: None,
        })
        .expect("layout should sanitize");

        assert_eq!(layout.top_grid.columns.len(), 2);
        assert_eq!(layout.top_grid.columns[0].ratio, 0.25);
        assert_eq!(layout.top_grid.columns[1].ratio, 0.75);
    }

    #[test]
    fn zero_row_grid_column_rebuilds_from_top_ratios() {
        let layout = sanitize_terminal_layout(TerminalLayoutSummary {
            top_panes: vec![
                TerminalPaneSummary {
                    title: "One".to_string(),
                    terminal_id: "terminal-1".to_string(),
                },
                TerminalPaneSummary {
                    title: "Two".to_string(),
                    terminal_id: "terminal-2".to_string(),
                },
            ],
            top_ratios: vec![0.2, 0.8],
            top_grid: TerminalTopGrid {
                columns: vec![
                    TerminalGridColumn {
                        ratio: 0.25,
                        rows: 0,
                        row_ratios: Vec::new(),
                    },
                    TerminalGridColumn {
                        ratio: 0.75,
                        rows: 2,
                        row_ratios: vec![0.5, 0.5],
                    },
                ],
            },
            tabs: Vec::new(),
            active_terminal_id: String::new(),
            bottom_ratio: 0.24,
            error: None,
        })
        .expect("layout should sanitize");

        assert_eq!(layout.top_grid.columns.len(), 2);
        assert_eq!(layout.top_grid.columns[0].rows, 1);
        assert_eq!(layout.top_grid.columns[0].ratio, 0.2);
        assert_eq!(layout.top_grid.columns[1].ratio, 0.8);
    }

    #[test]
    fn grid_roundtrip_serializes_columns() {
        let layout = TerminalLayoutSummary {
            top_panes: vec![
                TerminalPaneSummary {
                    title: "One".to_string(),
                    terminal_id: "terminal-1".to_string(),
                },
                TerminalPaneSummary {
                    title: "Two".to_string(),
                    terminal_id: "terminal-2".to_string(),
                },
            ],
            top_ratios: vec![0.5, 0.5],
            top_grid: TerminalTopGrid {
                columns: vec![TerminalGridColumn {
                    ratio: 1.0,
                    rows: 2,
                    row_ratios: vec![0.4, 0.6],
                }],
            },
            tabs: Vec::new(),
            active_terminal_id: String::new(),
            bottom_ratio: 0.24,
            error: None,
        };

        let json = serde_json::to_string(&layout).expect("serialize layout");
        let restored: TerminalLayoutSummary =
            serde_json::from_str(&json).expect("deserialize layout");
        assert_eq!(restored.top_grid.columns.len(), 1);
        assert_eq!(restored.top_grid.columns[0].rows, 2);
        assert_eq!(restored.top_grid.columns[0].row_ratios, vec![0.4, 0.6]);
    }
}
