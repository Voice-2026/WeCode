use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use wecode_protocol::{
    LOCAL_CONTROL_CAPABILITY_AGENT_LIST, LOCAL_CONTROL_CAPABILITY_APP_STATUS,
    LOCAL_CONTROL_CAPABILITY_AUTOMATION_CREATE, LOCAL_CONTROL_CAPABILITY_AUTOMATION_LIST,
    LOCAL_CONTROL_CAPABILITY_AUTOMATION_PAUSE, LOCAL_CONTROL_CAPABILITY_AUTOMATION_RESUME,
    LOCAL_CONTROL_CAPABILITY_AUTOMATION_RUN, LOCAL_CONTROL_CAPABILITY_AUTOMATION_UPDATE,
    LOCAL_CONTROL_CAPABILITY_MODEL_LIST, LOCAL_CONTROL_CAPABILITY_PROJECT_LIST,
    LOCAL_CONTROL_CAPABILITY_SESSION_CREATE, LOCAL_CONTROL_CAPABILITY_SESSION_LIST,
    LOCAL_CONTROL_CAPABILITY_SESSION_RESUME, LOCAL_CONTROL_CAPABILITY_SESSION_SEND,
    LOCAL_CONTROL_CAPABILITY_SESSION_STATUS, LOCAL_CONTROL_CAPABILITY_SESSION_STOP,
    LOCAL_CONTROL_CAPABILITY_TERMINAL_CLOSE, LOCAL_CONTROL_CAPABILITY_TERMINAL_CREATE,
    LOCAL_CONTROL_CAPABILITY_TERMINAL_LIST, LOCAL_CONTROL_CAPABILITY_TERMINAL_SEND,
    LOCAL_CONTROL_CAPABILITY_TERMINAL_SNAPSHOT, LOCAL_CONTROL_CAPABILITY_WORKTREE_CREATE,
    LOCAL_CONTROL_CAPABILITY_WORKTREE_LIST, LOCAL_CONTROL_CAPABILITY_WORKTREE_MERGE,
    LOCAL_CONTROL_CAPABILITY_WORKTREE_REMOVE, LOCAL_CONTROL_MAX_REQUEST_BYTES,
    LOCAL_CONTROL_MAX_REQUEST_ID_CHARS, LOCAL_CONTROL_MAX_RESPONSE_BYTES,
    LOCAL_CONTROL_METHOD_AGENT_LIST, LOCAL_CONTROL_METHOD_APP_STATUS,
    LOCAL_CONTROL_METHOD_AUTOMATION_CREATE, LOCAL_CONTROL_METHOD_AUTOMATION_LIST,
    LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE, LOCAL_CONTROL_METHOD_AUTOMATION_RESUME,
    LOCAL_CONTROL_METHOD_AUTOMATION_RUN, LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE,
    LOCAL_CONTROL_METHOD_MODEL_LIST, LOCAL_CONTROL_METHOD_PROJECT_LIST,
    LOCAL_CONTROL_METHOD_SESSION_CREATE, LOCAL_CONTROL_METHOD_SESSION_LIST,
    LOCAL_CONTROL_METHOD_SESSION_RESUME, LOCAL_CONTROL_METHOD_SESSION_SEND,
    LOCAL_CONTROL_METHOD_SESSION_STATUS, LOCAL_CONTROL_METHOD_SESSION_STOP,
    LOCAL_CONTROL_METHOD_TERMINAL_CLOSE, LOCAL_CONTROL_METHOD_TERMINAL_CREATE,
    LOCAL_CONTROL_METHOD_TERMINAL_LIST, LOCAL_CONTROL_METHOD_TERMINAL_SEND,
    LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT, LOCAL_CONTROL_METHOD_WORKTREE_CREATE,
    LOCAL_CONTROL_METHOD_WORKTREE_LIST, LOCAL_CONTROL_METHOD_WORKTREE_MERGE,
    LOCAL_CONTROL_METHOD_WORKTREE_REMOVE, LOCAL_CONTROL_PROTOCOL_VERSION, LocalControlAppStatus,
    LocalControlAutomationCreateParams, LocalControlAutomationTargetParams,
    LocalControlAutomationUpdateParams, LocalControlModelListParams, LocalControlProjectParams,
    LocalControlRequest, LocalControlResponse, LocalControlSessionCreateParams,
    LocalControlSessionListParams, LocalControlSessionResumeParams, LocalControlSessionSendParams,
    LocalControlSessionTargetParams, LocalControlTerminalCreateParams,
    LocalControlTerminalListParams, LocalControlTerminalSendParams,
    LocalControlTerminalSnapshotParams, LocalControlTerminalTargetParams,
    LocalControlWorktreeCreateParams, LocalControlWorktreeMutationParams,
};

use crate::{
    project_store::ProjectSummary,
    runtime_state::RuntimeService,
    worktree::{WorktreeCreateRequest, WorktreeMergeRequest, WorktreeRemoveRequest},
};

const LOCAL_CONTROL_MAX_CONCURRENT_REQUESTS: usize = 8;

pub struct LocalControlServer {
    socket_path: PathBuf,
    token_path: PathBuf,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalControlServer {
    pub fn start(
        product_version: impl Into<String>,
        runtime_service: RuntimeService,
    ) -> Result<Self, String> {
        Self::start_at_paths(
            crate::runtime_paths::local_control_socket_path(),
            crate::runtime_paths::local_control_token_path(),
            product_version.into(),
            runtime_service,
        )
    }

    #[cfg(unix)]
    fn start_at_paths(
        socket_path: PathBuf,
        token_path: PathBuf,
        product_version: String,
        runtime_service: RuntimeService,
    ) -> Result<Self, String> {
        use rand::RngCore;
        use std::os::unix::{fs::PermissionsExt, net::UnixListener};

        let parent = socket_path
            .parent()
            .ok_or_else(|| "local control socket has no parent directory".to_string())?;
        // macOS rejects sockaddr_un paths at roughly 104 bytes. Report the
        // resolved endpoint before touching token state so custom temp roots
        // fail safely and explainably.
        if socket_path.as_os_str().as_encoded_bytes().len() >= 100 {
            return Err(format!(
                "local control socket path is too long: {}",
                socket_path.display()
            ));
        }
        fs::create_dir_all(parent)
            .map_err(|error| format!("create local control directory: {error}"))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("secure local control directory: {error}"))?;

        if socket_path.exists() {
            if std::os::unix::net::UnixStream::connect(&socket_path).is_ok() {
                return Err("local control endpoint is already active".to_string());
            }
            fs::remove_file(&socket_path)
                .map_err(|error| format!("remove stale local control socket: {error}"))?;
        }

        let mut token_bytes = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut token_bytes);
        let token = token_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        fs::write(&token_path, format!("{token}\n"))
            .map_err(|error| format!("write local control token: {error}"))?;
        fs::set_permissions(&token_path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("secure local control token: {error}"))?;

        let listener = UnixListener::bind(&socket_path)
            .map_err(|error| format!("bind local control socket: {error}"))?;
        fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("secure local control socket: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("configure local control socket: {error}"))?;

        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let active_requests = Arc::new(AtomicUsize::new(0));
        let instance_id = uuid::Uuid::new_v4().to_string();
        let worker_token = token;
        let thread = thread::Builder::new()
            .name("wecode-local-control".to_string())
            .spawn(move || {
                while !worker_stopping.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let Some(slot) = RequestSlot::try_acquire(Arc::clone(&active_requests))
                            else {
                                write_response(
                                    stream,
                                    LocalControlResponse::failure(
                                        "unknown",
                                        "SERVER_BUSY",
                                        "local control has reached its concurrent request limit",
                                        json!({ "retryAfterMs": 100 }),
                                    ),
                                );
                                continue;
                            };
                            let token = worker_token.clone();
                            let version = product_version.clone();
                            let instance = instance_id.clone();
                            let service = runtime_service.clone();
                            if thread::Builder::new()
                                .name("wecode-local-control-request".to_string())
                                .spawn(move || {
                                    let _slot = slot;
                                    handle_stream(stream, &token, &version, &instance, &service);
                                })
                                .is_err()
                            {
                                crate::runtime_trace::runtime_trace(
                                    "local-control",
                                    "request worker could not be started",
                                );
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(25));
                        }
                        Err(error) => {
                            crate::runtime_trace::runtime_trace(
                                "local-control",
                                &format!("accept failed: {error}"),
                            );
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })
            .map_err(|error| format!("start local control thread: {error}"))?;

        Ok(Self {
            socket_path,
            token_path,
            stopping,
            thread: Some(thread),
        })
    }

    #[cfg(not(unix))]
    fn start_at_paths(
        _socket_path: PathBuf,
        _token_path: PathBuf,
        _product_version: String,
        _runtime_service: RuntimeService,
    ) -> Result<Self, String> {
        Err("local product control is not supported on this platform yet".to_string())
    }
}

struct RequestSlot {
    active: Arc<AtomicUsize>,
}

impl RequestSlot {
    fn try_acquire(active: Arc<AtomicUsize>) -> Option<Self> {
        active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < LOCAL_CONTROL_MAX_CONCURRENT_REQUESTS).then_some(count + 1)
            })
            .ok()
            .map(|_| Self { active })
    }
}

impl Drop for RequestSlot {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(unix)]
fn handle_stream(
    stream: std::os::unix::net::UnixStream,
    expected_token: &str,
    product_version: &str,
    instance_id: &str,
    runtime_service: &RuntimeService,
) {
    use std::io::{BufRead, BufReader, Read};

    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let response = match reader
        .by_ref()
        .take((LOCAL_CONTROL_MAX_REQUEST_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)
    {
        Ok(_) if bytes.len() > LOCAL_CONTROL_MAX_REQUEST_BYTES => LocalControlResponse::failure(
            "unknown",
            "REQUEST_TOO_LARGE",
            "local control request exceeds the size limit",
            Value::Null,
        ),
        Ok(0) => LocalControlResponse::failure(
            "unknown",
            "INVALID_REQUEST",
            "local control request is empty",
            Value::Null,
        ),
        Ok(_) => match serde_json::from_slice::<LocalControlRequest>(&bytes) {
            Ok(request) => dispatch_with_audit(
                request,
                expected_token,
                product_version,
                instance_id,
                runtime_service,
            ),
            Err(error) => LocalControlResponse::failure(
                "unknown",
                "INVALID_REQUEST",
                "local control request is not valid JSON",
                json!({ "reason": error.to_string() }),
            ),
        },
        Err(error) => LocalControlResponse::failure(
            "unknown",
            "INVALID_REQUEST",
            "failed to read local control request",
            json!({ "reason": error.to_string() }),
        ),
    };

    write_response(reader.into_inner(), response);
}

#[cfg(unix)]
fn write_response(mut stream: std::os::unix::net::UnixStream, response: LocalControlResponse) {
    use std::io::Write;

    let mut response = response;
    let mut payload = serde_json::to_vec(&response).unwrap_or_default();
    if payload.len() > LOCAL_CONTROL_MAX_RESPONSE_BYTES {
        response = LocalControlResponse::failure(
            response.request_id,
            "RESPONSE_TOO_LARGE",
            "local control response exceeds the size limit",
            Value::Null,
        );
        payload = serde_json::to_vec(&response).unwrap_or_default();
    }
    payload.push(b'\n');
    let _ = stream.write_all(&payload);
    let _ = stream.flush();
}

fn dispatch_with_audit(
    request: LocalControlRequest,
    expected_token: &str,
    product_version: &str,
    instance_id: &str,
    runtime_service: &RuntimeService,
) -> LocalControlResponse {
    let started_at = Instant::now();
    let method = audit_text(&request.method, 80);
    let request_id = audit_text(&request.request_id, 64);
    let response = dispatch(
        request,
        expected_token,
        product_version,
        instance_id,
        runtime_service,
    );
    let outcome = response
        .error
        .as_ref()
        .map(|error| error.code.as_str())
        .unwrap_or("OK");
    crate::runtime_trace::runtime_trace_elapsed(
        "local-control",
        "request",
        started_at,
        &format!("method={method} request_id={request_id} outcome={outcome}"),
    );
    response
}

fn audit_text(value: &str, max_chars: usize) -> String {
    let text = value
        .chars()
        .take(max_chars)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if text.is_empty() {
        "unknown".to_string()
    } else {
        text
    }
}

fn dispatch(
    request: LocalControlRequest,
    expected_token: &str,
    product_version: &str,
    instance_id: &str,
    runtime_service: &RuntimeService,
) -> LocalControlResponse {
    if !constant_time_eq(request.auth_token.as_bytes(), expected_token.as_bytes()) {
        return LocalControlResponse::failure(
            request.request_id,
            "UNAUTHORIZED",
            "local control authentication failed",
            Value::Null,
        );
    }
    if request.protocol_version != LOCAL_CONTROL_PROTOCOL_VERSION {
        return LocalControlResponse::failure(
            request.request_id,
            "UNSUPPORTED_PROTOCOL",
            "local control protocol version is not supported",
            json!({
                "clientVersion": request.protocol_version,
                "serverVersion": LOCAL_CONTROL_PROTOCOL_VERSION,
            }),
        );
    }
    if request.request_id.is_empty()
        || request.request_id.chars().count() > LOCAL_CONTROL_MAX_REQUEST_ID_CHARS
        || request.request_id.chars().any(char::is_control)
    {
        return LocalControlResponse::failure(
            "unknown",
            "INVALID_REQUEST",
            "local control request ID is invalid",
            Value::Null,
        );
    }
    if request.method.is_empty()
        || request.method.len() > 128
        || !request
            .method
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.')
    {
        return LocalControlResponse::failure(
            request.request_id,
            "INVALID_REQUEST",
            "local control method name is invalid",
            Value::Null,
        );
    }
    match request.method.as_str() {
        LOCAL_CONTROL_METHOD_APP_STATUS => {
            let status = LocalControlAppStatus {
                product: "WeCode".to_string(),
                version: product_version.to_string(),
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                instance_id: instance_id.to_string(),
                pid: std::process::id(),
                capabilities: vec![
                    LOCAL_CONTROL_CAPABILITY_APP_STATUS.to_string(),
                    LOCAL_CONTROL_CAPABILITY_PROJECT_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_WORKTREE_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_WORKTREE_CREATE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_WORKTREE_MERGE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_WORKTREE_REMOVE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AGENT_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_MODEL_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_CREATE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_RESUME.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_SEND.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_STATUS.to_string(),
                    LOCAL_CONTROL_CAPABILITY_SESSION_STOP.to_string(),
                    LOCAL_CONTROL_CAPABILITY_TERMINAL_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_TERMINAL_CREATE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_TERMINAL_SEND.to_string(),
                    LOCAL_CONTROL_CAPABILITY_TERMINAL_SNAPSHOT.to_string(),
                    LOCAL_CONTROL_CAPABILITY_TERMINAL_CLOSE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_LIST.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_CREATE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_UPDATE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_RUN.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_PAUSE.to_string(),
                    LOCAL_CONTROL_CAPABILITY_AUTOMATION_RESUME.to_string(),
                ],
            };
            LocalControlResponse::success(
                request.request_id,
                serde_json::to_value(status).unwrap_or(Value::Null),
            )
        }
        LOCAL_CONTROL_METHOD_PROJECT_LIST => success_value(
            request.request_id,
            runtime_service.project_list(),
            "encode project list",
        ),
        LOCAL_CONTROL_METHOD_WORKTREE_LIST => {
            let params = match decode_params::<LocalControlProjectParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            let project = match resolve_project(runtime_service, &params.project_id) {
                Ok(project) => project,
                Err(response) => return response.with_request_id(request.request_id),
            };
            success_value(
                request.request_id,
                runtime_service.worktree_snapshot(project.id, project.path),
                "encode worktree list",
            )
        }
        LOCAL_CONTROL_METHOD_WORKTREE_CREATE => {
            let params = match decode_params::<LocalControlWorktreeCreateParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            let project = match resolve_project(runtime_service, &params.project_id) {
                Ok(project) => project,
                Err(response) => return response.with_request_id(request.request_id),
            };
            match runtime_service.create_worktree_from_request(WorktreeCreateRequest {
                project_id: project.id,
                project_path: project.path,
                base_branch: params.base_branch,
                branch_name: params.branch_name,
                task_title: params.task_title,
            }) {
                Ok(snapshot) => {
                    success_value(request.request_id, snapshot, "encode worktree create")
                }
                Err(error) => operation_failure(request.request_id, error),
            }
        }
        LOCAL_CONTROL_METHOD_WORKTREE_MERGE | LOCAL_CONTROL_METHOD_WORKTREE_REMOVE => {
            dispatch_worktree_mutation(request, runtime_service)
        }
        LOCAL_CONTROL_METHOD_AGENT_LIST => LocalControlResponse::success(
            request.request_id,
            crate::product_control::agent_list(runtime_service),
        ),
        LOCAL_CONTROL_METHOD_MODEL_LIST => {
            let params = match decode_params::<LocalControlModelListParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::model_list(runtime_service, &params.agent_id),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_LIST => {
            let params = match decode_params::<LocalControlSessionListParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_list(
                    runtime_service,
                    params.project_id.as_deref(),
                    params.worktree_id.as_deref(),
                ),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_CREATE => {
            let params = match decode_params::<LocalControlSessionCreateParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_create(
                    runtime_service,
                    &params.project_id,
                    params.worktree_id.as_deref(),
                    &params.agent_id,
                    params.model.as_deref(),
                    params.permission_mode.as_deref(),
                ),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_RESUME => {
            let params = match decode_params::<LocalControlSessionResumeParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_resume(
                    runtime_service,
                    &params.session_id,
                    params.project_id.as_deref(),
                ),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_SEND => {
            let params = match decode_params::<LocalControlSessionSendParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_send(
                    runtime_service,
                    &params.session_id,
                    &params.prompt,
                ),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_STATUS => {
            let params = match decode_params::<LocalControlSessionTargetParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_status(runtime_service, &params.session_id),
            )
        }
        LOCAL_CONTROL_METHOD_SESSION_STOP => {
            let params = match decode_params::<LocalControlSessionTargetParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::session_stop(
                    runtime_service,
                    &params.session_id,
                    params.confirmed,
                ),
            )
        }
        LOCAL_CONTROL_METHOD_TERMINAL_LIST => {
            let params = match decode_params::<LocalControlTerminalListParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::terminal_list(
                    runtime_service,
                    params.project_id.as_deref(),
                    params.worktree_id.as_deref(),
                ),
            )
        }
        LOCAL_CONTROL_METHOD_TERMINAL_CREATE => {
            let params = match decode_params::<LocalControlTerminalCreateParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::terminal_create(
                    runtime_service,
                    &params.project_id,
                    params.worktree_id.as_deref(),
                    params.command.as_deref(),
                    params.title.as_deref(),
                ),
            )
        }
        LOCAL_CONTROL_METHOD_TERMINAL_SEND => {
            let params = match decode_params::<LocalControlTerminalSendParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::terminal_send(
                    runtime_service,
                    &params.terminal_id,
                    &params.text,
                    params.enter,
                ),
            )
        }
        LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT => {
            let params = match decode_params::<LocalControlTerminalSnapshotParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::terminal_snapshot(
                    runtime_service,
                    &params.terminal_id,
                    params.tail,
                ),
            )
        }
        LOCAL_CONTROL_METHOD_TERMINAL_CLOSE => {
            let params = match decode_params::<LocalControlTerminalTargetParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::terminal_close(
                    runtime_service,
                    &params.terminal_id,
                    params.confirmed,
                ),
            )
        }
        LOCAL_CONTROL_METHOD_AUTOMATION_LIST => LocalControlResponse::success(
            request.request_id,
            crate::product_control::automation_list(runtime_service),
        ),
        LOCAL_CONTROL_METHOD_AUTOMATION_CREATE => {
            let params = match decode_params::<LocalControlAutomationCreateParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::automation_create(runtime_service, &params),
            )
        }
        LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE => {
            let params = match decode_params::<LocalControlAutomationUpdateParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::automation_update(runtime_service, &params),
            )
        }
        LOCAL_CONTROL_METHOD_AUTOMATION_RUN => {
            let params = match decode_params::<LocalControlAutomationTargetParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::automation_run(runtime_service, &params.automation_id),
            )
        }
        LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE | LOCAL_CONTROL_METHOD_AUTOMATION_RESUME => {
            let enabled = request.method == LOCAL_CONTROL_METHOD_AUTOMATION_RESUME;
            let params = match decode_params::<LocalControlAutomationTargetParams>(&request) {
                Ok(params) => params,
                Err(response) => return response,
            };
            product_result(
                request.request_id,
                crate::product_control::automation_set_enabled(
                    runtime_service,
                    &params.automation_id,
                    enabled,
                ),
            )
        }
        _ => LocalControlResponse::failure(
            request.request_id,
            "METHOD_NOT_FOUND",
            "local control method is not available",
            json!({ "method": request.method }),
        ),
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let width = left.len().max(right.len());
    for index in 0..width {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

fn decode_params<T: serde::de::DeserializeOwned>(
    request: &LocalControlRequest,
) -> Result<T, LocalControlResponse> {
    serde_json::from_value(request.params.clone()).map_err(|error| {
        LocalControlResponse::failure(
            request.request_id.clone(),
            "INVALID_PARAMS",
            "local control method parameters are invalid",
            json!({ "reason": error.to_string() }),
        )
    })
}

fn resolve_project(
    runtime_service: &RuntimeService,
    project_id: &str,
) -> Result<ProjectSummary, LocalControlResponse> {
    runtime_service
        .project_list()
        .projects
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            LocalControlResponse::failure(
                "unknown",
                "PROJECT_NOT_FOUND",
                "the requested project is not registered in WeCode Desktop",
                json!({ "projectId": project_id }),
            )
        })
}

fn dispatch_worktree_mutation(
    request: LocalControlRequest,
    runtime_service: &RuntimeService,
) -> LocalControlResponse {
    let params = match decode_params::<LocalControlWorktreeMutationParams>(&request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let project = match resolve_project(runtime_service, &params.project_id) {
        Ok(project) => project,
        Err(response) => return response.with_request_id(request.request_id),
    };
    let snapshot = runtime_service.worktree_snapshot(project.id.clone(), project.path.clone());
    let Some(worktree) = snapshot
        .worktrees
        .iter()
        .find(|worktree| worktree.id == params.worktree_id)
    else {
        return LocalControlResponse::failure(
            request.request_id,
            "WORKTREE_NOT_FOUND",
            "the requested worktree does not belong to the project",
            json!({ "projectId": project.id, "worktreeId": params.worktree_id }),
        );
    };
    if worktree.is_default {
        return LocalControlResponse::failure(
            request.request_id,
            "DEFAULT_WORKTREE_PROTECTED",
            "the default worktree cannot be merged or removed",
            json!({ "projectId": project.id, "worktreeId": params.worktree_id }),
        );
    }
    let preview = json!({
        "operation": request.method,
        "projectId": project.id,
        "projectName": project.name,
        "worktreeId": worktree.id,
        "worktreeName": worktree.name,
        "branch": worktree.branch,
        "path": worktree.path,
        "baseBranch": params.base_branch,
        "removeBranch": params.remove_branch,
        "gitSummary": worktree.git_summary,
    });
    if !params.confirmed {
        return LocalControlResponse::failure(
            request.request_id,
            "CONFIRMATION_REQUIRED",
            "review the worktree risk summary and retry with --confirm",
            preview,
        );
    }

    let result = if request.method == LOCAL_CONTROL_METHOD_WORKTREE_MERGE {
        runtime_service.merge_worktree_from_request(WorktreeMergeRequest {
            project_id: project.id,
            project_path: project.path,
            worktree_path: worktree.path.clone(),
            base_branch: params.base_branch,
            remove_branch: Some(params.remove_branch),
        })
    } else {
        runtime_service.remove_worktree_from_request(WorktreeRemoveRequest {
            project_id: project.id,
            project_path: project.path,
            worktree_path: worktree.path.clone(),
            remove_branch: params.remove_branch,
        })
    };
    match result {
        Ok(snapshot) => success_value(request.request_id, snapshot, "encode worktree mutation"),
        Err(error) => operation_failure(request.request_id, error),
    }
}

fn success_value<T: serde::Serialize>(
    request_id: String,
    value: T,
    context: &str,
) -> LocalControlResponse {
    match serde_json::to_value(value) {
        Ok(value) => LocalControlResponse::success(request_id, value),
        Err(error) => LocalControlResponse::failure(
            request_id,
            "INTERNAL_ERROR",
            context,
            json!({ "reason": error.to_string() }),
        ),
    }
}

fn operation_failure(request_id: String, error: String) -> LocalControlResponse {
    LocalControlResponse::failure(
        request_id,
        "OPERATION_FAILED",
        "WeCode Desktop could not complete the requested operation",
        json!({ "reason": error }),
    )
}

fn product_result(
    request_id: String,
    result: Result<Value, crate::product_control::ProductControlError>,
) -> LocalControlResponse {
    match result {
        Ok(value) => LocalControlResponse::success(request_id, value),
        Err(error) => {
            LocalControlResponse::failure(request_id, error.code, error.message, error.details)
        }
    }
}

trait ResponseRequestId {
    fn with_request_id(self, request_id: String) -> Self;
}

impl ResponseRequestId for LocalControlResponse {
    fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = request_id;
        self
    }
}

impl Drop for LocalControlServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        #[cfg(unix)]
        let _ = std::os::unix::net::UnixStream::connect(&self.socket_path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.token_path);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::Path;

    fn request(socket: &Path, request: &LocalControlRequest) -> LocalControlResponse {
        let mut stream = UnixStream::connect(socket).unwrap();
        let mut payload = serde_json::to_vec(request).unwrap();
        payload.push(b'\n');
        stream.write_all(&payload).unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    #[test]
    fn request_slots_are_bounded_and_released() {
        let active = Arc::new(AtomicUsize::new(0));
        let mut slots = (0..LOCAL_CONTROL_MAX_CONCURRENT_REQUESTS)
            .map(|_| RequestSlot::try_acquire(Arc::clone(&active)).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            active.load(Ordering::Acquire),
            LOCAL_CONTROL_MAX_CONCURRENT_REQUESTS
        );
        assert!(RequestSlot::try_acquire(Arc::clone(&active)).is_none());

        slots.pop();
        assert_eq!(
            active.load(Ordering::Acquire),
            LOCAL_CONTROL_MAX_CONCURRENT_REQUESTS - 1
        );
        assert!(RequestSlot::try_acquire(Arc::clone(&active)).is_some());
    }

    #[test]
    fn protocol_request_validation_and_audit_text_are_safe() {
        let root = PathBuf::from(format!("/tmp/wlc-protocol-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let service = RuntimeService::new(root.clone());

        let incompatible = dispatch(
            LocalControlRequest {
                protocol_version: "999".to_string(),
                request_id: "protocol".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_APP_STATUS.to_string(),
                params: json!({}),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        let error = incompatible.error.unwrap();
        assert_eq!(error.code, "UNSUPPORTED_PROTOCOL");
        assert_eq!(error.details["clientVersion"], "999");
        assert_eq!(
            error.details["serverVersion"],
            LOCAL_CONTROL_PROTOCOL_VERSION
        );

        let invalid_request_id = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "unsafe\nrequest".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_APP_STATUS.to_string(),
                params: json!({}),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(invalid_request_id.request_id, "unknown");
        assert_eq!(invalid_request_id.error.unwrap().code, "INVALID_REQUEST");

        let invalid_method = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "method".to_string(),
                auth_token: "token".to_string(),
                method: "session.send\nsecret".to_string(),
                params: json!({}),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(invalid_method.error.unwrap().code, "INVALID_REQUEST");
        assert_eq!(
            audit_text("session.send\nsecret", 80),
            "session.send_secret"
        );
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"different"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn oversized_wire_request_is_rejected_without_parsing_contents() {
        let root = PathBuf::from(format!("/tmp/wlc-oversized-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let service = RuntimeService::new(root.clone());
        let (mut client, server) = UnixStream::pair().unwrap();
        let handler = std::thread::spawn(move || {
            handle_stream(server, "token", "9.8.7", "instance", &service);
        });
        let mut payload = vec![b'x'; LOCAL_CONTROL_MAX_REQUEST_BYTES + 1];
        payload.push(b'\n');
        client.write_all(&payload).unwrap();

        let mut line = String::new();
        BufReader::new(client).read_line(&mut line).unwrap();
        let response: LocalControlResponse = serde_json::from_str(&line).unwrap();
        assert_eq!(response.request_id, "unknown");
        assert_eq!(response.error.unwrap().code, "REQUEST_TOO_LARGE");
        handler.join().unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn app_status_requires_token_and_returns_capabilities() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let socket = root.join("control.sock");
        let token_path = root.join("control.token");
        let support_dir = root.join("support");
        fs::create_dir_all(&support_dir).unwrap();
        let server = LocalControlServer::start_at_paths(
            socket.clone(),
            token_path.clone(),
            "9.8.7".to_string(),
            RuntimeService::new(support_dir),
        )
        .unwrap();
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&token_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let token = fs::read_to_string(&token_path).unwrap();

        let unauthorized = request(
            &socket,
            &LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "unauthorized".to_string(),
                auth_token: "wrong".to_string(),
                method: LOCAL_CONTROL_METHOD_APP_STATUS.to_string(),
                params: json!({}),
            },
        );
        assert_eq!(unauthorized.error.unwrap().code, "UNAUTHORIZED");

        let response = request(
            &socket,
            &LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "status".to_string(),
                auth_token: token.trim().to_string(),
                method: LOCAL_CONTROL_METHOD_APP_STATUS.to_string(),
                params: json!({}),
            },
        );
        assert!(response.ok);
        let status: LocalControlAppStatus = serde_json::from_value(response.data.unwrap()).unwrap();
        assert_eq!(status.version, "9.8.7");
        assert!(
            status
                .capabilities
                .contains(&LOCAL_CONTROL_CAPABILITY_APP_STATUS.to_string())
        );
        assert!(
            status
                .capabilities
                .contains(&LOCAL_CONTROL_CAPABILITY_WORKTREE_CREATE.to_string())
        );
        assert!(
            status
                .capabilities
                .contains(&LOCAL_CONTROL_CAPABILITY_AUTOMATION_CREATE.to_string())
        );
        assert!(
            status
                .capabilities
                .contains(&LOCAL_CONTROL_CAPABILITY_AUTOMATION_UPDATE.to_string())
        );
        drop(server);
        assert!(!socket.exists());
    }

    #[test]
    fn project_list_comes_from_desktop_state_and_unknown_projects_are_rejected() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-projects-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let support_dir = root.join("support");
        let project_path = root.join("project");
        fs::create_dir_all(&support_dir).unwrap();
        fs::create_dir_all(&project_path).unwrap();
        fs::write(
            support_dir.join("state.json"),
            serde_json::to_vec_pretty(&json!({
                "projects": [{
                    "id": "project-a",
                    "name": "Project A",
                    "path": project_path,
                }],
                "selectedProjectId": "project-a",
            }))
            .unwrap(),
        )
        .unwrap();
        let service = RuntimeService::new(support_dir);

        let listed = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "projects".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_PROJECT_LIST.to_string(),
                params: json!({}),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(listed.ok);
        assert_eq!(listed.data.unwrap()["projects"][0]["id"], "project-a");

        let missing = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "missing".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_WORKTREE_LIST.to_string(),
                params: json!({ "projectId": "unknown" }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(missing.request_id, "missing");
        assert_eq!(missing.error.unwrap().code, "PROJECT_NOT_FOUND");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn worktree_remove_requires_confirmation_before_mutating() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-confirm-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let support_dir = root.join("support");
        let project_path = root.join("project");
        fs::create_dir_all(&support_dir).unwrap();
        create_repo_with_commit(&project_path);
        fs::write(
            support_dir.join("state.json"),
            serde_json::to_vec_pretty(&json!({
                "projects": [{
                    "id": "project-a",
                    "name": "Project A",
                    "path": project_path,
                }],
                "selectedProjectId": "project-a",
            }))
            .unwrap(),
        )
        .unwrap();
        let service = RuntimeService::new(support_dir);
        let created = service
            .create_worktree_from_request(WorktreeCreateRequest {
                project_id: "project-a".to_string(),
                project_path: project_path.display().to_string(),
                base_branch: None,
                branch_name: "feature/confirmation".to_string(),
                task_title: Some("Confirmation".to_string()),
            })
            .unwrap();
        let worktree = created
            .worktrees
            .iter()
            .find(|worktree| !worktree.is_default)
            .unwrap();
        let worktree_id = worktree.id.clone();
        let worktree_path = PathBuf::from(&worktree.path);

        let preview = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "preview".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_WORKTREE_REMOVE.to_string(),
                params: json!({
                    "projectId": "project-a",
                    "worktreeId": worktree_id,
                    "confirmed": false,
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        let preview_error = preview.error.unwrap();
        assert_eq!(preview_error.code, "CONFIRMATION_REQUIRED");
        assert_eq!(preview_error.details["branch"], "feature/confirmation");
        assert!(worktree_path.exists());

        let removed = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "remove".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_WORKTREE_REMOVE.to_string(),
                params: json!({
                    "projectId": "project-a",
                    "worktreeId": worktree_id,
                    "confirmed": true,
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(removed.ok, "{removed:?}");
        assert!(!worktree_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_stop_requires_confirmation_and_uses_shared_terminal_manager() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-session-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let support_dir = root.join("support");
        let project_path = root.join("project");
        fs::create_dir_all(&support_dir).unwrap();
        fs::create_dir_all(&project_path).unwrap();
        fs::write(
            support_dir.join("state.json"),
            serde_json::to_vec_pretty(&json!({
                "projects": [{
                    "id": "project-a",
                    "name": "Project A",
                    "path": project_path,
                }],
                "selectedProjectId": "project-a",
            }))
            .unwrap(),
        )
        .unwrap();
        let service = RuntimeService::new(support_dir.clone());
        let terminal_id = service
            .terminal_manager()
            .create(
                crate::terminal_pty::TerminalPtyConfig {
                    cwd: Some(project_path.display().to_string()),
                    command: Some(
                        "read line; printf 'received:%s\\n' \"$line\"; sleep 30".to_string(),
                    ),
                    project_id: Some("project-a".to_string()),
                    project_name: Some("Project A".to_string()),
                    worktree_id: Some("project-a".to_string()),
                    title: Some("Codex".to_string()),
                    tool: Some("codex".to_string()),
                    support_dir: Some(support_dir),
                    runtime_root: Some(crate::runtime_bridge::staged_runtime_root_path()),
                    ..Default::default()
                },
                |_| {},
            )
            .unwrap();

        let sent_too_early = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "send".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_SESSION_SEND.to_string(),
                params: json!({ "sessionId": terminal_id, "prompt": "hello-control" }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(sent_too_early.error.unwrap().code, "SESSION_NOT_READY");

        let preview = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "stop-preview".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_SESSION_STOP.to_string(),
                params: json!({ "sessionId": terminal_id, "confirmed": false }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        let preview_error = preview.error.unwrap();
        assert_eq!(preview_error.code, "CONFIRMATION_REQUIRED");
        assert_eq!(preview_error.details["kind"], "active");
        assert!(
            service
                .terminal_manager()
                .list()
                .iter()
                .any(|terminal| terminal.id == terminal_id)
        );

        let stopped = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "stop".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_SESSION_STOP.to_string(),
                params: json!({ "sessionId": terminal_id, "confirmed": true }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(stopped.ok, "{stopped:?}");
        assert!(
            service
                .terminal_manager()
                .list()
                .iter()
                .all(|terminal| terminal.id != terminal_id)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ordinary_terminal_create_send_snapshot_and_close_share_the_desktop_manager() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-terminal-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let support_dir = root.join("support");
        let project_path = root.join("project");
        fs::create_dir_all(&support_dir).unwrap();
        fs::create_dir_all(&project_path).unwrap();
        fs::write(
            support_dir.join("state.json"),
            serde_json::to_vec_pretty(&json!({
                "projects": [{
                    "id": "project-a",
                    "name": "Project A",
                    "path": project_path,
                }],
                "selectedProjectId": "project-a",
            }))
            .unwrap(),
        )
        .unwrap();
        let service = RuntimeService::new(support_dir);

        let created = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "terminal-create".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_TERMINAL_CREATE.to_string(),
                params: json!({
                    "projectId": "project-a",
                    "title": "CLI Test",
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(created.ok, "{created:?}");
        let terminal_id = created.data.unwrap()["terminalId"]
            .as_str()
            .unwrap()
            .to_string();
        let mut ready = false;
        for _ in 0..250 {
            if service
                .terminal_manager()
                .snapshot(&terminal_id)
                .is_ok_and(|text| text.trim().chars().count() >= 2)
            {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(ready, "interactive shell did not become ready");

        let send_text = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "terminal-send-text".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_TERMINAL_SEND.to_string(),
                params: json!({
                    "terminalId": terminal_id,
                    "text": "printf '\\122\\105\\123\\125\\114\\124\\137\\117\\113\\n'",
                    "enter": false
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(send_text.ok, "{send_text:?}");
        let send_enter = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "terminal-send-enter".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_TERMINAL_SEND.to_string(),
                params: json!({ "terminalId": terminal_id, "text": "", "enter": true }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(send_enter.ok, "{send_enter:?}");

        let mut snapshot = None;
        let mut last_snapshot = None;
        for _ in 0..50 {
            let response = dispatch(
                LocalControlRequest {
                    protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                    request_id: "terminal-snapshot".to_string(),
                    auth_token: "token".to_string(),
                    method: LOCAL_CONTROL_METHOD_TERMINAL_SNAPSHOT.to_string(),
                    params: json!({ "terminalId": terminal_id, "tail": 2048 }),
                },
                "token",
                "9.8.7",
                "instance",
                &service,
            );
            if response
                .data
                .as_ref()
                .and_then(|data| data["text"].as_str())
                .is_some_and(|text| text.contains("RESULT_OK"))
            {
                snapshot = response.data;
                break;
            }
            last_snapshot = response.data;
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            snapshot.is_some(),
            "terminal output was not captured: {last_snapshot:?}"
        );

        let preview = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "terminal-close-preview".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_TERMINAL_CLOSE.to_string(),
                params: json!({ "terminalId": terminal_id, "confirmed": false }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        let preview_error = preview.error.unwrap();
        assert_eq!(preview_error.code, "CONFIRMATION_REQUIRED");
        assert_eq!(preview_error.details["kind"], "terminal");

        let closed = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "terminal-close".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_TERMINAL_CLOSE.to_string(),
                params: json!({ "terminalId": terminal_id, "confirmed": true }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(closed.ok, "{closed:?}");
        assert!(service.terminal_manager().list().is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn automation_commands_use_the_domain_service_and_dispatch_real_run_plans() {
        let root = PathBuf::from(format!(
            "/tmp/wlc-automation-{}-{}",
            std::process::id(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        let support_dir = root.join("support");
        let project_path = root.join("project");
        fs::create_dir_all(&support_dir).unwrap();
        fs::create_dir_all(&project_path).unwrap();
        fs::write(
            support_dir.join("state.json"),
            serde_json::to_vec_pretty(&json!({
                "projects": [{
                    "id": "project-a",
                    "name": "Project A",
                    "path": project_path,
                }],
                "selectedProjectId": "project-a",
            }))
            .unwrap(),
        )
        .unwrap();
        let service = RuntimeService::new(support_dir.clone());
        let automation_service =
            crate::automation::AutomationService::for_support_dir(&support_dir);
        let definition = automation_service
            .create(
                crate::automation::AutomationCreateRequest {
                    name: "CLI automation".to_string(),
                    project_id: "project-a".to_string(),
                    project_name: "Project A".to_string(),
                    workspace_id: "project-a".to_string(),
                    workspace_name: "Project A".to_string(),
                    workspace_path: project_path.display().to_string(),
                    workspace_mode: crate::automation::AutomationWorkspaceMode::Existing,
                    project_path: project_path.display().to_string(),
                    base_branch: None,
                    reuse_session: false,
                    host_device_id: None,
                    agent: crate::automation::AutomationAgent::Codex,
                    model: None,
                    prompt: "secret prompt not exposed by list".to_string(),
                    precheck_command: None,
                    precheck_timeout_seconds: 60,
                    schedule_spec: "cron:* * * * *".to_string(),
                    timezone: "UTC".to_string(),
                    catch_up_grace_seconds: crate::automation::DEFAULT_CATCH_UP_GRACE_SECONDS,
                },
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let (dispatch_tx, dispatch_rx) = flume::unbounded();
        service.set_automation_dispatch_sender(dispatch_tx);

        let created = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-create".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_CREATE.to_string(),
                params: json!({
                    "name": "Created by CLI",
                    "projectId": "project-a",
                    "prompt": "Review the project"
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(created.ok, "{created:?}");
        let created_data = created.data.unwrap();
        let created_id = created_data["automationId"].as_str().unwrap().to_string();
        assert_eq!(created_data["agentId"], "kiro_gateway_claude");
        assert_eq!(created_data["model"], "claude-opus-4.8");

        let updated = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-update".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_UPDATE.to_string(),
                params: json!({
                    "automationId": created_id,
                    "name": "Updated by CLI",
                    "model": "claude-sonnet-4.6"
                }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(updated.ok, "{updated:?}");
        let updated_data = updated.data.unwrap();
        assert_eq!(updated_data["name"], "Updated by CLI");
        assert_eq!(updated_data["model"], "claude-sonnet-4.6");

        let listed = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-list".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_LIST.to_string(),
                params: json!({}),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(listed.ok, "{listed:?}");
        let listed_data = listed.data.unwrap();
        assert_eq!(listed_data["automations"][0]["id"], definition.id);
        assert!(listed_data["automations"][0].get("prompt").is_none());

        let paused = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-pause".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE.to_string(),
                params: json!({ "automationId": definition.id }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(paused.data.unwrap()["enabled"], false);
        let resumed = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-resume".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_RESUME.to_string(),
                params: json!({ "automationId": definition.id }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(resumed.data.unwrap()["enabled"], true);

        let started = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-run".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_RUN.to_string(),
                params: json!({ "automationId": definition.id }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert!(started.ok, "{started:?}");
        let started_data = started.data.unwrap();
        let plan = dispatch_rx.try_recv().unwrap();
        assert_eq!(plan.run_id, started_data["runId"]);
        assert_eq!(plan.automation_id, definition.id);

        let overlapping = dispatch(
            LocalControlRequest {
                protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_string(),
                request_id: "automation-overlap".to_string(),
                auth_token: "token".to_string(),
                method: LOCAL_CONTROL_METHOD_AUTOMATION_RUN.to_string(),
                params: json!({ "automationId": definition.id }),
            },
            "token",
            "9.8.7",
            "instance",
            &service,
        );
        assert_eq!(overlapping.error.unwrap().code, "AUTOMATION_ACTIVE_RUN");
        let _ = fs::remove_dir_all(root);
    }

    fn create_repo_with_commit(repo: &Path) {
        fs::create_dir_all(repo).unwrap();
        let git = git2::Repository::init(repo).unwrap();
        let mut config = git.config().unwrap();
        config.set_str("user.email", "wecode@example.test").unwrap();
        config.set_str("user.name", "WeCode").unwrap();
        fs::write(repo.join("README.md"), "hello\n").unwrap();
        let mut index = git.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("WeCode", "wecode@example.test").unwrap();
        git.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
    }
}
