//! WeChat pairing actions: confirm/dismiss a chat peer's pairing request and
//! resolve the terminal session it binds to. Bridge lifecycle buttons live in
//! the settings card (`settings/panes/remote/wechat.rs`).

use codux_runtime::wechat_bridge_service::{
    wechat_bridge_bind_existing_to_session, wechat_bridge_confirm_pairing,
    wechat_bridge_dismiss_pairing, wechat_bridge_fallback_terminal_session_id,
};
use gpui::Context;

use super::{CoduxApp, UiRegion};

impl CoduxApp {
    /// Bind the pending WeChat peer to the active terminal session.
    pub(in crate::app) fn wechat_confirm_pairing(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_terminal_session_id() else {
            codux_runtime::runtime_trace::runtime_trace(
                "wechat",
                "confirm_pairing click skipped reason=no_active_terminal",
            );
            return;
        };
        let queued = wechat_bridge_confirm_pairing(&session_id);
        codux_runtime::runtime_trace::runtime_trace(
            "wechat",
            &format!("confirm_pairing click session={session_id} queued={queued}"),
        );
        self.invalidate_ui_region(cx, UiRegion::Root);
    }

    pub(in crate::app) fn wechat_dismiss_pairing(&mut self, cx: &mut Context<Self>) {
        wechat_bridge_dismiss_pairing();
        self.invalidate_ui_region(cx, UiRegion::Root);
    }

    pub(in crate::app) fn wechat_bind_terminal_session(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let queued = wechat_bridge_bind_existing_to_session(session_id);
        codux_runtime::runtime_trace::runtime_trace(
            "wechat",
            &format!("bind_terminal click session={session_id} queued={queued}"),
        );
        self.invalidate_task_column(cx);
        self.invalidate_ui_region(cx, UiRegion::Root);
    }

    /// The PTY session id of the active tab's first attached pane — the id
    /// `TerminalManager::write` expects (session id == terminal id).
    pub(in crate::app) fn active_terminal_session_id(&self) -> Option<String> {
        if let Some(session_id) = self
            .active_terminal_slot()
            .and_then(|(_, slot)| slot.terminal_id.clone())
            .or_else(|| {
                self.active_terminal()
                    .and_then(|tab| tab.terminal_id.clone())
            })
            .filter(|session_id| !session_id.trim().is_empty())
        {
            return Some(session_id);
        }

        let remembered = self.active_terminal_runtime_id();
        if !remembered.trim().is_empty()
            && self
                .terminal_manager
                .list()
                .iter()
                .any(|session| session.id == remembered)
        {
            return Some(remembered);
        }

        self.terminal_manager
            .list()
            .into_iter()
            .filter(|session| session.is_running)
            .max_by(|left, right| left.last_active_at.cmp(&right.last_active_at))
            .or_else(|| {
                self.terminal_manager
                    .list()
                    .into_iter()
                    .max_by(|left, right| left.last_active_at.cmp(&right.last_active_at))
            })
            .map(|session| session.id)
            .or_else(wechat_bridge_fallback_terminal_session_id)
    }
}
