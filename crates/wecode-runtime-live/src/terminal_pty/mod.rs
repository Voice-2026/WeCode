use crate::ai_runtime::{AIRuntimeBridge, AIRuntimeStateSnapshot, AIRuntimeTerminalBinding};
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(not(windows))]
use std::{
    process::{Command, Stdio},
    sync::OnceLock,
};
use uuid::Uuid;
use wecode_terminal_core::{
    HeadlessTerminalScreen, TerminalDriver as CoreTerminalDriver, TerminalEventSink,
    TerminalLaunchConfig, TerminalScreenSnapshot,
    TerminalSessionHandle as CoreTerminalSessionHandle, terminal_screen_plain_text,
};
pub use wecode_terminal_core::{TerminalEvent, TerminalSessionSnapshot, TerminalViewportState};
use wecode_terminal_pty::{
    LocalPtyCommandMode, LocalPtyProcessHandle, LocalPtySpawnConfig, spawn_local_pty,
};

const INPUT_CAPTURE_LIMIT: usize = 20;
const OUTPUT_CAPTURE_LIMIT: usize = 16 * 1024;
const MIN_HISTORY_BYTES: usize = 128 * 1024;
const MAX_CONFIGURED_HISTORY_BYTES: usize = 8 * 1024 * 1024;
const REMOTE_SCREEN_SCROLLBACK_CAP: usize = 2_000;
const REMOTE_SCREEN_IDLE_SCROLLBACK: usize = 500;
const TERMINAL_VIEWPORT_LEASE_TTL: Duration = Duration::from_secs(20);

mod capture;
mod config;
mod environment;
mod events;
mod manager;
mod osc;
mod platform;
mod session;
#[cfg(test)]
mod tests;
mod watcher;

pub use capture::{TerminalCapturedInput, TerminalInputSnapshot, TerminalOutputSnapshot};
pub use config::{TerminalLaunchContext, TerminalPtyConfig};
pub use environment::terminal_environment;
pub use events::{
    EventSink, ViewportOwnerResolver, terminal_viewport_local_owner, terminal_viewport_remote_owner,
};
pub use manager::{DesktopTerminalSessionHandle, TerminalManager};
pub use platform::default_shell;
pub use session::{TerminalPtySession, TerminalPtySessionHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalIoDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalIoOrigin {
    Local,
    WeChat,
    Pty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalIoEvent {
    pub sequence: u64,
    pub direction: TerminalIoDirection,
    pub origin: TerminalIoOrigin,
    pub bytes: Vec<u8>,
}

fn broadcast_io_event(
    subscribers: &parking_lot::Mutex<Vec<flume::Sender<TerminalIoEvent>>>,
    sequence: &AtomicU64,
    direction: TerminalIoDirection,
    origin: TerminalIoOrigin,
    bytes: &[u8],
) {
    if bytes.is_empty() {
        return;
    }
    let event = TerminalIoEvent {
        sequence: sequence.fetch_add(1, Ordering::SeqCst).saturating_add(1),
        direction,
        origin,
        bytes: bytes.to_vec(),
    };
    let mut subscribers = subscribers.lock();
    subscribers.retain(|subscriber| subscriber.send(event.clone()).is_ok());
}

use capture::*;
use config::*;
use events::*;
use osc::*;
use platform::*;
use session::*;
use watcher::*;
