use crate::ai_runtime::{
    probe::opencode::probe_opencode_runtime,
    tool_driver::{AIRuntimeMemoryInjectionDriver, AIRuntimeToolDriver, AIRuntimeToolHookDriver},
};

pub const DRIVER: AIRuntimeToolDriver = AIRuntimeToolDriver {
    id: "mimo",
    aliases: &["mimo"],
    wrapper_bins: &["mimo"],
    hook: AIRuntimeToolHookDriver::OpenCodePlugin,
    probe: Some(probe_opencode_runtime),
    memory_injection: AIRuntimeMemoryInjectionDriver::None,
};
