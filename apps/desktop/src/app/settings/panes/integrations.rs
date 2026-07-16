use super::widgets::*;
use super::*;
use wecode_integrations::{AgentIntegrationStatus, IntegrationManager};

pub(super) fn settings_integrations_pane(
    settings: &SettingsSummary,
    loading: bool,
    _window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let language = settings.language.as_str();
    let state =
        IntegrationManager::discover().map(|manager| (manager.snapshot(), manager.cli_status()));
    let Ok((snapshot, cli)) = state else {
        return settings_form(vec![
            settings_card(
                Some(settings_text(
                    language,
                    "settings.integrations.title",
                    "Agent Integrations",
                )),
                Some(settings_text(
                    language,
                    "settings.integrations.unavailable",
                    "Integration status is unavailable in this environment.",
                )),
                Vec::new(),
                cx,
            )
            .into_any_element(),
        ])
        .into_any_element();
    };

    let cli_label = if cli.installed {
        settings_text(language, "settings.integrations.installed", "Installed")
    } else {
        settings_text(language, "settings.integrations.install", "Install")
    };
    let cli_control = settings_small_button_state(
        "settings-integrations-cli",
        cli_label,
        loading,
        loading || cli.installed || !cli.bundled,
        cx,
        |app, _event, _window, cx| app.install_bundled_cli(cx),
    );

    let mut skill_rows = snapshot
        .agents
        .into_iter()
        .map(|agent| integration_agent_row(agent, snapshot.source_available, loading, language, cx))
        .collect::<Vec<_>>();

    if snapshot.update_available {
        skill_rows.insert(
            0,
            settings_row(
                settings_text(
                    language,
                    "settings.integrations.update_available",
                    "Skill update available",
                ),
                Some(settings_text(
                    language,
                    "settings.integrations.update_description",
                    "Refresh the canonical wecode-control Skill used by every linked Agent.",
                )),
                settings_small_button_state(
                    "settings-integrations-update",
                    settings_text(language, "settings.integrations.update", "Update"),
                    loading,
                    loading,
                    cx,
                    |app, _event, _window, cx| app.update_agent_integrations(cx),
                ),
            )
            .into_any_element(),
        );
    }

    settings_form(vec![
        settings_card(
            Some(settings_text(
                language,
                "settings.integrations.cli.title",
                "Command Line Tool",
            )),
            Some(settings_text(
                language,
                "settings.integrations.cli.description",
                "Install the bundled wecode command so terminals and external Agents can control WeCode.",
            )),
            vec![settings_row(
                "wecode CLI",
                Some(format!("{}", cli.command_path.display())),
                cli_control,
            )
            .into_any_element()],
            cx,
        )
        .into_any_element(),
        settings_card(
            Some(settings_text(
                language,
                "settings.integrations.skill.title",
                "wecode-control Skill",
            )),
            Some(settings_text(
                language,
                "settings.integrations.skill.description",
                "WeCode keeps one canonical Skill copy and links it to each Agent you choose.",
            )),
            skill_rows,
            cx,
        )
        .into_any_element(),
    ])
    .into_any_element()
}

fn integration_agent_row(
    agent: AgentIntegrationStatus,
    source_available: bool,
    loading: bool,
    language: &str,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let kind = agent.agent;
    let (button_label, disabled, uninstall) = if agent.managed {
        (
            settings_text(language, "settings.integrations.remove", "Remove"),
            loading,
            true,
        )
    } else if agent.installed {
        (
            settings_text(language, "settings.integrations.existing", "Existing"),
            true,
            false,
        )
    } else if !agent.detected {
        (
            settings_text(
                language,
                "settings.integrations.not_detected",
                "Not detected",
            ),
            true,
            false,
        )
    } else {
        (
            settings_text(language, "settings.integrations.install", "Install"),
            loading || !source_available,
            false,
        )
    };
    let id = format!("settings-integrations-agent-{}", kind.id());
    settings_row(
        agent.display_name,
        Some(format!("{} · {}", agent.detail, agent.path.display())),
        settings_small_button_state(
            id,
            button_label,
            loading,
            disabled,
            cx,
            move |app, _event, _window, cx| {
                if uninstall {
                    app.uninstall_agent_integration(kind, cx);
                } else {
                    app.install_agent_integration(kind, cx);
                }
            },
        ),
    )
    .into_any_element()
}
