## ADDED Requirements

### Requirement: Floating Overlay Chip on Terminal Panes
The system SHALL render a floating overlay chip positioned at the top-left of each terminal pane (`absolute().top_2().left_2()`) when an AI session is bound to that pane. The chip SHALL display a status dot followed by the humanized agent tool name and, when the session's `model` is present, a `·` separator and the shortened model name (`AISessionSnapshot.model` is `Option<String>`). The chip SHALL NOT alter the terminal pane layout or reduce terminal content area.

#### Scenario: Chip appears when agent session is bound
- **WHEN** a terminal pane has a bound `AISessionSnapshot` with `tool` `"claude"` and `model` `Some("claude-sonnet-4-5-20250514")`
- **THEN** a floating chip SHALL appear at the top-left of the pane
- **AND** the chip text SHALL read `"Claude Code · Sonnet 4.5"`

#### Scenario: Chip omits model segment when model is unknown
- **WHEN** a terminal pane has a bound `AISessionSnapshot` with `tool` `"codex"` and `model` `None`
- **THEN** the chip text SHALL read `"Codex"` with no trailing separator

#### Scenario: Chip hidden when no agent session
- **WHEN** a terminal pane has no bound AI session (plain shell)
- **THEN** no overlay chip SHALL be rendered
- **AND** the terminal pane SHALL display at full size with no overlay elements at the top-left

### Requirement: Status Dot Visual States
The system SHALL render a status dot inside the overlay chip whose appearance reflects the current `AgentLifecycleState`:
- `Working` → animated spinning dot in blue accent (`#4C8DFF`)
- `Waiting` → static dot in amber
- `Completed` → green checkmark; the checkmark disappears when the state decays to `Idle` (3 seconds, per the agent-lifecycle Completed Decay requirement)
- `Idle` → no dot (chip still shows agent name + model)

#### Scenario: Working state shows spinning blue dot
- **WHEN** the pane's `AgentLifecycleState` is `Working`
- **THEN** the status dot SHALL be blue (`#4C8DFF`)
- **AND** the dot SHALL be animated (spinning or pulsing)

#### Scenario: Waiting state shows static amber dot
- **WHEN** the pane's `AgentLifecycleState` is `Waiting`
- **THEN** the status dot SHALL be amber
- **AND** the dot SHALL be static (no animation)

#### Scenario: Completed state shows brief green checkmark
- **WHEN** the pane's `AgentLifecycleState` transitions to `Completed`
- **THEN** a green checkmark SHALL appear
- **AND** when the state decays to `Idle` after 3 seconds the checkmark SHALL disappear, leaving only the agent name + model text

### Requirement: Agent Name Humanization
The system SHALL provide a `humanize_tool_name(tool: &str) -> String` function that converts canonical agent tool identifiers (as produced by `canonical_tool_name` in `crates/codux-runtime-live/src/ai_runtime/tool_driver.rs`) to human-readable display names. Known mappings SHALL include: `"claude"` → `"Claude Code"`, `"codex"` → `"Codex"`, `"kiro"` → `"Kiro"`, `"opencode"` → `"OpenCode"`. Unknown tool names SHALL be title-cased with underscores and hyphens replaced by spaces as a fallback.

#### Scenario: Known agent name humanized
- **WHEN** `humanize_tool_name("claude")` is called
- **THEN** the result SHALL be `"Claude Code"`

#### Scenario: Unknown agent name falls back to title-case
- **WHEN** `humanize_tool_name("new_agent")` is called
- **THEN** the result SHALL be `"New Agent"`

### Requirement: Model Name Shortening
The system SHALL provide a `shorten_model_name(model: &str) -> String` function that strips version suffixes and date stamps from model identifiers for compact display. Known families SHALL have explicit mappings, including Claude (`"claude-sonnet-4-5-20250514"` → `"Sonnet 4.5"`) and GPT (`"gpt-4o"` → `"GPT-4o"`). Unknown models SHALL fall back to the raw string truncated to 20 characters.

#### Scenario: Claude model with date suffix shortened
- **WHEN** `shorten_model_name("claude-sonnet-4-5-20250514")` is called
- **THEN** the result SHALL be `"Sonnet 4.5"`

#### Scenario: GPT model mapped explicitly
- **WHEN** `shorten_model_name("gpt-4o")` is called
- **THEN** the result SHALL be `"GPT-4o"`

#### Scenario: Unknown model truncated
- **WHEN** `shorten_model_name("some-very-long-model-identifier-v2")` is called
- **THEN** the result SHALL be truncated to at most 20 characters

### Requirement: Collapsible Overlay
The system SHALL allow the user to collapse the overlay chip by clicking it. When collapsed, the chip SHALL disappear. The chip SHALL automatically reappear when the pane's `AgentLifecycleState` transitions to `Waiting`, so the user never misses an agent approval prompt. The collapsed state SHALL reset (chip visible) on each new app session.

#### Scenario: User collapses chip
- **WHEN** the user clicks the overlay chip on a pane
- **THEN** the chip SHALL disappear from that pane

#### Scenario: Collapsed chip reappears on waiting
- **WHEN** a pane's chip is collapsed and the `AgentLifecycleState` transitions to `Waiting`
- **THEN** the chip SHALL reappear with the amber waiting dot

#### Scenario: Collapse resets on restart
- **WHEN** the app is restarted after the user collapsed a pane's chip
- **THEN** the chip SHALL be visible again on that pane if an agent session is bound

### Requirement: Motion Reduction Support
The system SHALL disable the spinning/pulsing animation on the status dot when the operating system's "reduce motion" accessibility setting is enabled, showing a static dot instead.

#### Scenario: Animation disabled when reduce motion is on
- **WHEN** the system "reduce motion" setting is enabled and the pane state is `Working`
- **THEN** the status dot SHALL be static blue (no animation)
