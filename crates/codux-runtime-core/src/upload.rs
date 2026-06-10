use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn terminal_upload_directory(session_id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("CoduxUploads")
        .join(sanitized_upload_name(session_id))
}

pub fn sanitized_upload_name(value: &str) -> String {
    let name = Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("upload.png");
    let cleaned = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string();
    if cleaned.is_empty() {
        "upload.png".to_string()
    } else {
        cleaned
    }
}

pub fn terminal_upload_kind(payload: &Value) -> String {
    let kind = payload
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("image")
        .trim()
        .to_ascii_lowercase();
    if kind == "file" {
        "file".to_string()
    } else {
        "image".to_string()
    }
}

pub fn terminal_upload_path_input(path: &Path) -> String {
    quote_terminal_path(&path.to_string_lossy())
}

#[cfg(windows)]
pub fn quote_terminal_path(value: &str) -> String {
    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '&' | '(' | ')' | '[' | ']' | '{' | '}'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(not(windows))]
pub fn quote_terminal_path(value: &str) -> String {
    if value.chars().any(|ch| {
        ch.is_whitespace()
            || matches!(
                ch,
                '\'' | '"' | '\\' | '$' | '`' | '!' | '&' | '(' | ')' | ';' | '<' | '>' | '|'
            )
    }) {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.to_string()
    }
}

pub fn unique_upload_path(directory: &Path, file_name: &str) -> PathBuf {
    let file_name = sanitized_upload_name(file_name);
    let path = PathBuf::from(&file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("upload");
    let extension = path.extension().and_then(|value| value.to_str());
    let mut candidate = directory.join(&file_name);
    let mut index = 1;
    while candidate.exists() {
        let next = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem}-{index}.{extension}"),
            _ => format!("{stem}-{index}"),
        };
        candidate = directory.join(next);
        index += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_names_are_sanitized() {
        assert_eq!(
            sanitized_upload_name("../unsafe path/$image.png"),
            "_image.png"
        );
        assert_eq!(sanitized_upload_name("..."), "upload.png");
    }

    #[test]
    fn upload_path_quotes_shell_sensitive_paths() {
        assert_eq!(
            quote_terminal_path("/tmp/CoduxUploads/file.txt"),
            "/tmp/CoduxUploads/file.txt"
        );

        #[cfg(not(windows))]
        assert_eq!(
            terminal_upload_path_input(Path::new("/tmp/Codux Uploads/file name.txt")),
            "'/tmp/Codux Uploads/file name.txt'"
        );
    }
}
