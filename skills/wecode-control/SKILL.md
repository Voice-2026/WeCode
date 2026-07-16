---
name: wecode-control
description: Control a local WeCode Desktop instance through the stable WeCode Product CLI. Use when an Agent needs to inspect WeCode capabilities, resolve projects or worktrees, create or resume AI sessions, submit prompts, monitor execution and approval states, control ordinary terminals, or create, update, run, pause, and resume scheduled automations.
---

# WeCode Control

Use `wecode` as the only product-control boundary. Do not read or mutate WeCode's socket, token, state database, PTY, rollout, or application-support files directly.

## Establish the control surface

1. Resolve `wecode` from `PATH`. If it is unavailable on macOS, ask the user to open WeCode Desktop's **Settings > Integrations** and install the bundled CLI. Do not substitute the repository's debug binary unless the user explicitly asks for local development testing.
2. Run `wecode integration status --json` when checking whether this Skill is current or installed for another Agent.
3. Run `wecode app status --json` before every product-control workflow.
4. Require `ok=true` and inspect `data.capabilities`. Do not call a capability the running Desktop instance does not advertise.
5. Use `--json` for every command. Parse the response envelope instead of terminal prose.

Installing, updating, or removing an integration writes to an external Agent's Skill directory. Show the target paths and obtain explicit user confirmation before calling `integration install`, `integration update`, or `integration uninstall` with `--confirm`. Never replace an unmanaged `wecode-control` installation.

Read [references/cli-contract.md](references/cli-contract.md) when exact commands, fields, error codes, or destructive-operation rules are needed.

## Resolve targets before acting

- Run `wecode project list --json`; use returned IDs instead of guessing names.
- For worktree-scoped work, run `wecode worktree list --project <project-id> --json` and select its returned ID.
- Run `wecode agent list --json`; require `installed=true` and the relevant capability.
- Run `wecode model list --agent <agent-id> --json` before passing `--model`. Only use models explicitly exposed by WeCode.
- Never request `fullAccess` unless `permissionMode=fullAccess` is already configured for that Agent and the user explicitly authorized that mode.

If multiple targets match the user's wording, show the candidates and ask which one to use. Never choose a project, worktree, session, or automation by position alone.

## Drive an Agent session

### Create or resume

- Create with `wecode session create --project <project-id> --agent <agent-id> --json`. Add a worktree, model, or permission mode only after resolving it.
- Resume only a history item with `canResume=true`, using `wecode session resume --id <session-id> --json`.
- Preserve the returned active `id`/`terminalId`; use that active ID for status, send, and stop.

### Wait until input is safe

Poll `wecode session status --id <active-id> --json` with bounded retries. Submit a prompt only when:

- `canSend=true`; and
- `status` is `starting`, `completed`, or `failed`.

Do not send while `status=running`; the Agent may place text in its composer without submitting a new turn. Do not use ordinary `terminal send` against an Agent session.

Treat `SESSION_NOT_READY` as retryable: poll status and retry the same prompt once readiness is true. Treat `SESSION_NOT_FOUND` after a Desktop restart as a signal to list indexed history and resume a `canResume=true` session, not as permission to create a duplicate.

### Monitor the turn

After `session send`, poll status until one of these outcomes:

- `running`: continue polling.
- `waiting_input`: stop. Tell the user the Agent is waiting for an approval or answer in WeCode Desktop. Never approve, deny, or synthesize input on the user's behalf.
- `completed`: return the latest assistant preview and identifiers.
- `failed`: report the interruption/failure and preserve the session for an explicit retry.
- `stopped`: report that the live session ended.
- `starting`: continue polling while the runtime binding settles.
- `unknown`: stop after a short retry window and report the raw JSON envelope.

Use a modest polling interval and a finite deadline. On timeout, return the latest structured status; do not kill the session automatically.

### Stop safely

First run `session status` and show the target summary. Call `wecode session stop --id <active-id> --confirm --json` only when the user explicitly requested or confirmed stopping that session.

## Control ordinary terminals

Ordinary terminals and Agent sessions are separate surfaces.

- Create/list/send/snapshot terminals only with `wecode terminal ... --json`.
- Append Enter only when `--enter` is intended.
- Keep snapshot tails bounded and treat them as potentially sensitive command output.
- Review the terminal identity and state before `terminal close --confirm`.
- Never use terminal commands to bypass Agent-session approval or permission boundaries.

## Control worktrees and automations

- Creating a worktree is allowed only when the user's request authorizes that branch/worktree creation.
- Before worktree merge or removal, run the command without `--confirm`, present the returned risk summary, and wait for explicit confirmation. Then repeat with `--confirm`.
- List automations before updating, running, pausing, or resuming one. Resolve the exact automation ID.
- Create an automation only when the user authorized creating a persistent scheduled task. Resolve the project and optional worktree first.
- When creating an automation, omit `--agent` and `--model` to use the product defaults: `kiro_gateway_claude` and `claude-opus-4.8` (`Claude + Kiro / Opus 4.8`).
- Pass a raw schedule spec such as `daily:10:00`, `weekly:1,2,3,4,5@10:00`, or `cron:0 10 * * 1-5`. Default timezone is `Asia/Shanghai`.
- Use `automation update` for partial updates; omitted fields remain unchanged. Changing Agent or model disables session reuse unless `--reuse-session true` is explicitly requested.
- Do not delete automations: deletion is not exposed by the current Product CLI.

## Return evidence

Report the exact target IDs, Agent/model, final structured status, and any remaining user action. Do not claim success from command acceptance alone; a submitted turn is complete only when status becomes `completed`, or explicitly waiting when it becomes `waiting_input`.
