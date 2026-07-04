use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::terminal_layout::{
    TerminalSplitNode, TerminalTopGrid, normalize_split_tree, normalize_top_grid,
    top_grid_from_split_tree,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutRecord {
    #[serde(default)]
    pub tabs: Vec<TerminalBottomTabRecord>,
    #[serde(default, skip_serializing)]
    pub active_terminal_id: String,
    #[serde(default)]
    pub top_panes: Vec<TerminalTopPaneRecord>,
    #[serde(default, skip_serializing)]
    pub top_ratios: Vec<f64>,
    #[serde(default)]
    pub top_grid: TerminalTopGrid,
    #[serde(default)]
    pub split_tree: Option<TerminalSplitNode>,
    #[serde(default = "default_bottom_ratio", skip_serializing)]
    pub bottom_ratio: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBottomTabRecord {
    pub label: String,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTopPaneRecord {
    pub title: String,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutsSnapshot {
    pub layouts: HashMap<String, TerminalLayoutRecord>,
}

pub(super) fn sanitize_terminal_layout(
    layout: TerminalLayoutRecord,
) -> Option<TerminalLayoutRecord> {
    let top_panes = migrate_legacy_tabs_to_top_panes(layout.top_panes, layout.tabs);
    let (top_panes, top_ratios) = sanitize_top_pane_ratio_entries(top_panes, layout.top_ratios);
    let fallback_grid = normalize_top_grid(layout.top_grid, &top_ratios, top_panes.len());
    let split_tree = normalize_split_tree(
        layout.split_tree,
        &fallback_grid,
        &top_ratios,
        top_panes.len(),
    );
    let top_grid = split_tree
        .as_ref()
        .map(|tree| top_grid_from_split_tree(tree, top_panes.len()))
        .unwrap_or(fallback_grid);
    if top_panes.is_empty() {
        return None;
    }
    Some(TerminalLayoutRecord {
        tabs: Vec::new(),
        active_terminal_id: String::new(),
        top_panes,
        top_ratios: top_grid.columns.iter().map(|column| column.ratio).collect(),
        top_grid,
        split_tree,
        bottom_ratio: clamp_ratio(layout.bottom_ratio, 0.18, 0.72, default_bottom_ratio()),
    })
}

fn migrate_legacy_tabs_to_top_panes(
    mut panes: Vec<TerminalTopPaneRecord>,
    tabs: Vec<TerminalBottomTabRecord>,
) -> Vec<TerminalTopPaneRecord> {
    let mut seen = panes
        .iter()
        .map(|pane| pane.terminal_id.trim().to_string())
        .filter(|terminal_id| !terminal_id.is_empty())
        .collect::<HashSet<_>>();
    for tab in tabs {
        let Some(terminal_id) = normalized_string(&tab.terminal_id) else {
            continue;
        };
        if !seen.insert(terminal_id.clone()) {
            continue;
        }
        panes.push(TerminalTopPaneRecord {
            title: normalized_string(&tab.label).unwrap_or_else(|| "Terminal".to_string()),
            terminal_id,
        });
    }
    panes
}

fn sanitize_top_pane_ratio_entries(
    panes: Vec<TerminalTopPaneRecord>,
    ratios: Vec<f64>,
) -> (Vec<TerminalTopPaneRecord>, Vec<f64>) {
    let mut seen = HashSet::new();
    let next = panes
        .into_iter()
        .enumerate()
        .filter_map(|(index, pane)| {
            let terminal_id = normalized_string(&pane.terminal_id)?;
            if !seen.insert(terminal_id.clone()) {
                return None;
            }
            Some((
                TerminalTopPaneRecord {
                    title: normalized_string(&pane.title).unwrap_or_else(|| "Split".to_string()),
                    terminal_id,
                },
                ratios.get(index).copied().unwrap_or(0.0),
            ))
        })
        .collect::<Vec<_>>();
    let top_panes = next
        .iter()
        .map(|(pane, _)| pane.clone())
        .collect::<Vec<_>>();
    let top_ratios = normalize_ratios(
        next.into_iter().map(|(_, ratio)| ratio).collect(),
        top_panes.len(),
    );
    (top_panes, top_ratios)
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

fn default_bottom_ratio() -> f64 {
    0.32
}

fn normalized_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
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
    use crate::terminal_layout::{TerminalGridColumn, single_row_top_grid};

    #[test]
    fn sanitize_terminal_layout_migrates_legacy_bottom_tabs_to_top_panes() {
        let layout = sanitize_terminal_layout(TerminalLayoutRecord {
            tabs: vec![
                TerminalBottomTabRecord {
                    label: "  Second  ".to_string(),
                    terminal_id: "term-2".to_string(),
                },
                TerminalBottomTabRecord {
                    label: String::new(),
                    terminal_id: "term-1".to_string(),
                },
                TerminalBottomTabRecord {
                    label: "Duplicate".to_string(),
                    terminal_id: "term-1".to_string(),
                },
            ],
            active_terminal_id: "missing".to_string(),
            top_panes: vec![TerminalTopPaneRecord {
                title: String::new(),
                terminal_id: "term-3".to_string(),
            }],
            top_ratios: vec![0.0],
            top_grid: TerminalTopGrid::default(),
            split_tree: None,
            bottom_ratio: 0.99,
        })
        .unwrap();

        assert!(layout.tabs.is_empty());
        assert_eq!(layout.active_terminal_id, "");
        assert_eq!(
            layout
                .top_panes
                .iter()
                .map(|pane| (pane.title.as_str(), pane.terminal_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Split", "term-3"),
                ("Second", "term-2"),
                ("Terminal", "term-1")
            ]
        );
        assert_eq!(layout.top_ratios, vec![1.0 / 3.0; 3]);
        assert_eq!(
            layout.top_grid.columns,
            single_row_top_grid(vec![1.0 / 3.0; 3], 3).columns
        );
        assert_eq!(layout.bottom_ratio, 0.72);
    }

    #[test]
    fn sanitize_terminal_layout_rejects_empty_layout() {
        assert!(
            sanitize_terminal_layout(TerminalLayoutRecord {
                tabs: Vec::new(),
                active_terminal_id: String::new(),
                top_panes: Vec::new(),
                top_ratios: Vec::new(),
                top_grid: TerminalTopGrid::default(),
                split_tree: None,
                bottom_ratio: 0.32,
            })
            .is_none()
        );
    }

    #[test]
    fn terminal_layout_record_serialization_omits_runtime_ui_state() {
        let layout = TerminalLayoutRecord {
            tabs: Vec::new(),
            active_terminal_id: "terminal-1".to_string(),
            top_panes: vec![TerminalTopPaneRecord {
                title: "Split".to_string(),
                terminal_id: "terminal-1".to_string(),
            }],
            top_ratios: vec![1.0],
            top_grid: TerminalTopGrid::default(),
            split_tree: None,
            bottom_ratio: 0.72,
        };

        let value = serde_json::to_value(&layout).expect("serialize layout");
        assert!(value.get("activeTerminalId").is_none());
        assert!(value.get("topRatios").is_none());
        assert!(value.get("topGrid").is_some());
        assert!(value.get("bottomRatio").is_none());
    }

    #[test]
    fn sanitize_terminal_layout_preserves_valid_grid() {
        let layout = sanitize_terminal_layout(TerminalLayoutRecord {
            tabs: Vec::new(),
            active_terminal_id: String::new(),
            top_panes: vec![
                TerminalTopPaneRecord {
                    title: "One".to_string(),
                    terminal_id: "term-1".to_string(),
                },
                TerminalTopPaneRecord {
                    title: "Two".to_string(),
                    terminal_id: "term-2".to_string(),
                },
            ],
            top_ratios: vec![0.5, 0.5],
            top_grid: TerminalTopGrid {
                columns: vec![TerminalGridColumn {
                    ratio: 1.0,
                    rows: 2,
                    row_ratios: vec![1.0, 3.0],
                }],
            },
            split_tree: None,
            bottom_ratio: 0.32,
        })
        .unwrap();

        assert_eq!(layout.top_grid.columns.len(), 1);
        assert_eq!(layout.top_grid.columns[0].rows, 2);
        assert_eq!(layout.top_grid.columns[0].row_ratios, vec![0.25, 0.75]);
    }
}
