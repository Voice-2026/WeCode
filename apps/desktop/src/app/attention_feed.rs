use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

const ATTENTION_FEED_CAPACITY: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::app) enum AttentionSemantic {
    Actionable,
    Completed,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum AttentionReadState {
    Unread,
    Read,
    Resolved,
}

#[derive(Clone, Debug)]
pub(in crate::app) struct AttentionItem {
    pub(in crate::app) id: String,
    pub(in crate::app) semantic: AttentionSemantic,
    pub(in crate::app) read_state: AttentionReadState,
    pub(in crate::app) project_id: String,
    pub(in crate::app) project_name: String,
    pub(in crate::app) project_path: Option<String>,
    pub(in crate::app) terminal_id: String,
    pub(in crate::app) session_title: String,
    pub(in crate::app) tool: String,
    pub(in crate::app) interrupted: bool,
    pub(in crate::app) updated_at: f64,
}

#[derive(Default)]
pub(in crate::app) struct AttentionFeed {
    items: VecDeque<AttentionItem>,
    seen_ids: HashSet<String>,
    terminal_semantics: HashMap<String, AttentionSemantic>,
    revision: u64,
}

impl AttentionFeed {
    pub(in crate::app) fn revision(&self) -> u64 {
        self.revision
    }

    pub(in crate::app) fn unread_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.read_state == AttentionReadState::Unread)
            .count()
    }

    pub(in crate::app) fn recent_items(&self, limit: usize) -> Vec<AttentionItem> {
        let mut items = self.items.iter().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            attention_priority(left)
                .cmp(&attention_priority(right))
                .then_with(|| right.updated_at.total_cmp(&left.updated_at))
        });
        items.truncate(limit);
        items
    }

    pub(in crate::app) fn ingest(
        &mut self,
        events: &[wecode_runtime::ai_runtime::AIRuntimeSupervisorEvent],
        sessions: &[wecode_runtime::ai_runtime_state::AIRuntimeSessionSummary],
    ) -> bool {
        let previous_revision = self.revision;

        for event in events {
            let wecode_runtime::ai_runtime::AIRuntimeSupervisorEvent::Completion { completion } =
                event
            else {
                continue;
            };
            let Some(session) = completion.session.as_ref() else {
                continue;
            };
            let semantic = if completion.was_interrupted {
                AttentionSemantic::Actionable
            } else {
                AttentionSemantic::Completed
            };
            self.terminal_semantics
                .insert(session.terminal_id.clone(), semantic);
            self.push(AttentionItem {
                id: format!("completion:{}", completion.id),
                semantic,
                read_state: AttentionReadState::Unread,
                project_id: session.project_id.clone(),
                project_name: completion.project_name.clone(),
                project_path: session.project_path.clone(),
                terminal_id: session.terminal_id.clone(),
                session_title: session.session_title.clone(),
                tool: completion.tool.clone(),
                interrupted: completion.was_interrupted,
                updated_at: session.updated_at,
            });
        }

        for session in sessions {
            let semantic = attention_semantic_for_session(session);
            if self.terminal_semantics.get(&session.terminal_id) == Some(&semantic) {
                continue;
            }
            if semantic == AttentionSemantic::Active {
                self.resolve_terminal(&session.terminal_id);
            }
            self.terminal_semantics
                .insert(session.terminal_id.clone(), semantic);
            self.push(AttentionItem {
                id: format!(
                    "session:{}:{semantic:?}:{:.6}",
                    session.terminal_id, session.updated_at
                ),
                semantic,
                read_state: if semantic == AttentionSemantic::Active {
                    AttentionReadState::Read
                } else {
                    AttentionReadState::Unread
                },
                project_id: session.project_id.clone(),
                project_name: session.project_name.clone(),
                project_path: session.project_path.clone(),
                terminal_id: session.terminal_id.clone(),
                session_title: session.session_title.clone(),
                tool: session.tool.clone(),
                interrupted: session.was_interrupted,
                updated_at: session.updated_at,
            });
        }

        self.revision != previous_revision
    }

    pub(in crate::app) fn mark_read(&mut self, id: &str) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.read_state != AttentionReadState::Unread {
            return false;
        }
        item.read_state = AttentionReadState::Read;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub(in crate::app) fn mark_all_read(&mut self) -> bool {
        let mut changed = false;
        for item in &mut self.items {
            if item.read_state == AttentionReadState::Unread {
                item.read_state = AttentionReadState::Read;
                changed = true;
            }
        }
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }

    #[allow(dead_code)]
    pub(in crate::app) fn resolve(&mut self, id: &str) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.read_state == AttentionReadState::Resolved {
            return false;
        }
        item.read_state = AttentionReadState::Resolved;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    fn push(&mut self, item: AttentionItem) {
        if !self.seen_ids.insert(item.id.clone()) {
            return;
        }
        self.items.push_front(item);
        while self.items.len() > ATTENTION_FEED_CAPACITY {
            if let Some(removed) = self.items.pop_back() {
                self.seen_ids.remove(&removed.id);
            }
        }
        self.revision = self.revision.wrapping_add(1);
    }

    fn resolve_terminal(&mut self, terminal_id: &str) {
        let mut changed = false;
        for item in &mut self.items {
            if item.terminal_id == terminal_id
                && item.semantic != AttentionSemantic::Active
                && item.read_state != AttentionReadState::Resolved
            {
                item.read_state = AttentionReadState::Resolved;
                changed = true;
            }
        }
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
    }
}

impl WeCodeApp {
    pub(in crate::app) fn open_attention_item(
        &mut self,
        item_id: String,
        project_id: String,
        project_path: Option<String>,
        terminal_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.attention_feed.mark_read(&item_id);
        if !project_id.is_empty()
            && self
                .state
                .selected_project
                .as_ref()
                .is_none_or(|project| project.id != project_id)
            && self
                .state
                .projects
                .iter()
                .any(|project| project.id == project_id)
        {
            self.select_project(project_id, window, cx);
        }
        if let Some(project_path) = project_path.as_deref() {
            let target_worktree_id = self
                .state
                .worktrees
                .worktrees
                .iter()
                .find(|worktree| worktree.path == project_path)
                .map(|worktree| worktree.id.clone());
            if let Some(worktree_id) = target_worktree_id
                && self.state.worktrees.selected_worktree_id.as_deref()
                    != Some(worktree_id.as_str())
            {
                self.select_worktree(worktree_id, window, cx);
            }
        }
        self.workspace_view = WorkspaceView::Terminal;
        self.workspace_split = None;
        if let Some(scope_key) = current_worktree_scope_key(&self.state) {
            self.active_terminal_runtime_ids
                .insert(scope_key, terminal_id.clone());
        }
        self.set_active_terminal_runtime_id(Some(&terminal_id));
        self.status_message =
            self.text("workspace.attention.opened", "Opened the related terminal");
        self.invalidate_ui(
            cx,
            [
                UiRegion::WorkspaceChrome,
                UiRegion::WorkspaceBody,
                UiRegion::StatusBar,
            ],
        );
        let _ = self.focus_active_terminal_view(window, cx);
    }

    pub(in crate::app) fn mark_all_attention_read(&mut self, cx: &mut Context<Self>) {
        if self.attention_feed.mark_all_read() {
            self.invalidate_ui(cx, [UiRegion::WorkspaceChrome, UiRegion::WorkspaceBody]);
        }
    }
}

pub(in crate::app) fn attention_relative_time(updated_at: f64, now: f64, language: &str) -> String {
    let elapsed = (now - updated_at).max(0.0) as u64;
    let is_zh = language.starts_with("zh");
    if elapsed < 60 {
        if is_zh {
            "刚刚".to_string()
        } else {
            "now".to_string()
        }
    } else if elapsed < 3_600 {
        let minutes = elapsed / 60;
        if is_zh {
            format!("{minutes} 分钟前")
        } else {
            format!("{minutes}m ago")
        }
    } else if elapsed < 86_400 {
        let hours = elapsed / 3_600;
        if is_zh {
            format!("{hours} 小时前")
        } else {
            format!("{hours}h ago")
        }
    } else {
        let days = elapsed / 86_400;
        if is_zh {
            format!("{days} 天前")
        } else {
            format!("{days}d ago")
        }
    }
}

fn attention_semantic_for_session(
    session: &wecode_runtime::ai_runtime_state::AIRuntimeSessionSummary,
) -> AttentionSemantic {
    if session.was_interrupted || session.state == "needs-input" {
        AttentionSemantic::Actionable
    } else if session.state == "completed" || session.has_completed_turn {
        AttentionSemantic::Completed
    } else {
        AttentionSemantic::Active
    }
}

fn attention_priority(item: &AttentionItem) -> u8 {
    match (item.read_state, item.semantic) {
        (AttentionReadState::Unread, AttentionSemantic::Actionable) => 0,
        (AttentionReadState::Unread, AttentionSemantic::Completed) => 1,
        (_, AttentionSemantic::Actionable) => 2,
        (_, AttentionSemantic::Completed) => 3,
        (_, AttentionSemantic::Active) => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(
        state: &str,
        updated_at: f64,
    ) -> wecode_runtime::ai_runtime_state::AIRuntimeSessionSummary {
        wecode_runtime::ai_runtime_state::AIRuntimeSessionSummary {
            terminal_id: "terminal-1".to_string(),
            project_id: "project-1".to_string(),
            project_path: None,
            tool: "Claude".to_string(),
            ai_session_id: None,
            model: None,
            state: state.to_string(),
            project_name: "Demo".to_string(),
            session_title: "Implement feature".to_string(),
            started_at: None,
            updated_at,
            event_count: 1,
            has_completed_turn: state == "completed",
            was_interrupted: false,
            notification_type: None,
            target_tool_name: None,
            message: None,
            latest_assistant_preview: None,
            plan: None,
            total_tokens: 0,
            cached_input_tokens: 0,
            raw_total_tokens: 0,
            raw_cached_input_tokens: 0,
            baseline_total_tokens: 0,
            baseline_cached_input_tokens: 0,
            usage_amounts: Vec::new(),
            raw_usage_amounts: Vec::new(),
            baseline_usage_amounts: Vec::new(),
            source: "test".to_string(),
        }
    }

    #[test]
    fn adds_only_state_transitions_and_counts_actionable_as_unread() {
        let mut feed = AttentionFeed::default();
        assert!(feed.ingest(&[], &[session("running", 1.0)]));
        assert_eq!(feed.unread_count(), 0);
        assert!(!feed.ingest(&[], &[session("running", 2.0)]));
        assert!(feed.ingest(&[], &[session("needs-input", 3.0)]));
        assert_eq!(feed.unread_count(), 1);
        assert_eq!(
            feed.recent_items(1)[0].semantic,
            AttentionSemantic::Actionable
        );
        assert!(feed.ingest(&[], &[session("running", 4.0)]));
        assert_eq!(feed.unread_count(), 0);
        assert_eq!(
            feed.recent_items(3)[0].read_state,
            AttentionReadState::Resolved
        );
    }

    #[test]
    fn mark_all_read_updates_unread_count() {
        let mut feed = AttentionFeed::default();
        assert!(feed.ingest(&[], &[session("completed", 1.0)]));
        assert_eq!(feed.unread_count(), 1);
        assert!(feed.mark_all_read());
        assert_eq!(feed.unread_count(), 0);
        assert!(!feed.mark_all_read());
    }

    #[test]
    fn relative_time_uses_compact_localized_units() {
        assert_eq!(attention_relative_time(100.0, 120.0, "zh-Hans"), "刚刚");
        assert_eq!(attention_relative_time(100.0, 220.0, "zh-Hans"), "2 分钟前");
        assert_eq!(attention_relative_time(100.0, 7_300.0, "en"), "2h ago");
    }
}
