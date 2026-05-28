impl SettingsService {
    pub fn set_ai_memory_bool(&self, key: &str, value: bool) -> Result<SettingsSummary, String> {
        let key = match key {
            "enabled" => "enabled",
            "automaticInjectionEnabled" => "automaticInjectionEnabled",
            "automaticExtractionEnabled" => "automaticExtractionEnabled",
            "allowCrossProjectUserRecall" => "allowCrossProjectUserRecall",
            _ => return Err("Unsupported memory setting.".to_string()),
        };
        let mut raw = self.raw_settings();
        let memory = ai_memory_mut(&mut raw)?;
        memory.insert(key.to_string(), Value::Bool(value));
        self.save_raw_settings(&raw)?;
        Ok(summary_from_raw(&raw))
    }

    pub fn set_ai_memory_number(&self, key: &str, value: &str) -> Result<SettingsSummary, String> {
        let (key, allowed, default) = match key {
            "extractionIdleDelaySeconds" => (
                "extractionIdleDelaySeconds",
                &[60, 120, 300, 600, 900][..],
                300,
            ),
            "maxIndexSessions" => ("maxIndexSessions", &[5, 10, 20, 50, 100][..], 20),
            _ => return Err("Unsupported memory setting.".to_string()),
        };
        let parsed = numeric_string(value, default, 1, 86_400);
        let value = allowed
            .iter()
            .find(|option| **option == parsed)
            .copied()
            .unwrap_or(default);
        let mut raw = self.raw_settings();
        let memory = ai_memory_mut(&mut raw)?;
        memory.insert(key.to_string(), Value::Number(value.into()));
        self.save_raw_settings(&raw)?;
        Ok(summary_from_raw(&raw))
    }

    pub fn set_ai_memory_provider(&self, provider_id: &str) -> Result<SettingsSummary, String> {
        let mut raw = self.raw_settings();
        let memory = ai_memory_mut(&mut raw)?;
        memory.insert(
            "defaultExtractorProviderId".to_string(),
            Value::String(sanitize_provider_reference(provider_id)),
        );
        self.save_raw_settings(&raw)?;
        Ok(summary_from_raw(&raw))
    }
}
