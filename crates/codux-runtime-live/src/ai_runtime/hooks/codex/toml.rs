pub(super) fn codex_hook_state_key(line: &str) -> Option<String> {
    let trimmed = normalized_line(line);
    let raw = trimmed
        .strip_prefix("[hooks.state.")
        .and_then(|value| value.strip_suffix(']'))?;
    parse_toml_basic_string(raw).or_else(|| parse_toml_literal_string(raw))
}

fn parse_toml_basic_string(value: &str) -> Option<String> {
    let raw = value.strip_prefix('"')?.strip_suffix('"')?;
    let mut output = String::new();
    let mut chars = raw.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next()? {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            other => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    Some(output)
}

fn parse_toml_literal_string(value: &str) -> Option<String> {
    value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .map(str::to_string)
}

pub(super) fn is_toml_table_header(line: &str) -> bool {
    let trimmed = normalized_line(line);
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

pub(super) fn normalized_line(line: &str) -> String {
    line.trim().to_string()
}
