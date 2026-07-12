fn initialize_connection(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;

    for statement in SCHEMA_STATEMENTS {
        conn.execute_batch(statement)?;
    }

    let stored_version: Option<String> = conn
        .query_row(
            "SELECT value FROM ai_history_meta WHERE key = 'normalized_history_schema_version' LIMIT 1;",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if stored_version.as_deref() != Some(NORMALIZED_HISTORY_SCHEMA_VERSION) {
        migrate_schema(conn)?;
    }
    repair_project_fallback_session_titles(conn)?;
    Ok(())
}

fn repair_project_fallback_session_titles(conn: &Connection) -> Result<()> {
    conn.execute(
        r#"
        UPDATE ai_history_file_session_link AS session
        SET session_title = (
            SELECT json_extract(checkpoint.payload_json, '$.sessionTitle')
            FROM ai_history_file_checkpoint AS checkpoint
            WHERE checkpoint.source = session.source
              AND checkpoint.file_path = session.file_path
              AND checkpoint.project_path = session.project_path
            LIMIT 1
        )
        WHERE session.session_title = session.project_name
          AND NOT EXISTS (
              SELECT 1
              FROM ai_history_session_title_override AS title_override
              WHERE title_override.project_path = session.project_path
                AND title_override.source = session.source
                AND title_override.session_key = session.session_key
          )
          AND EXISTS (
              SELECT 1
              FROM ai_history_file_checkpoint AS checkpoint
              WHERE checkpoint.source = session.source
                AND checkpoint.file_path = session.file_path
                AND checkpoint.project_path = session.project_path
                AND TRIM(COALESCE(json_extract(checkpoint.payload_json, '$.sessionTitle'), '')) <> ''
                AND TRIM(json_extract(checkpoint.payload_json, '$.sessionTitle')) <> session.project_name
          );
        "#,
        [],
    )?;
    Ok(())
}

fn jsonl_index_mode(
    current_file_size: i64,
    current_modified_at: f64,
    stored_summary: Option<&AIExternalFileSummary>,
    checkpoint: Option<&AIExternalFileCheckpoint>,
) -> JSONLIndexMode {
    let (Some(stored_summary), Some(checkpoint)) = (stored_summary, checkpoint) else {
        return JSONLIndexMode::Rebuild;
    };
    if current_file_size < checkpoint.file_size {
        return JSONLIndexMode::Rebuild;
    }
    if checkpoint.last_offset < current_file_size {
        return JSONLIndexMode::Append;
    }
    if same_timestamp(stored_summary.file_modified_at, current_modified_at)
        && same_timestamp(checkpoint.file_modified_at, current_modified_at)
        && checkpoint.last_offset >= current_file_size
    {
        return JSONLIndexMode::Unchanged;
    }
    if current_file_size >= checkpoint.file_size && checkpoint.last_offset <= current_file_size {
        return JSONLIndexMode::Append;
    }
    JSONLIndexMode::Rebuild
}

fn merge_usage_buckets(existing: &[AIUsageBucket], delta: &[AIUsageBucket]) -> Vec<AIUsageBucket> {
    let mut map = HashMap::<(String, String, String, i64), AIUsageBucket>::new();
    for bucket in existing.iter().chain(delta.iter()) {
        let key = (
            bucket.source.clone(),
            bucket.session_key.clone(),
            bucket.model.clone().unwrap_or_default(),
            bucket.bucket_start as i64,
        );
        map.entry(key)
            .and_modify(|current| {
                current.input_tokens += bucket.input_tokens;
                current.output_tokens += bucket.output_tokens;
                current.total_tokens += bucket.total_tokens;
                current.cached_input_tokens += bucket.cached_input_tokens;
                merge_usage_amounts(&mut current.usage_amounts, &bucket.usage_amounts);
                current.request_count += bucket.request_count;
                current.active_duration_seconds += bucket.active_duration_seconds;
                current.first_seen_at = min_nonzero(current.first_seen_at, bucket.first_seen_at);
                current.last_seen_at = current.last_seen_at.max(bucket.last_seen_at);
                current.external_session_id = current
                    .external_session_id
                    .clone()
                    .or(bucket.external_session_id.clone());
                current.session_title = preferred_session_title(
                    Some(&current.session_title),
                    Some(&bucket.session_title),
                    &bucket.project_name,
                )
                .unwrap_or_else(|| bucket.project_name.clone());
                current.model = current.model.clone().or(bucket.model.clone());
            })
            .or_insert_with(|| bucket.clone());
    }
    let mut values = map.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.bucket_start
            .total_cmp(&right.bucket_start)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.session_key.cmp(&right.session_key))
            .then_with(|| left.model.cmp(&right.model))
    });
    values
}
