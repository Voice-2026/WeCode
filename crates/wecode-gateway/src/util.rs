use sha2::{Digest, Sha256};

/// SHA256 of "{hostname}-{username}-kiro-gateway", matching the Python gateway
/// so the KiroIDE User-Agent fingerprint is identical.
pub fn machine_fingerprint() -> String {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    let username = whoami::username();
    let unique = format!("{hostname}-{username}-kiro-gateway");
    let mut hasher = Sha256::new();
    hasher.update(unique.as_bytes());
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn generate_completion_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4().simple())
}

pub fn generate_message_id() -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("msg_{}", &id[..24])
}

pub fn generate_tool_call_id() -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("call_{}", &id[..8])
}

pub fn generate_tool_use_id() -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("toolu_{}", &id[..24])
}

pub fn generate_thinking_signature() -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("sig_{}", &id[..32])
}

/// Headers for the Kiro generateAssistantResponse call.
pub fn kiro_headers(token: &str, fingerprint: &str) -> Vec<(String, String)> {
    vec![
        ("Authorization".into(), format!("Bearer {token}")),
        (
            "Content-Type".into(),
            "application/x-amz-json-1.0".into(),
        ),
        (
            "x-amz-target".into(),
            "AmazonCodeWhispererStreamingService.GenerateAssistantResponse".into(),
        ),
        (
            "User-Agent".into(),
            format!(
                "aws-sdk-js/1.0.27 ua/2.1 os/win32#10.0.19044 lang/js md/nodejs#22.21.1 api/codewhispererstreaming#1.0.27 m/E KiroIDE-0.7.45-{fingerprint}"
            ),
        ),
        (
            "x-amz-user-agent".into(),
            format!("aws-sdk-js/1.0.27 KiroIDE-0.7.45-{fingerprint}"),
        ),
        ("x-amzn-codewhisperer-optout".into(), "true".into()),
        ("x-amzn-kiro-agent-mode".into(), "vibe".into()),
        (
            "amz-sdk-invocation-id".into(),
            uuid::Uuid::new_v4().to_string(),
        ),
        ("amz-sdk-request".into(), "attempt=1; max=3".into()),
    ]
}
