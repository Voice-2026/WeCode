use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use wecode_protocol::REMOTE_PROTOCOL_VERSION;

mod ai_stats;
mod cmd_app;
mod cmd_config;
mod cmd_device;
mod cmd_pair;
mod cmd_product;
mod cmd_service;
mod cmd_start;
mod cmd_update;
mod config_store;
mod device_store;
mod host;
mod logo;
mod memory;
mod paths;
mod projects;
mod runstate;
mod sessions;
mod smoke;
mod terminals;
mod web_test;
mod worktree;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "wecode",
    bin_name = "wecode",
    version = VERSION,
    about = "WeCode headless host — run your projects' terminals, Git, AI and memory for remote desktops",
    after_help = "Examples:\n  wecode app status --json\n  wecode project list --json\n  wecode session create --project <id> --agent codex --json\n  wecode automation list --json\n  wecode completion zsh"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the version and protocol revision.
    Version,
    /// Control the local WeCode Desktop application.
    App {
        #[command(subcommand)]
        command: AppCommand,
    },
    /// Inspect projects registered in the local WeCode Desktop application.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Create and manage WeCode Desktop worktrees.
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommand,
    },
    /// Inspect AI Agents available to WeCode Desktop.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Inspect models explicitly configured for an Agent.
    Model {
        #[command(subcommand)]
        command: ModelCommand,
    },
    /// Create, resume, drive, inspect and stop Agent sessions.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Create and control ordinary terminals in WeCode Desktop.
    Terminal {
        #[command(subcommand)]
        command: TerminalCommand,
    },
    /// Inspect and control scheduled Agent automations in WeCode Desktop.
    Automation {
        #[command(subcommand)]
        command: AutomationCommand,
    },
    /// Generate shell completion for the public WeCode command tree.
    Completion {
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Interactive setup wizard — writes/updates wecode.toml.
    Config {
        /// Set the device name without prompting.
        #[arg(long)]
        device_name: Option<String>,
        /// Set the relay preset (global, china-tencent, china-aliyun, custom).
        #[arg(long)]
        relay_preset: Option<String>,
        /// Set a custom relay URL. Implies --relay-preset custom.
        #[arg(long)]
        relay_url: Option<String>,
        /// Set the custom relay auth token.
        #[arg(long)]
        relay_authentication: Option<String>,
    },
    /// Install and enable WeCode as a system startup service.
    Install,
    /// Stop and remove the system service.
    Uninstall,
    /// Start the host (idempotent; prints the path if already running).
    Start {
        /// Run detached in the background (used by the service).
        #[arg(long)]
        detach: bool,
    },
    /// Stop the running host.
    Stop,
    /// Show whether the host is running, since when, and how many devices.
    Status,
    /// Print the pairing QR code in the terminal (starts the host if needed).
    Qrcode {
        /// Switch relay before generating the QR.
        #[arg(long)]
        relay_preset: Option<String>,
        /// Switch to a custom relay URL before generating the QR.
        #[arg(long)]
        relay_url: Option<String>,
        /// Set the custom relay auth token before generating the QR.
        #[arg(long)]
        relay_authentication: Option<String>,
    },
    /// Print the pairing ticket for the desktop to paste (starts the host if needed).
    Link {
        /// Switch relay before printing the link.
        #[arg(long)]
        relay_preset: Option<String>,
        /// Switch to a custom relay URL before printing the link.
        #[arg(long)]
        relay_url: Option<String>,
        /// Set the custom relay auth token before printing the link.
        #[arg(long)]
        relay_authentication: Option<String>,
    },
    /// Check for a newer release and update in place.
    Update {
        /// Include beta pre-releases in the update channel.
        #[arg(long)]
        beta: bool,
    },
    /// List paired devices.
    Device,
    /// Remove a paired device by id.
    #[command(name = "device:del")]
    DeviceDel { id: String },
    /// Rename a paired device by id (prompts for the new name).
    #[command(name = "device:rename")]
    DeviceRename { id: String },
    /// Remove every paired device.
    #[command(name = "device:clear")]
    DeviceClear,
    /// Internal smoke tests (pty | transport | serve).
    #[command(name = "smoke", hide = true)]
    Smoke { kind: String },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Self::Bash,
            CompletionShell::Zsh => Self::Zsh,
            CompletionShell::Fish => Self::Fish,
            CompletionShell::PowerShell => Self::PowerShell,
            CompletionShell::Elvish => Self::Elvish,
        }
    }
}

#[derive(Subcommand)]
enum AppCommand {
    /// Show whether the local WeCode Desktop control endpoint is available.
    Status {
        /// Print a stable machine-readable response envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// List projects registered in WeCode Desktop.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum WorktreeCommand {
    /// List a project's worktrees.
    List {
        #[arg(long)]
        project: String,
        #[arg(long)]
        json: bool,
    },
    /// Create a managed worktree and branch.
    Create {
        #[arg(long)]
        project: String,
        #[arg(long)]
        branch: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Merge a managed worktree after reviewing its risk summary.
    Merge {
        #[arg(long)]
        project: String,
        #[arg(long)]
        worktree: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        remove_branch: bool,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
    /// Remove a managed worktree after reviewing its risk summary.
    Remove {
        #[arg(long)]
        project: String,
        #[arg(long)]
        worktree: String,
        #[arg(long)]
        remove_branch: bool,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AgentCommand {
    /// List adapted Agents, installation state and capabilities.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ModelCommand {
    /// List model IDs explicitly configured in WeCode for an Agent.
    List {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// List active and indexed historical Agent sessions.
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create an interactive Agent session in a project or worktree.
    Create {
        #[arg(long)]
        project: String,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        permission_mode: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Resume an indexed historical Agent session in a new terminal.
    Resume {
        #[arg(long)]
        id: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Submit one prompt to a live Agent session.
    Send {
        #[arg(long)]
        id: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the structured state of an Agent session.
    Status {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Stop a live Agent session after reviewing its target summary.
    Stop {
        #[arg(long)]
        id: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TerminalCommand {
    /// List ordinary terminals, optionally scoped to a project or worktree.
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create an ordinary terminal in a project or worktree.
    Create {
        #[arg(long)]
        project: String,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Send exact text to a terminal; append Enter only when requested.
    Send {
        #[arg(long)]
        terminal: String,
        #[arg(long, default_value = "")]
        text: String,
        #[arg(long)]
        enter: bool,
        #[arg(long)]
        json: bool,
    },
    /// Read a bounded plain-text tail from a terminal buffer.
    Snapshot {
        #[arg(long)]
        terminal: String,
        #[arg(long)]
        tail: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Close a terminal after reviewing its target and state.
    Close {
        #[arg(long)]
        terminal: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AutomationCommand {
    /// List automation definitions and their latest run state.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Create an automation; defaults to Claude + Kiro with Claude Opus 4.8.
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long, default_value = "existing")]
        workspace_mode: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        reuse_session: bool,
        #[arg(long, default_value = "kiro_gateway_claude")]
        agent: String,
        #[arg(long, default_value = "claude-opus-4.8")]
        model: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        precheck: Option<String>,
        #[arg(long, default_value_t = 60)]
        precheck_timeout: u64,
        #[arg(long, default_value = "daily:09:00")]
        schedule: String,
        #[arg(long, default_value = "Asia/Shanghai")]
        timezone: String,
        #[arg(long, default_value_t = 43_200)]
        catch_up_grace: i64,
        #[arg(long)]
        json: bool,
    },
    /// Update selected fields of an existing automation.
    Update {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        workspace_mode: Option<String>,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        reuse_session: Option<bool>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        precheck: Option<String>,
        #[arg(long)]
        precheck_timeout: Option<u64>,
        #[arg(long)]
        schedule: Option<String>,
        #[arg(long)]
        timezone: Option<String>,
        #[arg(long)]
        catch_up_grace: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    /// Queue an automation for immediate execution.
    Run {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Pause future scheduled runs without deleting the task.
    Pause {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Resume future scheduled runs.
    Resume {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match wecode_runtime_live::wrapper_helper::handle_args(&args) {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(64);
        }
    }

    let cli = Cli::parse();
    let result = match cli.command {
        None => {
            print_version();
            println!();
            println!("Run `wecode --help` for commands, or `wecode config` to set up.");
            Ok(())
        }
        Some(Command::Version) => {
            print_version();
            Ok(())
        }
        Some(Command::App {
            command: AppCommand::Status { json },
        }) => cmd_app::status(json),
        Some(Command::Project {
            command: ProjectCommand::List { json },
        }) => cmd_product::project_list(json),
        Some(Command::Worktree {
            command: WorktreeCommand::List { project, json },
        }) => cmd_product::worktree_list(project, json),
        Some(Command::Worktree {
            command:
                WorktreeCommand::Create {
                    project,
                    branch,
                    base,
                    title,
                    json,
                },
        }) => cmd_product::worktree_create(project, branch, base, title, json),
        Some(Command::Worktree {
            command:
                WorktreeCommand::Merge {
                    project,
                    worktree,
                    base,
                    remove_branch,
                    confirm,
                    json,
                },
        }) => cmd_product::worktree_mutate(
            wecode_protocol::LOCAL_CONTROL_METHOD_WORKTREE_MERGE,
            project,
            worktree,
            base,
            remove_branch,
            confirm,
            json,
        ),
        Some(Command::Agent {
            command: AgentCommand::List { json },
        }) => cmd_product::agent_list(json),
        Some(Command::Model {
            command: ModelCommand::List { agent, json },
        }) => cmd_product::model_list(agent, json),
        Some(Command::Session {
            command:
                SessionCommand::List {
                    project,
                    worktree,
                    json,
                },
        }) => cmd_product::session_list(project, worktree, json),
        Some(Command::Session {
            command:
                SessionCommand::Create {
                    project,
                    worktree,
                    agent,
                    model,
                    permission_mode,
                    json,
                },
        }) => cmd_product::session_create(project, worktree, agent, model, permission_mode, json),
        Some(Command::Session {
            command: SessionCommand::Resume { id, project, json },
        }) => cmd_product::session_resume(id, project, json),
        Some(Command::Session {
            command: SessionCommand::Send { id, prompt, json },
        }) => cmd_product::session_send(id, prompt, json),
        Some(Command::Session {
            command: SessionCommand::Status { id, json },
        }) => cmd_product::session_status(id, json),
        Some(Command::Session {
            command: SessionCommand::Stop { id, confirm, json },
        }) => cmd_product::session_stop(id, confirm, json),
        Some(Command::Terminal {
            command:
                TerminalCommand::List {
                    project,
                    worktree,
                    json,
                },
        }) => cmd_product::terminal_list(project, worktree, json),
        Some(Command::Terminal {
            command:
                TerminalCommand::Create {
                    project,
                    worktree,
                    command,
                    title,
                    json,
                },
        }) => cmd_product::terminal_create(project, worktree, command, title, json),
        Some(Command::Terminal {
            command:
                TerminalCommand::Send {
                    terminal,
                    text,
                    enter,
                    json,
                },
        }) => cmd_product::terminal_send(terminal, text, enter, json),
        Some(Command::Terminal {
            command:
                TerminalCommand::Snapshot {
                    terminal,
                    tail,
                    json,
                },
        }) => cmd_product::terminal_snapshot(terminal, tail, json),
        Some(Command::Terminal {
            command:
                TerminalCommand::Close {
                    terminal,
                    confirm,
                    json,
                },
        }) => cmd_product::terminal_close(terminal, confirm, json),
        Some(Command::Automation {
            command: AutomationCommand::List { json },
        }) => cmd_product::automation_list(json),
        Some(Command::Automation {
            command:
                AutomationCommand::Create {
                    name,
                    project,
                    worktree,
                    workspace_mode,
                    base,
                    reuse_session,
                    agent,
                    model,
                    prompt,
                    precheck,
                    precheck_timeout,
                    schedule,
                    timezone,
                    catch_up_grace,
                    json,
                },
        }) => cmd_product::automation_create(
            wecode_protocol::LocalControlAutomationCreateParams {
                name,
                project_id: project,
                worktree_id: worktree,
                workspace_mode: Some(workspace_mode),
                base_branch: base,
                reuse_session,
                agent_id: Some(agent),
                model: Some(model),
                prompt,
                precheck_command: precheck,
                precheck_timeout_seconds: Some(precheck_timeout),
                schedule: Some(schedule),
                timezone: Some(timezone),
                catch_up_grace_seconds: Some(catch_up_grace),
            },
            json,
        ),
        Some(Command::Automation {
            command:
                AutomationCommand::Update {
                    id,
                    name,
                    project,
                    worktree,
                    workspace_mode,
                    base,
                    reuse_session,
                    agent,
                    model,
                    prompt,
                    precheck,
                    precheck_timeout,
                    schedule,
                    timezone,
                    catch_up_grace,
                    json,
                },
        }) => cmd_product::automation_update(
            wecode_protocol::LocalControlAutomationUpdateParams {
                automation_id: id,
                name,
                project_id: project,
                worktree_id: worktree,
                workspace_mode,
                base_branch: base,
                reuse_session,
                agent_id: agent,
                model,
                prompt,
                precheck_command: precheck,
                precheck_timeout_seconds: precheck_timeout,
                schedule,
                timezone,
                catch_up_grace_seconds: catch_up_grace,
            },
            json,
        ),
        Some(Command::Automation {
            command: AutomationCommand::Run { id, json },
        }) => cmd_product::automation_run(id, json),
        Some(Command::Automation {
            command: AutomationCommand::Pause { id, json },
        }) => cmd_product::automation_set_enabled(
            wecode_protocol::LOCAL_CONTROL_METHOD_AUTOMATION_PAUSE,
            id,
            json,
        ),
        Some(Command::Automation {
            command: AutomationCommand::Resume { id, json },
        }) => cmd_product::automation_set_enabled(
            wecode_protocol::LOCAL_CONTROL_METHOD_AUTOMATION_RESUME,
            id,
            json,
        ),
        Some(Command::Completion { shell }) => {
            let mut command = Cli::command();
            clap_complete::generate(
                clap_complete::Shell::from(shell),
                &mut command,
                "wecode",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        Some(Command::Worktree {
            command:
                WorktreeCommand::Remove {
                    project,
                    worktree,
                    remove_branch,
                    confirm,
                    json,
                },
        }) => cmd_product::worktree_mutate(
            wecode_protocol::LOCAL_CONTROL_METHOD_WORKTREE_REMOVE,
            project,
            worktree,
            None,
            remove_branch,
            confirm,
            json,
        ),
        Some(Command::Config {
            device_name,
            relay_preset,
            relay_url,
            relay_authentication,
        }) => cmd_config::run(cmd_config::ConfigArgs {
            device_name,
            relay_preset,
            relay_url,
            relay_authentication,
        }),
        Some(Command::Install) => cmd_service::install(),
        Some(Command::Uninstall) => cmd_service::uninstall(),
        Some(Command::Start { detach }) => cmd_start::run(detach),
        Some(Command::Stop) => cmd_service::stop(),
        Some(Command::Status) => cmd_service::status(),
        Some(Command::Qrcode {
            relay_preset,
            relay_url,
            relay_authentication,
        }) => cmd_pair::qrcode(cmd_pair::PairArgs {
            relay_preset,
            relay_url,
            relay_authentication,
        }),
        Some(Command::Link {
            relay_preset,
            relay_url,
            relay_authentication,
        }) => cmd_pair::link(cmd_pair::PairArgs {
            relay_preset,
            relay_url,
            relay_authentication,
        }),
        Some(Command::Update { beta }) => cmd_update::run(VERSION, beta),
        Some(Command::Device) => cmd_device::list(),
        Some(Command::DeviceDel { id }) => cmd_device::del(&id),
        Some(Command::DeviceRename { id }) => cmd_device::rename(&id),
        Some(Command::DeviceClear) => cmd_device::clear(),
        Some(Command::Smoke { kind }) => smoke::run(&kind).map(|output| println!("{output}")),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn print_version() {
    println!("wecode {VERSION}");
    println!("protocol {REMOTE_PROTOCOL_VERSION}");
}

#[cfg(test)]
mod product_cli_contract_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn product_command_tree_and_help_contract_are_valid() {
        Cli::command().debug_assert();
        let command = Cli::command();
        let expected_groups = [
            "app",
            "project",
            "worktree",
            "agent",
            "model",
            "session",
            "terminal",
            "automation",
        ];
        for group in expected_groups {
            assert!(
                command.find_subcommand(group).is_some(),
                "missing product command group: {group}"
            );
        }
        assert!(command.find_subcommand("completion").is_some());
        for legacy in [
            "config",
            "install",
            "uninstall",
            "start",
            "stop",
            "status",
            "qrcode",
            "link",
            "update",
            "device",
            "device:del",
            "device:rename",
            "device:clear",
        ] {
            assert!(
                command.find_subcommand(legacy).is_some(),
                "missing legacy host command: {legacy}"
            );
        }

        let cases: &[&[&str]] = &[
            &["wecode", "app", "status", "--json"],
            &["wecode", "project", "list", "--json"],
            &["wecode", "worktree", "list", "--project", "p", "--json"],
            &[
                "wecode",
                "worktree",
                "create",
                "--project",
                "p",
                "--branch",
                "b",
                "--json",
            ],
            &[
                "wecode",
                "worktree",
                "merge",
                "--project",
                "p",
                "--worktree",
                "w",
                "--confirm",
                "--json",
            ],
            &[
                "wecode",
                "worktree",
                "remove",
                "--project",
                "p",
                "--worktree",
                "w",
                "--confirm",
                "--json",
            ],
            &["wecode", "agent", "list", "--json"],
            &["wecode", "model", "list", "--agent", "codex", "--json"],
            &["wecode", "session", "list", "--json"],
            &[
                "wecode",
                "session",
                "create",
                "--project",
                "p",
                "--agent",
                "codex",
                "--json",
            ],
            &["wecode", "session", "resume", "--id", "s", "--json"],
            &[
                "wecode", "session", "send", "--id", "s", "--prompt", "hello", "--json",
            ],
            &["wecode", "session", "status", "--id", "s", "--json"],
            &[
                "wecode",
                "session",
                "stop",
                "--id",
                "s",
                "--confirm",
                "--json",
            ],
            &["wecode", "terminal", "list", "--json"],
            &["wecode", "terminal", "create", "--project", "p", "--json"],
            &[
                "wecode",
                "terminal",
                "send",
                "--terminal",
                "t",
                "--text",
                "echo",
                "--enter",
                "--json",
            ],
            &[
                "wecode",
                "terminal",
                "snapshot",
                "--terminal",
                "t",
                "--json",
            ],
            &[
                "wecode",
                "terminal",
                "close",
                "--terminal",
                "t",
                "--confirm",
                "--json",
            ],
            &["wecode", "automation", "list", "--json"],
            &["wecode", "automation", "run", "--id", "a", "--json"],
            &["wecode", "automation", "pause", "--id", "a", "--json"],
            &["wecode", "automation", "resume", "--id", "a", "--json"],
            &["wecode", "completion", "zsh"],
        ];
        for args in cases {
            assert!(
                Cli::try_parse_from(*args).is_ok(),
                "product CLI contract failed to parse: {args:?}"
            );
        }
    }
}
