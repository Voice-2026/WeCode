mod buffer_assembler;
mod headless_screen;
mod output_sequence;
mod remote_pty;
mod runtime_model;
mod sequence_guard;
mod terminal_driver;

pub use buffer_assembler::{TerminalBufferAssembler, TerminalBufferAssemblyResult};
pub use headless_screen::{HeadlessTerminalScreen, TerminalScreenSnapshot};
pub use output_sequence::{
    TerminalOutputSequenceAction, TerminalOutputSequenceResult, TerminalOutputSequencer,
};
pub use remote_pty::{RemotePtyBaselinePageResult, RemotePtySession, RemotePtySnapshot};
pub use runtime_model::{
    RemoteRuntimeModel, RemoteRuntimePlan, RemoteRuntimeProject, RemoteRuntimeStateSnapshot,
    RemoteRuntimeTerminal,
};
pub use sequence_guard::RemoteSequenceGuard;
pub use terminal_driver::{
    TerminalBaselineRequest, TerminalDriver, TerminalEvent, TerminalEventSink,
    TerminalLaunchConfig, TerminalSessionHandle, TerminalSessionSnapshot, TerminalViewportState,
};

pub type TerminalSequence = i64;

#[cfg(test)]
mod tests;
