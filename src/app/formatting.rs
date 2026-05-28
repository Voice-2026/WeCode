use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::app) fn compact_number(value: i64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if abs >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
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
