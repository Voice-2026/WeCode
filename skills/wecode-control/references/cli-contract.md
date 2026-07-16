# WeCode Product CLI contract

## Response envelope

Every automation call must use `--json`.

Success:

```json
{
  "ok": true,
  "requestId": "...",
  "protocolVersion": "1",
  "data": {}
}
```

Failure:

```json
{
  "ok": false,
  "requestId": "...",
  "protocolVersion": "1",
  "error": {
    "code": "SESSION_NOT_READY",
    "message": "...",
    "details": {}
  }
}
```

Preserve `requestId`, `error.code`, and `error.details` when reporting a failure.

## App integrations

```bash
wecode integration status --json
wecode integration install --agent all|codex|claude|kiro --confirm --json
wecode integration update --confirm --json
wecode integration uninstall --agent all|codex|claude|kiro --confirm --json
```

`status` is read-only and reports the bundled CLI, bundled Skill, canonical managed copy, per-Agent target paths, and whether an update is available. The other commands require `--confirm`; without it they return `CONFIRMATION_REQUIRED`. WeCode installs one canonical Skill copy and links it into supported Agent directories. It refuses to overwrite or remove an unmanaged file or directory.

## Discovery

```bash
wecode app status --json
wecode project list --json
wecode worktree list --project <project-id> --json
wecode agent list --json
wecode model list --agent <agent-id> --json
```

The running Desktop advertises capabilities such as `session.create.v1`. Check them before use.

## Agent sessions

```bash
wecode session list [--project <project-id>] [--worktree <worktree-id>] --json
wecode session create --project <project-id> [--worktree <worktree-id>] --agent <agent-id> [--model <model-id>] [--permission-mode default|fullAccess] --json
wecode session resume --id <history-session-id> [--project <project-id>] --json
wecode session send --id <active-session-id> --prompt <text> --json
wecode session status --id <active-session-id-or-history-id> --json
wecode session stop --id <active-session-id> [--confirm] --json
```

Important session fields:

- `kind`: `active`, `history`, or `runtime`.
- `id`: the identifier accepted by the corresponding operation.
- `terminalId`: the live terminal identifier for an active session.
- `externalSessionId`: the Agent's persistent conversation identifier when known.
- `status`: `starting`, `running`, `waiting_input`, `completed`, `failed`, `stopped`, or `unknown`.
- `canSend`: whether the live TUI has passed startup gates. Also require a non-running status before sending.
- `canResume`: whether an indexed history session has a reliable resume identifier.

Recommended state loop:

```text
create/resume
  -> poll status
  -> canSend + starting/completed/failed
  -> send
  -> running
     -> completed       return result
     -> waiting_input   request user action in Desktop
     -> failed          report and preserve session
```

## Ordinary terminals

```bash
wecode terminal list [--project <project-id>] [--worktree <worktree-id>] --json
wecode terminal create --project <project-id> [--worktree <worktree-id>] [--command <command>] [--title <title>] --json
wecode terminal send --terminal <terminal-id> [--text <text>] [--enter] --json
wecode terminal snapshot --terminal <terminal-id> [--tail <characters>] --json
wecode terminal close --terminal <terminal-id> [--confirm] --json
```

Agent terminals are intentionally excluded from these commands.

## Worktrees

```bash
wecode worktree list --project <project-id> --json
wecode worktree create --project <project-id> --branch <branch> [--base <ref>] [--title <title>] --json
wecode worktree merge --project <project-id> --worktree <worktree-id> [--base <ref>] [--remove-branch] [--confirm] --json
wecode worktree remove --project <project-id> --worktree <worktree-id> [--remove-branch] [--confirm] --json
```

For merge/remove, the first call without `--confirm` returns `CONFIRMATION_REQUIRED` with the reviewable target/risk summary. Present it before retrying with `--confirm`.

## Automations

```bash
wecode automation list --json
wecode automation create --name <name> --project <project-id> --prompt <text> [--worktree <worktree-id>] [--workspace-mode existing|new] [--base <branch>] [--reuse-session] [--agent <agent-id>] [--model <model-id>] [--precheck <command>] [--precheck-timeout <seconds>] [--schedule <spec>] [--timezone <iana-timezone>] [--catch-up-grace <seconds>] --json
wecode automation update --id <automation-id> [--name <name>] [--project <project-id>] [--worktree <worktree-id>] [--workspace-mode existing|new] [--base <branch>] [--reuse-session true|false] [--agent <agent-id>] [--model <model-id>] [--prompt <text>] [--precheck <command>] [--precheck-timeout <seconds>] [--schedule <spec>] [--timezone <iana-timezone>] [--catch-up-grace <seconds>] --json
wecode automation run --id <automation-id> --json
wecode automation pause --id <automation-id> --json
wecode automation resume --id <automation-id> --json
```

Create defaults:

- `agentId`: `kiro_gateway_claude` (`Claude + Kiro`)
- `model`: `claude-opus-4.8`
- `workspaceMode`: `existing`
- `schedule`: `daily:09:00`
- `timezone`: `Asia/Shanghai`
- `precheckTimeoutSeconds`: `60`
- `catchUpGraceSeconds`: `43200`

Supported `Claude + Kiro` model IDs include `claude-haiku-4.5`, `claude-sonnet-4.6`, `claude-opus-4.6`, `claude-opus-4.7`, `claude-opus-4.8`, `deepseek-3.2`, `glm-5`, `minimax-m2.5`, and `qwen3-coder-next`.

`automation update` is partial: omitted values remain unchanged. Changing the Agent or model disables reuse of the previous session unless `--reuse-session true` is explicitly supplied.

## Error handling

- `DESKTOP_OFFLINE`: ask the user to start WeCode Desktop; do not mutate support files.
- `UNSUPPORTED_PROTOCOL`: the CLI and Desktop versions are incompatible; report both versions.
- `SERVER_BUSY`: retry with bounded backoff.
- `SESSION_NOT_READY` / `TERMINAL_NOT_READY`: poll and retry after readiness.
- `SESSION_NOT_FOUND`: list sessions; after restart, resume a reliable history session.
- `CONFIRMATION_REQUIRED`: present `error.details` and wait for explicit confirmation.
- `INTEGRATION_CONFLICT`: an unmanaged CLI or Skill already occupies the target; preserve it and ask the user how to proceed.
- `UNAUTHORIZED`: the requested model or permission mode exceeds WeCode configuration; do not bypass it.
- `UNSUPPORTED_CAPABILITY`: use only the advertised surface; do not emulate it through internal files.
- `AMBIGUOUS_TARGET`: list candidates and ask the user to choose.
- `AUTOMATION_ACTIVE_RUN`: do not queue a duplicate run.
- `OPERATION_FAILED` / `INTERNAL_ERROR`: report the request ID and details; do not repeat destructive operations automatically.
