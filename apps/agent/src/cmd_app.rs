use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    time::Duration,
};
use wecode_protocol::{
    LOCAL_CONTROL_MAX_RESPONSE_BYTES, LOCAL_CONTROL_METHOD_APP_STATUS,
    LOCAL_CONTROL_PROTOCOL_VERSION, LocalControlRequest, LocalControlResponse,
};

#[cfg(unix)]
use std::path::Path;

const EXIT_DESKTOP_OFFLINE: i32 = 3;
const EXIT_UNAUTHORIZED: i32 = 4;
const EXIT_NOT_FOUND: i32 = 5;
const EXIT_CONFLICT: i32 = 6;
const EXIT_CONFIRMATION_REQUIRED: i32 = 7;
const EXIT_PROTOCOL: i32 = 8;
const EXIT_INTERNAL: i32 = 1;
const EXIT_USAGE: i32 = 64;

pub fn status(json_output: bool) -> Result<(), String> {
    match request_status() {
        Ok(response) if response.ok => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string(&response)
                        .map_err(|error| format!("encode app status: {error}"))?
                );
            } else if let Some(data) = response.data {
                println!("WeCode Desktop: online");
                println!(
                    "Version: {}",
                    data.get("version")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                );
                println!("Control protocol: {LOCAL_CONTROL_PROTOCOL_VERSION}");
                let capabilities = data
                    .get("capabilities")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                println!("Capabilities: {capabilities}");
            }
            Ok(())
        }
        Ok(response) => {
            let exit_code = response
                .error
                .as_ref()
                .map(|error| exit_code_for_error(&error.code))
                .unwrap_or(EXIT_INTERNAL);
            exit_with_response(response, json_output, exit_code)
        }
        Err(error) => {
            let response = LocalControlResponse::failure(
                uuid_request_id(),
                "DESKTOP_OFFLINE",
                "WeCode Desktop local control is unavailable",
                json!({ "reason": error }),
            );
            exit_with_response(response, json_output, EXIT_DESKTOP_OFFLINE)
        }
    }
}

#[cfg(unix)]
fn request_status() -> Result<LocalControlResponse, String> {
    request(LOCAL_CONTROL_METHOD_APP_STATUS, json!({}))
}

#[cfg(unix)]
pub(crate) fn request(method: &str, params: Value) -> Result<LocalControlResponse, String> {
    let socket_path = wecode_runtime_live::runtime_paths::local_control_socket_path();
    let token_path = wecode_runtime_live::runtime_paths::local_control_token_path();
    request_at_paths(&socket_path, &token_path, method, params)
}

#[cfg(unix)]
fn request_at_paths(
    socket_path: &Path,
    token_path: &Path,
    method: &str,
    params: Value,
) -> Result<LocalControlResponse, String> {
    use std::os::unix::net::UnixStream;

    let token = fs::read_to_string(&token_path)
        .map_err(|error| format!("read {}: {error}", token_path.display()))?;
    let stream = UnixStream::connect(&socket_path)
        .map_err(|error| format!("connect {}: {error}", socket_path.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("set local control read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("set local control write timeout: {error}"))?;

    let request = LocalControlRequest {
        protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
        request_id: uuid_request_id(),
        auth_token: token.trim().to_string(),
        method: method.to_string(),
        params,
    };
    let mut payload = serde_json::to_vec(&request)
        .map_err(|error| format!("encode local control request: {error}"))?;
    payload.push(b'\n');
    let mut stream = stream;
    stream
        .write_all(&payload)
        .map_err(|error| format!("send local control request: {error}"))?;

    let mut bytes = Vec::new();
    BufReader::new(stream)
        .take((LOCAL_CONTROL_MAX_RESPONSE_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| format!("read local control response: {error}"))?;
    if bytes.len() > LOCAL_CONTROL_MAX_RESPONSE_BYTES {
        return Err("local control response exceeds the size limit".to_string());
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode local control response: {error}"))
}

#[cfg(not(unix))]
fn request_status() -> Result<LocalControlResponse, String> {
    Err("local product control is not supported on this platform yet".to_string())
}

#[cfg(not(unix))]
pub(crate) fn request(_method: &str, _params: Value) -> Result<LocalControlResponse, String> {
    Err("local product control is not supported on this platform yet".to_string())
}

pub(crate) fn handle_response_error(
    response: LocalControlResponse,
    json_output: bool,
) -> Result<(), String> {
    let exit_code = response
        .error
        .as_ref()
        .map(|error| exit_code_for_error(&error.code))
        .unwrap_or(EXIT_INTERNAL);
    exit_with_response(response, json_output, exit_code)
}

fn exit_code_for_error(code: &str) -> i32 {
    match code {
        "INVALID_REQUEST" | "INVALID_PARAMS" | "REQUEST_TOO_LARGE" => EXIT_USAGE,
        "DESKTOP_OFFLINE" => EXIT_DESKTOP_OFFLINE,
        "UNAUTHORIZED" => EXIT_UNAUTHORIZED,
        "PROJECT_NOT_FOUND"
        | "WORKTREE_NOT_FOUND"
        | "SESSION_NOT_FOUND"
        | "SESSION_NOT_ACTIVE"
        | "TERMINAL_NOT_FOUND"
        | "AUTOMATION_NOT_FOUND"
        | "AGENT_NOT_FOUND"
        | "AMBIGUOUS_TARGET" => EXIT_NOT_FOUND,
        "OPERATION_FAILED"
        | "DEFAULT_WORKTREE_PROTECTED"
        | "SESSION_NOT_READY"
        | "TERMINAL_NOT_READY"
        | "TERMINAL_NOT_RUNNING"
        | "AUTOMATION_ACTIVE_RUN"
        | "AUTOMATION_DISPATCH_UNAVAILABLE"
        | "SERVER_BUSY"
        | "RESPONSE_TOO_LARGE" => EXIT_CONFLICT,
        "CONFIRMATION_REQUIRED" => EXIT_CONFIRMATION_REQUIRED,
        "UNSUPPORTED_PROTOCOL"
        | "METHOD_NOT_FOUND"
        | "UNSUPPORTED_CAPABILITY"
        | "AGENT_UNAVAILABLE" => EXIT_PROTOCOL,
        _ => EXIT_INTERNAL,
    }
}

pub(crate) fn offline(error: String, json_output: bool) -> Result<(), String> {
    let response = LocalControlResponse::failure(
        uuid_request_id(),
        "DESKTOP_OFFLINE",
        "WeCode Desktop local control is unavailable",
        json!({ "reason": error }),
    );
    exit_with_response(response, json_output, EXIT_DESKTOP_OFFLINE)
}

fn exit_with_response(
    response: LocalControlResponse,
    json_output: bool,
    exit_code: i32,
) -> Result<(), String> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string(&response)
                .map_err(|error| format!("encode app status error: {error}"))?
        );
    } else if let Some(error) = response.error {
        eprintln!("error: {} ({})", error.message, error.code);
    }
    std::process::exit(exit_code);
}

fn uuid_request_id() -> String {
    let mut bytes = [0_u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        return format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        );
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn serve_status_once(
        socket: &Path,
        expected_token: &'static str,
        instance_id: &'static str,
    ) -> std::thread::JoinHandle<()> {
        let listener = UnixListener::bind(socket).unwrap();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let request: LocalControlRequest = serde_json::from_str(&line).unwrap();
            assert_eq!(request.auth_token, expected_token);
            let response = LocalControlResponse::success(
                request.request_id,
                json!({
                    "product": "WeCode",
                    "version": "9.8.7",
                    "protocolVersion": LOCAL_CONTROL_PROTOCOL_VERSION,
                    "instanceId": instance_id,
                    "pid": 42,
                    "capabilities": ["app.status.v1"],
                }),
            );
            let mut payload = serde_json::to_vec(&response).unwrap();
            payload.push(b'\n');
            reader.get_mut().write_all(&payload).unwrap();
        })
    }

    #[test]
    fn client_sends_authenticated_status_request_and_decodes_response() {
        let root = std::path::PathBuf::from(format!(
            "/tmp/wlc-client-{}-{}",
            std::process::id(),
            &uuid_request_id()[..8]
        ));
        fs::create_dir_all(&root).unwrap();
        let socket = root.join("control.sock");
        let token_path = root.join("control.token");
        fs::write(&token_path, "test-token\n").unwrap();
        let listener = UnixListener::bind(&socket).unwrap();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let request: LocalControlRequest = serde_json::from_str(&line).unwrap();
            assert_eq!(request.auth_token, "test-token");
            assert_eq!(request.method, LOCAL_CONTROL_METHOD_APP_STATUS);
            let response = LocalControlResponse::success(
                request.request_id,
                json!({
                    "product": "WeCode",
                    "version": "9.8.7",
                    "protocolVersion": LOCAL_CONTROL_PROTOCOL_VERSION,
                    "instanceId": "test-instance",
                    "pid": 42,
                    "capabilities": ["app.status.v1"],
                }),
            );
            let mut payload = serde_json::to_vec(&response).unwrap();
            payload.push(b'\n');
            reader.get_mut().write_all(&payload).unwrap();
        });

        let response = request_at_paths(
            &socket,
            &token_path,
            LOCAL_CONTROL_METHOD_APP_STATUS,
            json!({}),
        )
        .unwrap();
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["version"], "9.8.7");
        server.join().unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn product_error_codes_have_stable_exit_categories() {
        assert_eq!(exit_code_for_error("INVALID_PARAMS"), EXIT_USAGE);
        assert_eq!(exit_code_for_error("DESKTOP_OFFLINE"), EXIT_DESKTOP_OFFLINE);
        assert_eq!(exit_code_for_error("UNAUTHORIZED"), EXIT_UNAUTHORIZED);
        assert_eq!(exit_code_for_error("AUTOMATION_NOT_FOUND"), EXIT_NOT_FOUND);
        assert_eq!(exit_code_for_error("SERVER_BUSY"), EXIT_CONFLICT);
        assert_eq!(
            exit_code_for_error("CONFIRMATION_REQUIRED"),
            EXIT_CONFIRMATION_REQUIRED
        );
        assert_eq!(exit_code_for_error("UNSUPPORTED_PROTOCOL"), EXIT_PROTOCOL);
        assert_eq!(exit_code_for_error("INTERNAL_ERROR"), EXIT_INTERNAL);
    }

    #[test]
    fn client_reconnects_after_desktop_restart_and_token_rotation() {
        let root = std::path::PathBuf::from(format!(
            "/tmp/wlc-reconnect-{}-{}",
            std::process::id(),
            &uuid_request_id()[..8]
        ));
        fs::create_dir_all(&root).unwrap();
        let socket = root.join("control.sock");
        let token_path = root.join("control.token");

        fs::write(&token_path, "first-token\n").unwrap();
        let first_server = serve_status_once(&socket, "first-token", "instance-one");
        let first = request_at_paths(
            &socket,
            &token_path,
            LOCAL_CONTROL_METHOD_APP_STATUS,
            json!({}),
        )
        .unwrap();
        assert_eq!(first.data.unwrap()["instanceId"], "instance-one");
        first_server.join().unwrap();

        fs::remove_file(&socket).unwrap();
        fs::write(&token_path, "second-token\n").unwrap();
        let second_server = serve_status_once(&socket, "second-token", "instance-two");
        let second = request_at_paths(
            &socket,
            &token_path,
            LOCAL_CONTROL_METHOD_APP_STATUS,
            json!({}),
        )
        .unwrap();
        assert_eq!(second.data.unwrap()["instanceId"], "instance-two");
        second_server.join().unwrap();

        let _ = fs::remove_dir_all(root);
    }
}
