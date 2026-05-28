use super::http::remote_server_url;
use super::types::{RemotePairingInfo, RemoteSettings};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{Engine as _, engine::general_purpose};
use serde_json::json;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

pub(crate) fn remote_host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| crate::runtime_paths::app_display_name().to_string())
}

pub(crate) fn remote_random_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub(crate) fn ensure_remote_host_identity(settings: &mut RemoteSettings) {
    if let Some(private_key) = remote_e2e_private_key(&settings.host_private_key) {
        let public_key = X25519PublicKey::from(&private_key);
        let derived_public = remote_base64_url_encode(public_key.as_bytes());
        if settings.host_public_key.trim().is_empty() || settings.host_public_key == derived_public
        {
            settings.host_public_key = derived_public;
            return;
        }
    }
    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    let private_key = StaticSecret::from(bytes);
    let public_key = X25519PublicKey::from(&private_key);
    settings.host_private_key = remote_base64_url_encode(private_key.to_bytes().as_slice());
    settings.host_public_key = remote_base64_url_encode(public_key.as_bytes());
}

pub(crate) fn remote_pairing_qr_payload(
    settings: &RemoteSettings,
    pairing: &RemotePairingInfo,
) -> String {
    let payload = json!({
        "server": remote_server_url(settings),
        "code": pairing.code,
        "secret": pairing.secret,
        "hostName": remote_host_name(),
        "hostPublicKey": settings.host_public_key,
        "cryptoVersion": 1,
    });
    serde_json::to_vec(&payload)
        .ok()
        .map(|data| remote_base64_url_encode(&data))
        .unwrap_or_default()
}

pub(crate) fn remote_pairing_match_code(
    settings: &RemoteSettings,
    pairing_code: &str,
    pairing_secret: &str,
    device_public_key: &str,
) -> Option<String> {
    if settings.host_public_key.trim().is_empty() || device_public_key.trim().is_empty() {
        return None;
    }
    let material = format!(
        "codux-e2e-match-v1|{}|{}|{}|{}",
        settings.host_public_key, device_public_key, pairing_code, pairing_secret
    );
    let digest = Sha256::digest(material.as_bytes());
    let prefix = digest
        .iter()
        .take(3)
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    Some(format!("{}-{}", &prefix[..3], &prefix[3..]))
}

fn remote_e2e_private_key(value: &str) -> Option<StaticSecret> {
    let bytes = remote_base64_url_decode(value).ok()?;
    let array: [u8; 32] = bytes.as_slice().try_into().ok()?;
    Some(StaticSecret::from(array))
}

pub(crate) fn remote_e2e_symmetric_key(
    host_private_key: &str,
    remote_public_key: &str,
    host_id: &str,
    device_id: &str,
) -> Result<[u8; 32], String> {
    let private_key = remote_e2e_private_key(host_private_key)
        .ok_or_else(|| "Invalid host private key.".to_string())?;
    let public_bytes = remote_base64_url_decode(remote_public_key)?;
    let public_array: [u8; 32] = public_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid device public key.".to_string())?;
    let public_key = X25519PublicKey::from(public_array);
    let shared = private_key.diffie_hellman(&public_key);
    let salt = format!("codux-e2e-v1|{host_id}|{device_id}");
    let hkdf = hkdf::Hkdf::<Sha256>::new(Some(salt.as_bytes()), shared.as_bytes());
    let mut key = [0_u8; 32];
    hkdf.expand(b"codux-remote-payload-v1", &mut key)
        .map_err(|_| "Failed to derive encryption key.".to_string())?;
    Ok(key)
}

pub(crate) fn remote_e2e_encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    host_id: &str,
    device_id: &str,
) -> Result<Value, String> {
    let nonce_bytes = uuid::Uuid::new_v4().as_bytes()[..12].to_vec();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let aad = format!("codux-e2e-aad-v1|{host_id}|{device_id}");
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Failed to encrypt remote payload.".to_string())?;
    if encrypted.len() < 16 {
        return Err("Invalid encrypted payload.".to_string());
    }
    let (ciphertext, tag) = encrypted.split_at(encrypted.len() - 16);
    Ok(json!({
        "v": 1,
        "alg": "X25519-HKDF-SHA256-AES-256-GCM",
        "nonce": remote_base64_url_encode(&nonce_bytes),
        "ciphertext": remote_base64_url_encode(ciphertext),
        "tag": remote_base64_url_encode(tag),
    }))
}

pub(crate) fn remote_e2e_decrypt(
    payload: &Value,
    key: &[u8; 32],
    host_id: &str,
    device_id: &str,
) -> Result<Vec<u8>, String> {
    if payload.get("v").and_then(Value::as_i64) != Some(1) {
        return Err("Unsupported encrypted payload.".to_string());
    }
    let nonce = remote_base64_url_decode(
        payload
            .get("nonce")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing nonce.".to_string())?,
    )?;
    let mut ciphertext = remote_base64_url_decode(
        payload
            .get("ciphertext")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing ciphertext.".to_string())?,
    )?;
    let tag = remote_base64_url_decode(
        payload
            .get("tag")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing tag.".to_string())?,
    )?;
    ciphertext.extend_from_slice(&tag);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let aad = format!("codux-e2e-aad-v1|{host_id}|{device_id}");
    cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Failed to decrypt remote payload.".to_string())
}

pub(crate) fn remote_base64_url_encode(data: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(data)
}

pub(crate) fn remote_base64_url_decode(value: &str) -> Result<Vec<u8>, String> {
    general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|error| error.to_string())
}
