use super::*;

pub(super) fn attach_ai_runtime_terminal_output_watcher(
    session: &TerminalPtySession,
    ai_runtime: Arc<AIRuntimeBridge>,
) {
    let binding = session.ai_runtime_binding();
    let watcher = Arc::new(parking_lot::Mutex::new(
        AIRuntimeTerminalOutputWatcher::new(binding, ai_runtime),
    ));
    session.subscribe_events(Arc::new(move |event| {
        watcher.lock().handle_terminal_event(&event);
        true
    }));
}

pub(super) struct AIRuntimeTerminalOutputWatcher {
    binding: AIRuntimeTerminalBinding,
    ai_runtime: Arc<AIRuntimeBridge>,
    parser: TerminalProgressOscParser,
    last_activity_at: f64,
    last_screen_signal_at: f64,
}

/// Throttle output heartbeats so a chatty turn does not lock the state store on
/// every byte; one refresh per second is ample against the 90s staleness sweep.
const OUTPUT_ACTIVITY_THROTTLE_SECONDS: f64 = 1.0;
const SCREEN_SIGNAL_THROTTLE_SECONDS: f64 = 0.25;

impl AIRuntimeTerminalOutputWatcher {
    pub(super) fn new(binding: AIRuntimeTerminalBinding, ai_runtime: Arc<AIRuntimeBridge>) -> Self {
        Self {
            binding,
            ai_runtime,
            parser: TerminalProgressOscParser::default(),
            last_activity_at: 0.0,
            last_screen_signal_at: 0.0,
        }
    }

    pub(super) fn handle_terminal_event(&mut self, event: &TerminalEvent) {
        let TerminalEvent::Output {
            session_id, bytes, ..
        } = event
        else {
            return;
        };
        if session_id != &self.binding.terminal_id {
            return;
        }
        let now = now_seconds();
        if now - self.last_activity_at >= OUTPUT_ACTIVITY_THROTTLE_SECONDS {
            self.last_activity_at = now;
            // Keeps an in-flight turn's loading state alive on genuine output;
            // a no-op unless a `responding` turn already exists, so plain shell
            // or service-command output never fabricates AI activity.
            self.ai_runtime
                .note_output_activity(&self.binding.terminal_id, now);
        }
        if now - self.last_screen_signal_at >= SCREEN_SIGNAL_THROTTLE_SECONDS {
            self.last_screen_signal_at = now;
            self.ai_runtime
                .refresh_screen_signal(&self.binding.terminal_id);
        }
        for progress in self.parser.push(bytes) {
            match progress {
                TerminalProgressOsc::Started => {
                    if self.current_session_is_codewhale() {
                        self.submit_progress_hook("promptSubmitted", false);
                    }
                }
                TerminalProgressOsc::Completed => {
                    if self.current_session_is_running() {
                        self.submit_progress_hook("turnCompleted", true);
                    }
                }
            }
        }
    }

    fn current_session_is_running(&self) -> bool {
        self.current_session_is_codewhale_with_state(|state| {
            matches!(state, "responding" | "needsInput")
        })
    }

    fn current_session_is_codewhale(&self) -> bool {
        self.current_session_is_codewhale_with_state(|state| {
            matches!(state, "idle" | "responding" | "needsInput")
        })
    }

    fn current_session_is_codewhale_with_state(
        &self,
        state_matches: impl Fn(&str) -> bool,
    ) -> bool {
        self.ai_runtime
            .runtime_state_snapshot()
            .sessions
            .iter()
            .any(|session| {
                session.terminal_id == self.binding.terminal_id
                    && canonical_tool_name(&session.tool).as_deref() == Some("codewhale")
                    && state_matches(session.state.as_str())
            })
    }

    fn submit_progress_hook(&self, kind: &str, has_completed_turn: bool) {
        let existing = self
            .ai_runtime
            .runtime_state_snapshot()
            .sessions
            .into_iter()
            .find(|session| session.terminal_id == self.binding.terminal_id);
        let payload = AIHookEventPayload {
            kind: kind.to_string(),
            terminal_id: self.binding.terminal_id.clone(),
            terminal_instance_id: self.binding.terminal_instance_id.clone(),
            project_id: self.binding.project_id.clone(),
            project_name: existing
                .as_ref()
                .map(|session| session.project_name.clone())
                .unwrap_or_else(|| "Workspace".to_string()),
            project_path: existing
                .as_ref()
                .and_then(|session| session.project_path.clone())
                .or_else(|| Some(self.binding.cwd.clone())),
            session_title: existing
                .as_ref()
                .map(|session| session.session_title.clone())
                .unwrap_or_else(|| self.binding.title.clone()),
            tool: "codewhale".to_string(),
            ai_session_id: existing
                .as_ref()
                .and_then(|session| session.ai_session_id.clone())
                .or_else(|| self.binding.session_key.clone()),
            model: existing.as_ref().and_then(|session| session.model.clone()),
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            total_tokens: None,
            updated_at: now_seconds(),
            metadata: Some(AIHookEventMetadata {
                transcript_path: None,
                notification_type: None,
                source: Some("terminal-progress-osc".to_string()),
                reason: Some(
                    if has_completed_turn {
                        "progress-completed"
                    } else {
                        "progress-started"
                    }
                    .to_string(),
                ),
                cwd: Some(self.binding.cwd.clone()),
                target_tool_name: None,
                message: None,
                was_interrupted: Some(false),
                has_completed_turn: Some(has_completed_turn),
            }),
        };
        if let Err(error) = self.ai_runtime.submit_hook_event(payload) {
            crate::ai_runtime::runtime_log_line(
                "terminal-ai-runtime",
                &format!(
                    "submit codewhale progress hook failed terminal={} kind={} error={}",
                    self.binding.terminal_id, kind, error
                ),
            );
        }
    }
}
