#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::Path,
    process::Command,
};
use wecode_protocol::{LOCAL_CONTROL_PROTOCOL_VERSION, LocalControlRequest, LocalControlResponse};

const TOKEN: &str = "contract-token";

#[test]
fn completion_generation_is_noninteractive() {
    let output = Command::new(env!("CARGO_BIN_EXE_wecode-agent"))
        .args(["completion", "zsh"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("_wecode"));
    assert!(stdout.contains("automation"));
}

#[test]
fn every_product_command_emits_one_json_envelope_for_success_and_failure() {
    let root = Path::new("/tmp").join(format!(
        "wlc-json-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let home = root.join("home");
    let temp = root.join("tmp");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&temp).unwrap();
    let runtime = fs::canonicalize(&temp).unwrap().join("wecode-dev");
    fs::create_dir_all(&runtime).unwrap();
    let socket = runtime.join("local-control.sock");
    let token = runtime.join("local-control.token");
    fs::write(&token, format!("{TOKEN}\n")).unwrap();

    let cases: &[(&str, &[&str])] = &[
        ("app.status", &["app", "status", "--json"]),
        ("project.list", &["project", "list", "--json"]),
        (
            "worktree.list",
            &["worktree", "list", "--project", "p", "--json"],
        ),
        (
            "worktree.create",
            &[
                "worktree",
                "create",
                "--project",
                "p",
                "--branch",
                "b",
                "--json",
            ],
        ),
        (
            "worktree.merge",
            &[
                "worktree",
                "merge",
                "--project",
                "p",
                "--worktree",
                "w",
                "--confirm",
                "--json",
            ],
        ),
        (
            "worktree.remove",
            &[
                "worktree",
                "remove",
                "--project",
                "p",
                "--worktree",
                "w",
                "--confirm",
                "--json",
            ],
        ),
        ("agent.list", &["agent", "list", "--json"]),
        (
            "model.list",
            &["model", "list", "--agent", "codex", "--json"],
        ),
        ("session.list", &["session", "list", "--json"]),
        (
            "session.create",
            &[
                "session",
                "create",
                "--project",
                "p",
                "--agent",
                "codex",
                "--json",
            ],
        ),
        (
            "session.resume",
            &["session", "resume", "--id", "s", "--json"],
        ),
        (
            "session.send",
            &[
                "session", "send", "--id", "s", "--prompt", "hello", "--json",
            ],
        ),
        (
            "session.status",
            &["session", "status", "--id", "s", "--json"],
        ),
        (
            "session.stop",
            &["session", "stop", "--id", "s", "--confirm", "--json"],
        ),
        ("terminal.list", &["terminal", "list", "--json"]),
        (
            "terminal.create",
            &["terminal", "create", "--project", "p", "--json"],
        ),
        (
            "terminal.send",
            &[
                "terminal",
                "send",
                "--terminal",
                "t",
                "--text",
                "echo",
                "--enter",
                "--json",
            ],
        ),
        (
            "terminal.snapshot",
            &["terminal", "snapshot", "--terminal", "t", "--json"],
        ),
        (
            "terminal.close",
            &[
                "terminal",
                "close",
                "--terminal",
                "t",
                "--confirm",
                "--json",
            ],
        ),
        ("automation.list", &["automation", "list", "--json"]),
        (
            "automation.create",
            &[
                "automation",
                "create",
                "--name",
                "Daily review",
                "--project",
                "p",
                "--prompt",
                "Review changes",
                "--json",
            ],
        ),
        (
            "automation.update",
            &[
                "automation",
                "update",
                "--id",
                "a",
                "--model",
                "claude-sonnet-4.6",
                "--json",
            ],
        ),
        (
            "automation.run",
            &["automation", "run", "--id", "a", "--json"],
        ),
        (
            "automation.pause",
            &["automation", "pause", "--id", "a", "--json"],
        ),
        (
            "automation.resume",
            &["automation", "resume", "--id", "a", "--json"],
        ),
    ];

    for (method, args) in cases {
        run_case(&socket, &token, &home, &temp, method, args, true);
        run_case(&socket, &token, &home, &temp, method, args, false);
    }

    let _ = fs::remove_dir_all(root);
}

fn run_case(
    socket: &Path,
    token: &Path,
    home: &Path,
    temp: &Path,
    expected_method: &str,
    args: &[&str],
    success: bool,
) {
    let _ = fs::remove_file(socket);
    fs::write(token, format!("{TOKEN}\n")).unwrap();
    let listener = UnixListener::bind(socket).unwrap();
    let expected_method = expected_method.to_string();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let request: LocalControlRequest = serde_json::from_str(&line).unwrap();
        assert_eq!(request.protocol_version, LOCAL_CONTROL_PROTOCOL_VERSION);
        assert_eq!(request.auth_token, TOKEN);
        assert_eq!(request.method, expected_method);
        let response = if success {
            LocalControlResponse::success(request.request_id, serde_json::json!({}))
        } else {
            LocalControlResponse::failure(
                request.request_id,
                "PROJECT_NOT_FOUND",
                "contract failure",
                serde_json::json!({ "projectId": "missing" }),
            )
        };
        let mut payload = serde_json::to_vec(&response).unwrap();
        payload.push(b'\n');
        reader.get_mut().write_all(&payload).unwrap();
    });

    let output = Command::new(env!("CARGO_BIN_EXE_wecode-agent"))
        .args(args)
        .env("HOME", home)
        .env("TMPDIR", temp)
        .output()
        .unwrap();
    server.join().unwrap();

    assert_eq!(
        output.status.code(),
        Some(if success { 0 } else { 5 }),
        "unexpected exit for {args:?}: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "JSON mode wrote stderr for {args:?}"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "expected one JSON line for {args:?}"
    );
    let response: LocalControlResponse = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response.ok, success);
    assert_eq!(response.protocol_version, LOCAL_CONTROL_PROTOCOL_VERSION);
    assert_eq!(response.data.is_some(), success);
    assert_eq!(response.error.is_some(), !success);
}
