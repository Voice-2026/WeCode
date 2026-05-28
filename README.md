# Codux GPUI

This is the Rust-native GPUI migration target for Codux. The current milestone
keeps the existing terminal experiment working while replacing the single-window
spike with a Codux-shaped desktop shell.

It intentionally does not use Tauri, WebView, React, or xterm.js. The long-term
goal is to move the Codux desktop UI and Rust runtime into a native GPUI app.

## Stack

- UI: `gpui`
- Terminal widget: `gpui-terminal`
- Terminal core: `alacritty_terminal`
- PTY: `portable-pty`
- Runtime assets: copied from `codux-tauri/src-tauri/runtime-assets`
- Staged Rust runtime source: copied from `codux-tauri/src-tauri/src` into
  `tauri-runtime-src`

## Run

```sh
cargo run
```

The app opens a native GPUI Codux shell and spawns the user's default shell in
the main terminal pane.

## Current Status

- Codux-like GPUI layout is in place: title HUD, sidebar, project panel,
  workspace tabs, terminal pane, inspector panel, and status metrics.
- The left sidebar now drives real workspace views. AI keeps the temporary PTY
  terminal, while Files, Git, Memory, and Settings render GPUI-native panels
  backed by the migrated runtime services instead of only changing nav
  highlight state.
- The PTY terminal remains functional through `gpui-terminal` and
  `portable-pty`.
- The AI workspace can now launch Codex, Claude, Gemini, OpenCode, and Kiro
  from GPUI controls by sending the configured wrapper command into the active
  terminal pane. The launch path refreshes tool permissions and memory
  artifacts before handing control to the terminal.
- The shell now reads real Codux desktop state from Application Support:
  `state.json`, `settings.json`, selected project Git summary, and top-level
  file entries.
- Project rows are clickable inside GPUI and refresh the selected project's Git
  summary and file list without restarting the app.
- Terminal tabs and split panes can now be created inside the native GPUI
  workspace. The active terminal remains focusable and keeps the existing
  Ctrl+= / Ctrl+- font-size shortcuts.
- GPUI startup and the `LoadLayout` action now restore terminal tabs and split
  panes from the selected project's existing `terminalLayouts` entry in
  `state.json`, preserving saved labels and pane titles while spawning fresh
  temporary PTYs for the current GPUI terminal backend.
- Terminal tab selection, tab creation, split creation, and tab close now write
  the GPUI terminal layout back to the selected project's `terminalLayouts`
  state while preserving restored slot and terminal ids when available.
- GPUI terminal launches now start in the selected project's working directory
  and inject the Codux runtime wrapper environment, including wrapper `PATH`,
  project metadata, SSH profile discovery, shell hooks, runtime event/log
  locations, and synced tool-permission settings for the managed AI CLIs.
- Each GPUI terminal tab and split pane now receives Tauri-compatible
  `CODUX_TERMINAL_ID`, `CODUX_SLOT_ID`, `DMUX_SESSION_*`, title, cwd, and stable
  instance-id environment variables so wrapper events can be correlated back to
  the native pane that launched them.
- GPUI terminal launches also generate lightweight Codux memory launch
  artifacts under `runtime-root/memory-workspaces/<project>` and inject
  `DMUX_AI_MEMORY_WORKSPACE_ROOT`, `DMUX_AI_MEMORY_PROMPT_FILE`, and
  `DMUX_AI_MEMORY_INDEX_FILE` so managed AI CLIs can discover indexed project
  memory while the full Tauri memory retrieval pipeline is still being ported.
- A small GPUI runtime service is now wired to the Codux support directory. It
  can reload `state.json` / `settings.json` and safely write the terminal
  scrollback setting back to `settings.json` as the first non-Tauri settings
  mutation path.
- GPUI-native runtime path and file services are now in `src/runtime_paths.rs`
  and `src/files_service.rs`, adapted from the Tauri runtime boundaries without
  `AppHandle` or command macros.
- The inspector can refresh the current project's file list and preview text
  files through the new file service, with root-escape protection and large /
  binary file guards.
- The Files workspace can now browse nested project directories, return to the
  parent directory, and preview files from the selected directory while keeping
  root-escape protection in the GPUI file service.
- The Files workspace can select entries, create auto-named files or folders in
  the current directory, and delete the selected entry through the same
  project-root guard and system Trash / Recycle Bin path used by the Tauri file
  runtime.
- File mutations now also include GPUI-native text write and same-directory
  rename paths backed by the file runtime's root-escape and no-overwrite
  guards; the current UI exposes these through `Save` and `Rename` actions.
- Text files now load into a lightweight GPUI editor buffer in the Files
  workspace. Basic typing, Enter, Tab, Backspace, and Cmd/Ctrl+S update and
  save the buffer through the GPUI file runtime while large/binary files remain
  read-only previews.
- File copy is available from the Files workspace via `Copy`, including
  recursive directory copy, automatic `copy N` conflict names, project-root
  protection, and the same "cannot copy a folder into itself" guard used by the
  Tauri runtime.
- Files can now be revealed in the system file manager or opened with the
  system default application from the GPUI Files workspace while preserving the
  same project-root guard as the Tauri file runtime.
- `src/git_service.rs` now provides the first GPUI-native Git status service:
  branch, upstream, ahead / behind, changed files, local branches, remotes, and
  recent commits. The inspector can refresh this data for the selected project.
- The Git workspace now supports selecting changed files and loading a staged /
  unstaged diff preview through the GPUI-native Git service, including a safe
  untracked-text preview fallback.
- The Git workspace can stage and unstage the selected changed file, then
  refresh the status and diff preview from the GPUI-native Git service.
- The Git workspace now also exposes GPUI-native commit, fetch, pull, and push
  actions. Commit uses the currently staged files and a generated summary while
  remote actions refresh Git and active worktree state after completion.
- The Git workspace can select local branches, check out the selected branch,
  and create an auto-named GPUI branch through the native Git service while
  refreshing the active worktree summary after branch changes.
- The Git workspace can now discard the selected changed file, delete safe
  untracked files or directories inside the repository, and delete a selected
  non-current local branch through GPUI-native Git service calls.
- `src/project_store.rs` can persist project selection back to Codux
  `state.json` while preserving unknown fields, so GPUI selection begins to
  synchronize with the existing Codux runtime state instead of being only local
  UI state.
- `src/project_store.rs` also has the first GPUI project mutations: add/select
  the GPUI project, close the selected project, reorder the selected project up
  or down, rename/update the selected project through the Tauri-shaped project
  update path, prune related worktree/task/layout state on close, and preserve
  unknown `state.json` fields.
- `src/settings_service.rs` reads the real Codux `settings.json` into a broader
  settings summary and performs local JSON-preserving mutations for terminal
  scrollback, theme cycling, and developer HUD toggling.
- Settings now includes a safe AI provider summary from `settings.json`,
  including provider kind, display name, model, base URL, enabled state, memory
  extraction flag, runtime tool count, and Git commit-message provider id
  without loading or serializing API keys.
- GPUI now reads the existing remote relay settings into a Tauri-shaped
  read-only remote status summary, including relay URL, host id, encryption
  readiness, cached paired devices, online counts, and revoked-device filtering.
- GPUI now reads and writes the desktop `sleepMode` setting and holds a native
  sleep-prevention assertion through the same platform boundaries as the Tauri
  runtime (`IOPMAssertion` on macOS, Windows power requests, and Linux
  inhibit helpers).
- The Settings workspace can select an AI provider, toggle its enabled state,
  and set it as the Git commit-message provider while preserving unknown
  settings fields and provider secret fields in `settings.json`.
- `src/ai_history_service.rs` reads the existing `ai-usage.sqlite3` index with
  `rusqlite`, so the project panel now shows real indexed AI session and token
  data instead of placeholder rows.
- `src/ai_history_service.rs` also has the first GPUI AI-history mutation path:
  rename and remove a project session by raw session id or deterministic grouped
  id, updating the existing SQLite index while leaving transcript files alone.
- The GPUI project panel now renders AI sessions as selectable rows, and the
  inspector AI actions operate on the selected session instead of hard-coding
  the newest indexed session.
- Indexed AI sessions can now be restored from GPUI. The session list and
  inspector send the same Codex / Claude / Gemini / OpenCode / Antigravity
  resume commands used by the Tauri UI into the active terminal pane.
- `src/memory_service.rs` reads `memory.sqlite3` and surfaces active/core/
  working/archive memory counts, extraction queue state, project profile
  presence, and recent memory entries in the inspector.
- `src/ssh_service.rs` reads `ssh_profiles.json` and reports saved SSH profiles
  plus wrapper availability without loading or displaying passwords,
  passphrases, or private key paths.
- GPUI can now select a saved SSH profile and send the same `codux-ssh
  <profile-id>` wrapper command into the active terminal pane, keeping credential
  handling inside the shared runtime wrapper instead of exposing secrets to the
  UI.
- `src/terminal_layout_service.rs` reads and writes the selected project's
  `terminalLayouts` entry in `state.json`, allowing the GPUI tab/split model to
  synchronize with the existing Codux layout state.
- GPUI now persists live terminal runtime metadata into
  `gpui-terminal-runtime.json`, including active terminal/slot ids, tab and
  pane identity, project/cwd metadata, running/closed status, and a short closed
  history without serializing command output, environment variables, or secrets.
- The same GPUI terminal runtime file now records a short, sanitized input
  history from both direct terminal typing and native GPUI launch actions, plus
  a bounded PTY output tail and byte counts for each pane. This gives the
  runtime panel a durable session trail while full scrollback replay remains
  future work.
- GPUI startup and terminal layout reload now read the saved output tail back
  into each matching pane and render it as a small restored-tail preview above
  the live PTY, so restored terminals have visible context before fresh output
  arrives.
- `src/worktree_service.rs` reads real project worktree/task state from
  `state.json`, exposes the selected worktree plus active worktree Git status,
  can persist selected worktree changes, and can rescan `git worktree list
  --porcelain` back into `state.json` while preserving unknown fields and
  existing task metadata.
- GPUI now exposes initial worktree actions in the inspector: `SyncWT` rescans
  Git worktrees, `NewWT` creates an auto-named managed worktree under
  `.codux/worktrees`, `RemoveWT` removes the selected non-default worktree while
  keeping its branch, and `RemoveWT+Branch` removes the matching local branch
  after the worktree is removed. `MergeWT` checks out the task base branch in
  the root worktree, merges the selected non-default worktree branch, and then
  refreshes worktree state.
- `src/update_service.rs` reads update settings and resolves the current
  `latest.json` manifest, including the local stable manifest used by current
  development builds.
- `src/runtime_activity_service.rs` adds a GPUI-native read-only runtime
  activity bridge. It surfaces Codux support/runtime directories, runtime and
  live log tails, runtime event counts, runtime-support installation state, and
  currently running AI tool processes in the native shell.
- `src/performance_service.rs` adds the first native GPUI performance summary
  for the current app process. The title HUD and runtime inspector now show
  real CPU and memory labels instead of the previous static GPUI version pill.
- `src/runtime_event_service.rs` reads the shared `runtime-events` directory
  without draining it, decodes `ai-hook` and `opencode-runtime` event envelopes,
  and shows recent tool/kind/project/session activity in the GPUI runtime
  panels. It also folds recent events into a lightweight AI runtime session
  summary with running, needs-input, and completed counts for the native shell.
- Runtime sessions in the GPUI runtime panel are now selectable. Selecting a
  session shows its latest state, project, title, event count, and terminal id,
  and focuses the matching GPUI terminal pane when the runtime terminal id maps
  back to the native tab/split identity.
- GPUI now persists a native AI runtime state snapshot to
  `gpui-ai-runtime-state.json`, derived from the shared runtime event stream.
  The snapshot records session ids, tools, state, project, title, counts, and
  update time while preserving unknown JSON fields and avoiding environment or
  credential data.
- `src/runtime_ingress_service.rs` starts a native GPUI `runtime-events.sock`
  listener when the socket is not already owned by another live runtime. Managed
  AI wrappers can now post hook frames directly into GPUI, which persists them
  into `runtime-events/*.json` for the event/session summary bridge.
- `src/pet_service.rs` reads the existing desktop pet state in the same
  encrypted/JSON formats as the Tauri runtime and shows claimed species, level,
  XP, archived pet count, and installed custom-pet count without mutating pet
  data.
- `src/tool_permissions_service.rs` reads `ai.runtimeTools` from
  `settings.json`, sanitizes it with the same permission/model rules as the
  Tauri runtime, and writes the runtime `tool-permissions.json` consumed by the
  managed AI wrapper scripts.
- The Memory workspace can now select recent memory entries and archive or
  restore them through `memory.sqlite3`, keeping project/user scope checks in
  the Rust service layer.
- GPUI now exposes a Tauri-shaped Memory Manager snapshot with target rows,
  scope overview, project profile, active/history entries, summaries, and
  extraction counts. The Memory workspace can switch manager tabs and delete
  selected entries, summaries, or the selected project's profile through the
  GPUI Rust service.
- `runtime-assets` is present and counted by the app at startup, including i18n
  locale files, wrapper binaries, and shell hooks.
- At startup GPUI now stages `runtime-assets` into the shared
  `codux-dev/runtime-root` runtime directory, preserving nested wrapper and
  shell-hook files before terminal launches use that staged root.
- `tauri-runtime-src` contains the current Tauri Rust runtime source as the
  migration source set.
- The Tauri Rust runtime modules are not yet compiled directly in this project.
  They still need to be split from Tauri command/event/AppHandle boundaries into
  GPUI-callable services.

## Next Runtime Work

- Port full terminal output/history persistence and remaining AI history
  indexing from `codux-tauri/src-tauri/src`.
- Continue replacing Tauri command handlers with direct Rust service APIs. The
  first settings and file-service paths are now live in GPUI.
- Add GPUI event/state adapters for session history, file tree, Git actions,
  settings, update status, and AI provider state.
- Replace static shell panels with live data from those services.

## Quick Checks

- `cargo check`
- `cargo run`
- Terminal typing and command execution
- Long output, for example `yes | head -10000`
- Full-screen apps, for example `top`, `vim`, `less`
- Resize behavior
- CPU and memory while idle

## Known Limitations

`gpui-terminal` is young. Its README currently notes that mouse text selection
and scrollback navigation are not implemented yet. The app shell is also still a
native mock of the Codux UI surface until the Tauri runtime modules are ported.
