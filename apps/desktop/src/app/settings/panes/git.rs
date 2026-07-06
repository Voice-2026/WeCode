use super::options::*;
use super::widgets::*;
use super::*;

pub(super) fn settings_git_pane(
    settings: &SettingsSummary,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let language = settings.language.as_str();
    settings_form(vec![
        settings_card(
            Some(settings_text(
                language,
                "settings.ai.git_commit_message",
                "Git Commit Message",
            )),
            None,
            vec![
                settings_row(
                    settings_text(
                        language,
                        "settings.ai.git_commit_message_provider",
                        "AI Provider",
                    ),
                    None,
                    settings_select_impl(
                        "settings-git-provider-auto",
                        &settings.git_commit_provider_id,
                        git_provider_options(settings, language),
                        window,
                        cx,
                        language,
                        |app, value, window, cx| app.set_git_commit_provider(value, window, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.ai.git_commit_message_tone", "Tone"),
                    None,
                    settings_select_impl(
                        "settings-git-tone",
                        &settings.git_commit_tone,
                        git_tone_options(),
                        window,
                        cx,
                        language,
                        |app, value, window, cx| app.set_git_commit_tone(value, window, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.language", "Language"),
                    None,
                    settings_select_impl(
                        "settings-git-language",
                        &settings.git_commit_language,
                        git_language_options(language),
                        window,
                        cx,
                        language,
                        |app, value, window, cx| app.set_git_commit_language(value, window, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(
                        language,
                        "settings.ai.git_commit_message_style_rules",
                        "Style Rules",
                    ),
                    Some(settings_text(
                        language,
                        "settings.ai.git_commit_message_style_rules_placeholder",
                        "Example: use Conventional Commits, keep subject under 72 characters.",
                    )),
                    settings_textarea(
                        "git-style-rules",
                        &settings.git_commit_style_rules,
                        3,
                        settings_text(
                            language,
                            "settings.ai.git_commit_message_style_rules_placeholder",
                            "Example: use Conventional Commits, keep subject under 72 characters.",
                        ),
                        window,
                        cx,
                        |app, value, window, cx| app.set_git_commit_style_rules(value, window, cx),
                    ),
                )
                .into_any_element(),
            ],
            cx,
        )
        .into_any_element(),
    ])
    .into_any_element()
}
