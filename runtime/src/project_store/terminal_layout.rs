use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutRecord {
    #[serde(default)]
    pub tabs: Vec<TerminalBottomTabRecord>,
    #[serde(default)]
    pub active_tab_id: String,
    #[serde(default)]
    pub top_panes: Vec<TerminalTopPaneRecord>,
    #[serde(default)]
    pub top_ratios: Vec<f64>,
    #[serde(default = "default_bottom_ratio")]
    pub bottom_ratio: f64,
    #[serde(default)]
    pub active_slot_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBottomTabRecord {
    pub id: String,
    pub label: String,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTopPaneRecord {
    pub id: String,
    pub title: String,
    pub terminal_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub detached: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLayoutsSnapshot {
    pub layouts: HashMap<String, TerminalLayoutRecord>,
}

pub(super) fn sanitize_terminal_layout(
    layout: TerminalLayoutRecord,
) -> Option<TerminalLayoutRecord> {
    let tabs = sanitize_bottom_tabs(layout.tabs);
    let (top_panes, top_ratios) =
        sanitize_top_pane_ratio_entries(layout.top_panes, layout.top_ratios);
    if tabs.is_empty() && top_panes.is_empty() {
        return None;
    }
    let active_tab_id = if tabs.iter().any(|tab| tab.id == layout.active_tab_id) {
        layout.active_tab_id
    } else {
        tabs.first().map(|tab| tab.id.clone()).unwrap_or_default()
    };
    let active_slot_id = if top_panes
        .iter()
        .any(|pane| pane.id == layout.active_slot_id)
        || tabs.iter().any(|tab| tab.id == layout.active_slot_id)
    {
        layout.active_slot_id
    } else {
        top_panes
            .first()
            .map(|pane| pane.id.clone())
            .or_else(|| tabs.first().map(|tab| tab.id.clone()))
            .unwrap_or_default()
    };

    Some(TerminalLayoutRecord {
        tabs,
        active_tab_id,
        top_panes,
        top_ratios,
        bottom_ratio: clamp_ratio(layout.bottom_ratio, 0.18, 0.72, default_bottom_ratio()),
        active_slot_id,
    })
}

fn sanitize_bottom_tabs(tabs: Vec<TerminalBottomTabRecord>) -> Vec<TerminalBottomTabRecord> {
    let mut seen = HashSet::new();
    let mut next = tabs
        .into_iter()
        .filter_map(|tab| {
            let id = normalized_string(&tab.id)?;
            if !seen.insert(id.clone()) {
                return None;
            }
            Some(TerminalBottomTabRecord {
                id,
                label: normalized_string(&tab.label).unwrap_or_else(|| "Tab".to_string()),
                terminal_id: normalized_string(&tab.terminal_id).unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    next.sort_by(|left, right| compare_slot_id(&left.id, &right.id));
    next
}

fn sanitize_top_pane_ratio_entries(
    panes: Vec<TerminalTopPaneRecord>,
    ratios: Vec<f64>,
) -> (Vec<TerminalTopPaneRecord>, Vec<f64>) {
    let mut seen = HashSet::new();
    let mut next = panes
        .into_iter()
        .enumerate()
        .filter_map(|(index, pane)| {
            let id = normalized_string(&pane.id)?;
            if !seen.insert(id.clone()) {
                return None;
            }
            Some((
                TerminalTopPaneRecord {
                    id,
                    title: normalized_string(&pane.title).unwrap_or_else(|| "Split".to_string()),
                    terminal_id: normalized_string(&pane.terminal_id).unwrap_or_default(),
                    detached: false,
                },
                ratios.get(index).copied().unwrap_or(0.0),
            ))
        })
        .collect::<Vec<_>>();
    next.sort_by(|left, right| compare_slot_id(&left.0.id, &right.0.id));
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

fn compare_slot_id(left: &str, right: &str) -> std::cmp::Ordering {
    let (left_prefix, left_index) = parse_slot_id(left);
    let (right_prefix, right_index) = parse_slot_id(right);
    left_prefix
        .cmp(&right_prefix)
        .then_with(|| left_index.cmp(&right_index))
}

fn parse_slot_id(id: &str) -> (String, usize) {
    let Some((prefix, index)) = id.rsplit_once('-') else {
        return (id.to_string(), usize::MAX);
    };
    let index = index.parse::<usize>().unwrap_or(usize::MAX);
    (prefix.to_string(), index)
}

fn default_bottom_ratio() -> f64 {
    0.32
}

fn is_false(value: &bool) -> bool {
    !*value
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

    #[test]
    fn sanitize_terminal_layout_drops_empty_records_and_normalizes_order() {
        let layout = sanitize_terminal_layout(TerminalLayoutRecord {
            tabs: vec![
                TerminalBottomTabRecord {
                    id: "bottom-2".to_string(),
                    label: "  Second  ".to_string(),
                    terminal_id: "term-2".to_string(),
                },
                TerminalBottomTabRecord {
                    id: "bottom-1".to_string(),
                    label: String::new(),
                    terminal_id: "term-1".to_string(),
                },
                TerminalBottomTabRecord {
                    id: "bottom-1".to_string(),
                    label: "Duplicate".to_string(),
                    terminal_id: "term-x".to_string(),
                },
            ],
            active_tab_id: "missing".to_string(),
            top_panes: vec![TerminalTopPaneRecord {
                id: "top-1".to_string(),
                title: String::new(),
                terminal_id: "term-3".to_string(),
                detached: true,
            }],
            top_ratios: vec![0.0],
            bottom_ratio: 0.99,
            active_slot_id: "missing".to_string(),
        })
        .unwrap();

        assert_eq!(layout.tabs.len(), 2);
        assert_eq!(layout.tabs[0].id, "bottom-1");
        assert_eq!(layout.tabs[0].label, "Tab");
        assert_eq!(layout.active_tab_id, "bottom-1");
        assert_eq!(layout.top_panes[0].title, "Split");
        assert!(!layout.top_panes[0].detached);
        assert_eq!(layout.top_ratios, vec![1.0]);
        assert_eq!(layout.bottom_ratio, 0.72);
        assert_eq!(layout.active_slot_id, "top-1");
    }

    #[test]
    fn sanitize_terminal_layout_rejects_empty_layout() {
        assert!(
            sanitize_terminal_layout(TerminalLayoutRecord {
                tabs: Vec::new(),
                active_tab_id: String::new(),
                top_panes: Vec::new(),
                top_ratios: Vec::new(),
                bottom_ratio: 0.32,
                active_slot_id: String::new(),
            })
            .is_none()
        );
    }
}
