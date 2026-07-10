pub mod anthropic;
pub mod kiro;
pub mod openai;

use serde_json::Value;

/// Unified message format shared by the OpenAI and Anthropic adapters.
#[derive(Debug, Clone, Default)]
pub struct UnifiedMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCallSpec>>,
    pub tool_results: Option<Vec<ToolResult>>,
    pub images: Option<Vec<UnifiedImage>>,
}

#[derive(Debug, Clone)]
pub struct ToolCallSpec {
    pub id: String,
    pub name: String,
    /// Arguments as a JSON value (object) — normalized from string or object.
    pub input: Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct UnifiedImage {
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct UnifiedTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// Thinking (fake reasoning) configuration.
#[derive(Debug, Clone, Copy)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u32>,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            budget_tokens: None,
        }
    }
}
