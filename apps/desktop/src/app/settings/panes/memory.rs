use super::options::*;
use super::widgets::*;
use super::*;

pub(super) fn settings_memory_pane(
    settings: &SettingsSummary,
    _memory: &MemorySummary,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let language = settings.language.as_str();
    let mut cards = vec![
        settings_card(
            Some(settings_text(
                language,
                "settings.ai.section.memory",
                "Memory",
            )),
            None,
            vec![
                settings_row(
                    settings_text(language, "settings.ai.memory.enabled", "Enable Memory"),
                    None,
                    settings_toggle(
                        "settings-memory-enabled",
                        settings.memory_enabled,
                        cx,
                        |app, window, cx| {
                            let next = !app.state.settings.memory_enabled;
                            app.set_ai_memory_bool("enabled", next, window, cx)
                        },
                    ),
                )
                .into_any_element(),
            ],
            cx,
        )
        .into_any_element(),
    ];

    if settings.memory_enabled {
        cards.push(
            settings_card(
                Some(settings_text(
                    language,
                    "settings.ai.memory.automatic_injection",
                    "Automatic Injection",
                )),
                None,
                vec![
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.automatic_injection",
                            "Automatic Injection",
                        ),
                        None,
                        settings_toggle(
                            "settings-memory-auto-injection",
                            settings.memory_automatic_injection_enabled,
                            cx,
                            |app, window, cx| {
                                let next = !app.state.settings.memory_automatic_injection_enabled;
                                app.set_ai_memory_bool(
                                    "automaticInjectionEnabled",
                                    next,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.automatic_extraction",
                            "Automatic Extraction",
                        ),
                        None,
                        settings_toggle(
                            "settings-memory-auto-extraction",
                            settings.memory_automatic_extraction_enabled,
                            cx,
                            |app, window, cx| {
                                let next = !app.state.settings.memory_automatic_extraction_enabled;
                                app.set_ai_memory_bool(
                                    "automaticExtractionEnabled",
                                    next,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.extraction_interval",
                            "Extraction Interval",
                        ),
                        None,
                        settings_select_impl(
                            "settings-memory-extraction-interval",
                            &settings.memory_extraction_idle_delay_seconds,
                            memory_extraction_interval_options(),
                            window,
                            cx,
                            language,
                            |app, value, window, cx| {
                                app.set_ai_memory_number(
                                    "extractionIdleDelaySeconds",
                                    value,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.heuristic_gate",
                            "Skip Low-Signal Sessions",
                        ),
                        Some(settings_text(
                            language,
                            "settings.ai.memory.heuristic_gate.help",
                            "Avoid LLM extraction for transcripts that do not contain durable memory signals.",
                        )),
                        settings_toggle(
                            "settings-memory-heuristic-gate",
                            settings.memory_extraction_heuristic_gate_enabled,
                            cx,
                            |app, window, cx| {
                                let next =
                                    !app.state.settings.memory_extraction_heuristic_gate_enabled;
                                app.set_ai_memory_bool(
                                    "extractionHeuristicGateEnabled",
                                    next,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.growth_threshold",
                            "Minimum Transcript Growth",
                        ),
                        Some(settings_text(
                            language,
                            "settings.ai.memory.growth_threshold.help",
                            "Skip repeat extraction until a session grows by at least this many lines.",
                        )),
                        settings_text_input(
                            "settings-memory-growth-threshold",
                            &settings.memory_extraction_growth_threshold_lines,
                            "8",
                            false,
                            window,
                            cx,
                            |app, value, window, cx| {
                                app.set_ai_memory_number(
                                    "extractionGrowthThresholdLines",
                                    value,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.max_index_sessions",
                            "Maximum Recent Sessions",
                        ),
                        None,
                        settings_select_impl(
                            "settings-memory-max-index",
                            &settings.memory_max_index_sessions,
                            memory_max_index_options(language),
                            window,
                            cx,
                            language,
                            |app, value, window, cx| {
                                app.set_ai_memory_number("maxIndexSessions", value, window, cx)
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.recall_fts",
                            "Use FTS Recall",
                        ),
                        Some(settings_text(
                            language,
                            "settings.ai.memory.recall_fts.help",
                            "Use local SQLite BM25 search to recall older relevant memories.",
                        )),
                        settings_toggle(
                            "settings-memory-recall-fts",
                            settings.memory_recall_use_fts,
                            cx,
                            |app, window, cx| {
                                let next = !app.state.settings.memory_recall_use_fts;
                                app.set_ai_memory_bool("recallUseFts", next, window, cx)
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.privacy_scrub",
                            "Redact Secrets Before Saving",
                        ),
                        Some(settings_text(
                            language,
                            "settings.ai.memory.privacy_scrub.help",
                            "Redact common API keys, tokens and private keys before memory is stored.",
                        )),
                        settings_toggle(
                            "settings-memory-privacy-scrub",
                            settings.memory_privacy_scrub_enabled,
                            cx,
                            |app, window, cx| {
                                let next = !app.state.settings.memory_privacy_scrub_enabled;
                                app.set_ai_memory_bool("privacyScrubEnabled", next, window, cx)
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.cross_project_user",
                            "Cross-Project User Memory",
                        ),
                        None,
                        settings_toggle(
                            "settings-memory-cross-project",
                            settings.memory_allow_cross_project_user_recall,
                            cx,
                            |app, window, cx| {
                                let next =
                                    !app.state.settings.memory_allow_cross_project_user_recall;
                                app.set_ai_memory_bool(
                                    "allowCrossProjectUserRecall",
                                    next,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                ],
                cx,
            )
            .into_any_element(),
        );
        cards.push(
            settings_card(
                Some(settings_text(
                    language,
                    "settings.ai.memory.default_extraction_provider",
                    "Default Extraction Provider",
                )),
                None,
                vec![
                    settings_row(
                        settings_text(
                            language,
                            "settings.ai.memory.default_extraction_provider",
                            "Default Extraction Provider",
                        ),
                        None,
                        settings_select_impl(
                            "settings-memory-provider",
                            &settings.memory_default_extractor_provider_id,
                            ai_provider_options(settings, "memory", language),
                            window,
                            cx,
                            language,
                            |app, value, window, cx| app.set_ai_memory_provider(value, window, cx),
                        ),
                    )
                    .into_any_element(),
                ],
                cx,
            )
            .into_any_element(),
        );
    }

    settings_form(cards).into_any_element()
}
