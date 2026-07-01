pub(crate) fn privacy_scrub(text: &str) -> String {
    let mut output = redact_private_key_blocks(text);
    output = redact_authorization_headers(&output);
    output = redact_env_secrets(&output);
    output = redact_known_tokens(&output);
    output
}

fn redact_private_key_blocks(text: &str) -> String {
    let mut output = String::new();
    let mut in_key = false;
    for line in text.lines() {
        let mut rest = line;
        loop {
            if in_key {
                if let Some(end) = private_key_end_index(rest) {
                    in_key = false;
                    rest = &rest[end..];
                    continue;
                }
                break;
            }
            let Some(begin) = private_key_begin_index(rest) else {
                output.push_str(rest);
                break;
            };
            output.push_str(&rest[..begin]);
            output.push_str("[REDACTED_PRIVATE_KEY]");
            let after_begin = &rest[begin..];
            if let Some(end) = private_key_end_index(after_begin) {
                rest = &after_begin[end..];
            } else {
                in_key = true;
                break;
            }
        }
        output.push('\n');
    }
    if text.ends_with('\n') {
        output
    } else {
        output.trim_end_matches('\n').to_string()
    }
}

fn private_key_begin_index(line: &str) -> Option<usize> {
    line.match_indices("-----BEGIN ").find_map(|(index, _)| {
        line[index..]
            .find(" PRIVATE KEY-----")
            .map(|_| index)
    })
}

fn private_key_end_index(line: &str) -> Option<usize> {
    line.match_indices("-----END ").find_map(|(index, _)| {
        line[index..]
            .find(" PRIVATE KEY-----")
            .map(|end| index + end + " PRIVATE KEY-----".len())
    })
}

fn redact_authorization_headers(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_lowercase();
            if lower.trim_start().starts_with("authorization:") {
                let prefix_len = line.find(':').map(|index| index + 1).unwrap_or(line.len());
                format!("{} [REDACTED_SECRET]", &line[..prefix_len])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_env_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let Some((key, value)) = line.split_once('=') else {
                return line.to_string();
            };
            let key_trimmed = key.trim();
            if !looks_like_secret_key(key_trimmed) || value.trim().is_empty() {
                return line.to_string();
            }
            format!("{key_trimmed}=[REDACTED_SECRET]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn looks_like_secret_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    upper.contains("SECRET")
        || upper.contains("TOKEN")
        || upper.contains("PASSWORD")
        || upper.contains("PASSWD")
        || upper.contains("API_KEY")
        || upper.ends_with("_KEY")
        || upper.contains("PRIVATE_KEY")
}

fn redact_known_tokens(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut token = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            if !token.is_empty() {
                output.push_str(&redact_token(&token));
                token.clear();
            }
            output.push(character);
        } else {
            token.push(character);
        }
    }
    if !token.is_empty() {
        output.push_str(&redact_token(&token));
    }
    output
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| {
        matches!(ch, ',' | ';' | ':' | '"' | '\'' | ')' | ']' | '}')
    });
    let redacted = trimmed.starts_with("sk-")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("github_pat_")
        || trimmed.starts_with("xoxb-")
        || (trimmed.starts_with("AKIA") && trimmed.len() >= 16)
        || (trimmed.len() >= 32
            && !looks_like_durable_hash(trimmed)
            && trimmed.chars().all(|ch| ch.is_ascii_alphanumeric()));
    if !redacted {
        return token.to_string();
    }
    token.replacen(trimmed, "[REDACTED_SECRET]", 1)
}

fn looks_like_durable_hash(token: &str) -> bool {
    token.len() >= 32 && token.chars().all(|ch| ch.is_ascii_hexdigit())
}
