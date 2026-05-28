use super::{
    helpers::{deterministic_uuid, history_group_key, table_has_column},
    types::{AIProjectUsageSummary, AISessionSummary, SessionDetailLink, SessionLink},
};
use rusqlite::{Connection, params};

pub(super) fn load_sessions(
    conn: &Connection,
    project_path: &str,
) -> Result<Vec<AISessionSummary>, String> {
    let mut statement = conn
        .prepare(
            r#"
            SELECT
                l.session_key,
                l.external_session_id,
                l.session_title,
                l.source,
                l.last_model,
                l.last_seen_at,
                COALESCE(SUM(b.total_tokens), 0) AS total_tokens,
                COALESCE(SUM(b.cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(b.request_count), 0) AS request_count
            FROM ai_history_file_session_link l
            LEFT JOIN ai_history_file_usage_bucket b
                ON b.project_path = l.project_path
                AND b.source = l.source
                AND b.file_path = l.file_path
                AND b.session_key = l.session_key
            WHERE l.project_path = ?1
            GROUP BY l.session_key, l.external_session_id, l.session_title, l.source, l.last_model, l.last_seen_at
            ORDER BY l.last_seen_at DESC
            LIMIT 12
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([project_path], |row| {
            let session_key = row.get::<_, String>(0)?;
            let external_session_id = row.get::<_, Option<String>>(1)?;
            let source = row.get::<_, String>(3)?;
            Ok(AISessionSummary {
                id: deterministic_uuid(&history_group_key(
                    &source,
                    &session_key,
                    external_session_id.as_deref(),
                )),
                session_key,
                external_session_id,
                title: row.get(2)?,
                source,
                last_model: row.get(4)?,
                last_seen_at: row.get(5)?,
                total_tokens: row.get(6)?,
                cached_input_tokens: row.get(7)?,
                request_count: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn load_global_recent_sessions(
    conn: &Connection,
) -> Result<Vec<AISessionSummary>, String> {
    let mut statement = conn
        .prepare(
            r#"
            SELECT
                l.session_key,
                l.external_session_id,
                l.session_title,
                l.source,
                l.last_model,
                MAX(l.last_seen_at) AS last_seen_at,
                COALESCE(SUM(b.total_tokens), 0) AS total_tokens,
                COALESCE(SUM(b.cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(b.request_count), 0) AS request_count
            FROM ai_history_file_session_link l
            LEFT JOIN ai_history_file_usage_bucket b
                ON b.project_path = l.project_path
                AND b.source = l.source
                AND b.file_path = l.file_path
                AND b.session_key = l.session_key
            GROUP BY l.source, l.session_key, l.external_session_id, l.session_title, l.last_model
            ORDER BY last_seen_at DESC
            LIMIT 10
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], |row| {
            let session_key = row.get::<_, String>(0)?;
            let external_session_id = row.get::<_, Option<String>>(1)?;
            let source = row.get::<_, String>(3)?;
            Ok(AISessionSummary {
                id: deterministic_uuid(&history_group_key(
                    &source,
                    &session_key,
                    external_session_id.as_deref(),
                )),
                session_key,
                external_session_id,
                title: row.get(2)?,
                source,
                last_model: row.get(4)?,
                last_seen_at: row.get(5)?,
                total_tokens: row.get(6)?,
                cached_input_tokens: row.get(7)?,
                request_count: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn load_global_project_totals(
    conn: &Connection,
    today_start: f64,
) -> Result<Vec<AIProjectUsageSummary>, String> {
    let project_name_expr =
        if table_has_column(conn, "ai_history_project_index_state", "project_name") {
            "COALESCE(NULLIF(MAX(p.project_name), ''), l.project_path)"
        } else {
            "l.project_path"
        };
    let sql = format!(
        r#"
        SELECT
            l.project_path,
            {project_name_expr} AS project_name,
            COUNT(DISTINCT l.source || ':' || l.session_key) AS session_count,
            COALESCE(SUM(b.total_tokens), 0) AS total_tokens,
            COALESCE(SUM(b.cached_input_tokens), 0) AS cached_input_tokens,
            COALESCE(SUM(CASE WHEN b.bucket_start >= ?1 THEN b.total_tokens ELSE 0 END), 0) AS today_total_tokens
        FROM ai_history_file_session_link l
        LEFT JOIN ai_history_project_index_state p
            ON p.project_path = l.project_path
        LEFT JOIN ai_history_file_usage_bucket b
            ON b.project_path = l.project_path
            AND b.source = l.source
            AND b.file_path = l.file_path
            AND b.session_key = l.session_key
        GROUP BY l.project_path
        ORDER BY total_tokens DESC, l.project_path ASC
        LIMIT 12
        "#
    );
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([today_start], |row| {
            Ok(AIProjectUsageSummary {
                project_path: row.get(0)?,
                project_name: row.get(1)?,
                session_count: row.get::<_, i64>(2)?.max(0) as usize,
                total_tokens: row.get(3)?,
                cached_input_tokens: row.get(4)?,
                today_total_tokens: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn load_session_detail_links(
    conn: &Connection,
    project_path: &str,
) -> Result<Vec<SessionDetailLink>, String> {
    let has_first_seen = table_has_column(conn, "ai_history_file_session_link", "first_seen_at");
    let has_active_duration = table_has_column(
        conn,
        "ai_history_file_session_link",
        "active_duration_seconds",
    );
    let first_seen_expr = if has_first_seen {
        "first_seen_at"
    } else {
        "last_seen_at"
    };
    let active_duration_expr = if has_active_duration {
        "active_duration_seconds"
    } else {
        "0"
    };
    let sql = format!(
        r#"
        SELECT
            source,
            file_path,
            session_key,
            external_session_id,
            session_title,
            {first_seen_expr},
            last_seen_at,
            last_model,
            {active_duration_expr}
        FROM ai_history_file_session_link
        WHERE project_path = ?1
        ORDER BY last_seen_at DESC
        "#
    );
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([project_path], |row| {
            Ok(SessionDetailLink {
                source: row.get(0)?,
                file_path: row.get(1)?,
                session_key: row.get(2)?,
                external_session_id: row.get(3)?,
                title: row.get(4)?,
                first_seen_at: row.get(5)?,
                last_seen_at: row.get(6)?,
                last_model: row.get(7)?,
                active_duration_seconds: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn load_global_today_cached_tokens(
    conn: &Connection,
    today_start: f64,
) -> Result<i64, String> {
    conn.query_row(
        r#"
        SELECT COALESCE(SUM(cached_input_tokens), 0)
        FROM ai_history_file_usage_bucket
        WHERE bucket_start >= ?1
        "#,
        [today_start],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

pub(super) fn load_file_usage(
    conn: &Connection,
    project_path: &str,
    source: &str,
    file_path: &str,
    session_key: &str,
) -> Result<(i64, i64, i64), String> {
    conn.query_row(
        r#"
        SELECT
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(cached_input_tokens), 0),
            COALESCE(SUM(request_count), 0)
        FROM ai_history_file_usage_bucket
        WHERE project_path = ?1 AND source = ?2 AND file_path = ?3 AND session_key = ?4
        "#,
        params![project_path, source, file_path, session_key],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|error| error.to_string())
}

pub(super) fn load_session_links(
    conn: &Connection,
    project_path: &str,
) -> Result<Vec<SessionLink>, String> {
    let mut statement = conn
        .prepare(
            r#"
            SELECT source, session_key, external_session_id
            FROM ai_history_file_session_link
            WHERE project_path = ?1
            ORDER BY last_seen_at DESC
            "#,
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([project_path], |row| {
            Ok(SessionLink {
                source: row.get(0)?,
                session_key: row.get(1)?,
                external_session_id: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn load_today_tokens(
    conn: &Connection,
    project_path: &str,
    today_start: f64,
) -> Result<(i64, i64), String> {
    conn.query_row(
        r#"
        SELECT
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(cached_input_tokens), 0)
        FROM ai_history_file_usage_bucket
        WHERE project_path = ?1 AND bucket_start >= ?2
        "#,
        (project_path, today_start),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|error| error.to_string())
}
