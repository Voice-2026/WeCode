pub async fn pet_idle_speech_with_settings(
    settings: &AISettings,
    language: &str,
    request: PetIdleSpeechRequest,
) -> Result<PetIdleSpeechResponse, String> {
    if !settings.pet.speech_llm_enabled || settings.pet.speech_mode == "off" {
        return Ok(PetIdleSpeechResponse {
            text: String::new(),
        });
    }
    let now = chrono::Utc::now().timestamp();
    if settings
        .pet
        .speech_temporary_mute_until
        .is_some_and(|until| until > now)
        || quiet_hours_active(settings)
    {
        return Ok(PetIdleSpeechResponse {
            text: String::new(),
        });
    }
    let locale = locale_from_language_setting(language);
    let fallback_text = normalized_non_empty(&request.fallback_text);
    let system_prompt = pet_speech_system_prompt(&locale);
    let prompt = if let Some(fallback_text) = fallback_text {
        pet_speech_event_prompt(&request, &fallback_text)
    } else {
        pet_speech_idle_prompt(&locale)
    };
    let provider = select_provider(
        settings,
        Some(settings.pet.speech_provider_id.as_str()),
        "petSpeech",
    )
    .ok_or_else(|| "No available AI provider is configured for pet speech.".to_string())?;
    runtime_trace(
        "ai-pet",
        &format!(
            "speech request provider_id={} kind={} model={} event={} prompt_chars={}",
            provider.id,
            provider.kind,
            fallback_model(provider, default_model_for_provider_kind(&provider.kind)),
            normalized_non_empty(&request.event).unwrap_or_else(|| "idle".to_string()),
            prompt.chars().count()
        ),
    );
    let response_text = complete_with_provider_options(
        provider,
        &prompt,
        Some(&system_prompt),
        LLMProviderCompletionOptions {
            max_tokens: 80,
            temperature: 0.2,
            preserve_formatting: true,
            json_response: true,
            ..LLMProviderCompletionOptions::default()
        },
    )
    .await?;
    let text = decode_pet_speech_response(&response_text);
    let text = sanitize_pet_speech_line(&text);
    runtime_trace(
        "ai-pet",
        &format!("speech response text_chars={}", text.chars().count()),
    );
    Ok(PetIdleSpeechResponse { text })
}

fn quiet_hours_active(settings: &AISettings) -> bool {
    let Some(start) = settings.pet.speech_quiet_hours_start else {
        return false;
    };
    let Some(end) = settings.pet.speech_quiet_hours_end else {
        return false;
    };
    if start == end {
        return false;
    }
    let hour = Local::now().hour() as i32;
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

fn pet_speech_system_prompt(locale: &str) -> String {
    let language = pet_speech_language_label(locale);
    format!("Return minified JSON only: {{\"text\":\"...\"}}. One short safe {language} pet line.")
}

fn pet_speech_language_label(locale: &str) -> &'static str {
    let normalized = locale.replace('_', "-").to_lowercase();
    if normalized.starts_with("zh-hant") {
        "Traditional Chinese"
    } else if normalized.starts_with("zh") {
        "Simplified Chinese"
    } else if normalized.starts_with("ja") {
        "Japanese"
    } else if normalized.starts_with("ko") {
        "Korean"
    } else if normalized.starts_with("fr") {
        "French"
    } else if normalized.starts_with("de") {
        "German"
    } else if normalized.starts_with("es") {
        "Spanish"
    } else if normalized.starts_with("pt") {
        "Portuguese"
    } else if normalized.starts_with("ru") {
        "Russian"
    } else {
        "English"
    }
}

fn pet_speech_event_prompt(request: &PetIdleSpeechRequest, fallback_text: &str) -> String {
    format!(
        "Event: {}\nFallback line: {}\nReturn {{\"text\":\"...\"}}.",
        normalized_non_empty(&request.event).unwrap_or_else(|| "activity".to_string()),
        fallback_text
    )
}

fn pet_speech_idle_prompt(locale: &str) -> String {
    let _ = locale;
    "Event: idle\nReturn {\"text\":\"...\"}.".to_string()
}

fn decode_pet_speech_response(raw: &str) -> String {
    let value = serde_json::from_str::<Value>(raw)
        .ok()
        .or_else(|| llm_json_repair::parse::<Value>(raw).ok());
    if let Some(text) = value
        .as_ref()
        .and_then(|value| value.as_object())
        .and_then(|object| {
            ["text", "line", "message", "content", "response"]
                .iter()
                .find_map(|key| object.get(*key)?.as_str())
        })
    {
        return text.to_string();
    }
    raw.to_string()
}

fn sanitize_pet_speech_line(text: &str) -> String {
    sanitize_response_line(text).chars().take(80).collect()
}
