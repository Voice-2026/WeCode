use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::app) fn compact_number(value: i64) -> String {
    let abs = value.saturating_abs();
    if abs >= 1_000_000_000 {
        compact_unit(value, 1_000_000_000.0, "B")
    } else if abs >= 1_000_000 {
        compact_unit(value, 1_000_000.0, "M")
    } else if abs >= 1_000 {
        compact_unit(value, 1_000.0, "K")
    } else {
        value.to_string()
    }
}

fn compact_unit(value: i64, divisor: f64, suffix: &str) -> String {
    let scaled = value as f64 / divisor;
    let abs_scaled = scaled.abs();
    let formatted = if abs_scaled >= 100.0 {
        format!("{scaled:.0}")
    } else if abs_scaled >= 10.0 {
        format!("{scaled:.1}")
    } else {
        format!("{scaled:.2}")
    };
    format!(
        "{}{}",
        formatted.trim_end_matches('0').trim_end_matches('.'),
        suffix
    )
}

pub(in crate::app) fn relative_time_label(timestamp: f64) -> String {
    if timestamp <= 0.0 {
        return "刚刚".to_string();
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(timestamp);
    let seconds = (now - timestamp).max(0.0);

    if seconds < 60.0 {
        "刚刚".to_string()
    } else if seconds < 3600.0 {
        format!("{} 分钟前", (seconds / 60.0).floor() as i64)
    } else if seconds < 86_400.0 {
        format!("{} 小时前", (seconds / 3600.0).floor() as i64)
    } else if seconds < 604_800.0 {
        format!("{} 天前", (seconds / 86_400.0).floor() as i64)
    } else {
        format!("{} 周前", (seconds / 604_800.0).floor() as i64)
    }
}
