//! FFI surface for the shared E2E crypto. The mobile device calls these so it
//! runs the exact same key-derivation/encrypt/decrypt code as the desktop host
//! (codux-remote-crypto), instead of a hand-maintained second implementation.

use crate::common::{c_to_string, clear_last_error, set_last_error, string_to_c};
use codux_remote_crypto::{
    generate_keypair, remote_base64_url_decode, remote_base64_url_encode, remote_e2e_decrypt,
    remote_e2e_encrypt, remote_e2e_symmetric_key,
};
use serde_json::{Value, json};
use std::ffi::c_char;
use std::ptr;

fn required(value: *const c_char, label: &str) -> Option<String> {
    match c_to_string(value) {
        Some(value) => Some(value),
        None => {
            set_last_error(format!("missing {label}"));
            None
        }
    }
}

/// Generate a fresh device keypair. Returns JSON `{"privateKey","publicKey"}`
/// (base64url), or null with `codux_protocol_last_error` set.
#[unsafe(no_mangle)]
pub extern "C" fn codux_e2e_new_device_keypair() -> *mut c_char {
    clear_last_error();
    let (private_key, public_key) = generate_keypair();
    string_to_c(json!({ "privateKey": private_key, "publicKey": public_key }).to_string())
}

/// Encrypt `plaintext_b64` (base64url of the raw plaintext bytes) for the peer.
/// Returns the JSON payload `{v,alg,nonce,ciphertext,tag}`, or null on error.
#[unsafe(no_mangle)]
pub extern "C" fn codux_e2e_encrypt(
    device_private_key: *const c_char,
    host_public_key: *const c_char,
    host_id: *const c_char,
    device_id: *const c_char,
    plaintext_b64: *const c_char,
) -> *mut c_char {
    clear_last_error();
    let (Some(private_key), Some(public_key), Some(host_id), Some(device_id), Some(plaintext_b64)) = (
        required(device_private_key, "device private key"),
        required(host_public_key, "host public key"),
        required(host_id, "host id"),
        required(device_id, "device id"),
        required(plaintext_b64, "plaintext"),
    ) else {
        return ptr::null_mut();
    };
    let key = match remote_e2e_symmetric_key(&private_key, &public_key, &host_id, &device_id) {
        Ok(key) => key,
        Err(error) => {
            set_last_error(error);
            return ptr::null_mut();
        }
    };
    let plaintext = match remote_base64_url_decode(&plaintext_b64) {
        Ok(plaintext) => plaintext,
        Err(error) => {
            set_last_error(error);
            return ptr::null_mut();
        }
    };
    match remote_e2e_encrypt(&plaintext, &key, &host_id, &device_id) {
        Ok(payload) => string_to_c(payload.to_string()),
        Err(error) => {
            set_last_error(error);
            ptr::null_mut()
        }
    }
}

/// Decrypt `payload_json` (`{v,alg,nonce,ciphertext,tag}`). Returns the
/// recovered plaintext as base64url, or null on error.
#[unsafe(no_mangle)]
pub extern "C" fn codux_e2e_decrypt(
    device_private_key: *const c_char,
    host_public_key: *const c_char,
    host_id: *const c_char,
    device_id: *const c_char,
    payload_json: *const c_char,
) -> *mut c_char {
    clear_last_error();
    let (Some(private_key), Some(public_key), Some(host_id), Some(device_id), Some(payload_json)) = (
        required(device_private_key, "device private key"),
        required(host_public_key, "host public key"),
        required(host_id, "host id"),
        required(device_id, "device id"),
        required(payload_json, "payload"),
    ) else {
        return ptr::null_mut();
    };
    let payload = match serde_json::from_str::<Value>(&payload_json) {
        Ok(payload) => payload,
        Err(error) => {
            set_last_error(error.to_string());
            return ptr::null_mut();
        }
    };
    let key = match remote_e2e_symmetric_key(&private_key, &public_key, &host_id, &device_id) {
        Ok(key) => key,
        Err(error) => {
            set_last_error(error);
            return ptr::null_mut();
        }
    };
    match remote_e2e_decrypt(&payload, &key, &host_id, &device_id) {
        Ok(plaintext) => string_to_c(remote_base64_url_encode(&plaintext)),
        Err(error) => {
            set_last_error(error);
            ptr::null_mut()
        }
    }
}
