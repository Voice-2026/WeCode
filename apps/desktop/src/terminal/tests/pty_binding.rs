use super::super::*;

#[test]
fn pending_terminal_binding_matches_requested_config_before_attach() {
    let config = terminal_pty_config_with_view(
        TerminalPtyConfig {
            cwd: Some("/tmp/project".to_string()),
            project_id: Some("project-1".to_string()),
            terminal_id: Some("terminal-1".to_string()),
            session_key: Some("gpui:project-1:terminal-1".to_string()),
            ..Default::default()
        },
        &terminal_config(),
    );

    let (binding, _initial_layout_rx) = TerminalSessionBinding::pending(config.clone());

    assert!(binding.matches_pty_config(&config));

    let mut different_terminal = config;
    different_terminal.terminal_id = Some("terminal-2".to_string());
    assert!(!binding.matches_pty_config(&different_terminal));
}
