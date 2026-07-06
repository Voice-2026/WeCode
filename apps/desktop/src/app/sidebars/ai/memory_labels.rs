use super::*;

pub(super) fn memory_module_key(entry: &MemoryEntrySummary) -> String {
    let module = entry.module_key.trim();
    if module.is_empty() {
        "general".to_string()
    } else {
        module.to_string()
    }
}

pub(super) fn memory_kind_title(kind: &str, language: &str) -> String {
    let fallback = match kind {
        "preference" => "Preference",
        "convention" => "Convention",
        "decision" => "Decision",
        "fact" => "Fact",
        "bug_lesson" => "Bug Lesson",
        _ => kind,
    };
    ai_sidebar_text(language, &format!("memory.kind.{kind}"), fallback)
}

pub(super) fn memory_module_title(module_key: &str, language: &str) -> String {
    let fallback = match module_key {
        "general" => "General",
        "project" => "Project",
        "terminal" => "Terminal",
        "git" => "Git",
        "ui" => "UI",
        "runtime" => "Runtime",
        _ => module_key,
    };
    ai_sidebar_text(language, &format!("memory.module.{module_key}"), fallback)
}

pub(super) fn memory_tier_title(tier: &str, language: &str) -> String {
    let fallback = match tier {
        "core" => "Core",
        "working" => "Working",
        "archive" => "Archive",
        _ => tier,
    };
    ai_sidebar_text(language, &format!("memory.tier.{tier}"), fallback)
}

pub(super) fn memory_status_title(status: &str, language: &str) -> String {
    let fallback = match status {
        "active" => "Active",
        "merged" => "Merged",
        "archived" => "Archived",
        _ => status,
    };
    ai_sidebar_text(language, &format!("memory.status.{status}"), fallback)
}

pub(super) fn memory_extraction_status_label(status: &str, language: &str) -> String {
    match status {
        "running" => ai_sidebar_text(language, "memory.status.short_remembering", "Remembering"),
        "queued" | "pending" => ai_sidebar_text(language, "memory.status.short_queued", "Queued"),
        "failed" => ai_sidebar_text(language, "memory.status.short_failed", "Failed"),
        _ => status.to_string(),
    }
}

pub(super) fn memory_decision_title(decision: &str, language: &str) -> String {
    let fallback = match decision {
        "create" => "Created",
        "merge" => "Merged",
        "replace" => "Replaced",
        "archive" => "Archived",
        "skip" => "Skipped",
        _ => decision,
    };
    ai_sidebar_text(language, &format!("memory.decision.{decision}"), fallback)
}

pub(super) fn memory_kind_color(kind: &str) -> Hsla {
    color(match kind {
        "preference" => 0x8C6FF7,
        "convention" => 0x2F7FBD,
        "decision" => 0xB8781D,
        "fact" => 0x337A6B,
        "bug_lesson" => 0xC25555,
        _ => 0x7B8190,
    })
}

pub(super) fn memory_status_color(status: &str) -> Hsla {
    color(match status {
        "active" => 0x2E9B5F,
        "merged" => 0x6E6E8B,
        "archived" => 0x7B8190,
        _ => 0x7B8190,
    })
}

pub(super) fn memory_decision_color(decision: &str) -> Hsla {
    color(match decision {
        "create" => 0x2E9B5F,
        "merge" => 0x3D80FA,
        "replace" => 0xB8781D,
        "archive" => 0x7B8190,
        "skip" => 0xC25555,
        _ => 0x7B8190,
    })
}

pub(super) fn memory_date_label(seconds: f64) -> String {
    chrono::Local
        .timestamp_opt(seconds as i64, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}
