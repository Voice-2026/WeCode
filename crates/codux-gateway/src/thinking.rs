//! FSM parser that extracts `<thinking>` blocks from the start of a response
//! stream. Faithful port of thinking_parser.py.

const OPEN_TAGS: &[&str] = &["<thinking>", "<think>", "<reasoning>", "<thought>"];
const INITIAL_BUFFER_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    PreContent,
    InThinking,
    Streaming,
}

#[derive(Debug, Default, Clone)]
pub struct ParseResult {
    pub thinking_content: Option<String>,
    pub regular_content: Option<String>,
    pub is_first_thinking_chunk: bool,
    pub is_last_thinking_chunk: bool,
    state_changed: bool,
}

pub struct ThinkingParser {
    handling_mode: String,
    max_tag_length: usize,
    state: State,
    initial_buffer: String,
    thinking_buffer: String,
    open_tag: Option<&'static str>,
    close_tag: Option<String>,
    is_first_thinking_chunk: bool,
    found_thinking_block: bool,
}

impl ThinkingParser {
    pub fn new(handling_mode: &str) -> Self {
        let max_tag_length = OPEN_TAGS.iter().map(|t| t.len()).max().unwrap_or(0) * 2;
        Self {
            handling_mode: handling_mode.to_string(),
            max_tag_length,
            state: State::PreContent,
            initial_buffer: String::new(),
            thinking_buffer: String::new(),
            open_tag: None,
            close_tag: None,
            is_first_thinking_chunk: true,
            found_thinking_block: false,
        }
    }

    pub fn found_thinking_block(&self) -> bool {
        self.found_thinking_block
    }

    pub fn feed(&mut self, content: &str) -> ParseResult {
        let mut result = ParseResult::default();
        if content.is_empty() {
            return result;
        }
        if self.state == State::PreContent {
            result = self.handle_pre_content(content);
        }
        if self.state == State::InThinking && !result.state_changed {
            result = self.handle_in_thinking(content);
        }
        if self.state == State::Streaming && !result.state_changed {
            result.regular_content = Some(content.to_string());
        }
        result
    }

    fn handle_pre_content(&mut self, content: &str) -> ParseResult {
        let mut result = ParseResult::default();
        self.initial_buffer.push_str(content);
        let stripped = self.initial_buffer.trim_start().to_string();

        for tag in OPEN_TAGS {
            if stripped.starts_with(tag) {
                self.state = State::InThinking;
                self.open_tag = Some(tag);
                self.close_tag = Some(format!("</{}", &tag[1..]));
                self.found_thinking_block = true;
                result.state_changed = true;

                let after = &stripped[tag.len()..];
                self.thinking_buffer = after.to_string();
                self.initial_buffer.clear();

                let tr = self.process_thinking_buffer();
                if tr.thinking_content.is_some() {
                    result.thinking_content = tr.thinking_content;
                    result.is_first_thinking_chunk = tr.is_first_thinking_chunk;
                }
                if tr.is_last_thinking_chunk {
                    result.is_last_thinking_chunk = true;
                }
                if tr.regular_content.is_some() {
                    result.regular_content = tr.regular_content;
                }
                return result;
            }
        }

        // Might still be receiving the tag.
        for tag in OPEN_TAGS {
            if tag.starts_with(&stripped) && stripped.len() < tag.len() {
                return result;
            }
        }

        if self.initial_buffer.len() > INITIAL_BUFFER_SIZE || !could_be_tag_prefix(&stripped) {
            self.state = State::Streaming;
            result.state_changed = true;
            result.regular_content = Some(std::mem::take(&mut self.initial_buffer));
        }
        result
    }

    fn handle_in_thinking(&mut self, content: &str) -> ParseResult {
        self.thinking_buffer.push_str(content);
        self.process_thinking_buffer()
    }

    fn process_thinking_buffer(&mut self) -> ParseResult {
        let mut result = ParseResult::default();
        let Some(close_tag) = self.close_tag.clone() else {
            return result;
        };

        if let Some(idx) = self.thinking_buffer.find(&close_tag) {
            let thinking_content = self.thinking_buffer[..idx].to_string();
            let after = self.thinking_buffer[idx + close_tag.len()..].to_string();
            if !thinking_content.is_empty() {
                result.thinking_content = Some(thinking_content);
                result.is_first_thinking_chunk = self.is_first_thinking_chunk;
                self.is_first_thinking_chunk = false;
            }
            result.is_last_thinking_chunk = true;
            self.state = State::Streaming;
            result.state_changed = true;
            self.thinking_buffer.clear();
            let stripped_after = after.trim_start();
            if !stripped_after.is_empty() {
                result.regular_content = Some(stripped_after.to_string());
            }
            return result;
        }

        // Cautious sending: keep the last max_tag_length chars buffered.
        if self.thinking_buffer.chars().count() > self.max_tag_length {
            let chars: Vec<char> = self.thinking_buffer.chars().collect();
            let split = chars.len() - self.max_tag_length;
            let send_part: String = chars[..split].iter().collect();
            self.thinking_buffer = chars[split..].iter().collect();
            result.thinking_content = Some(send_part);
            result.is_first_thinking_chunk = self.is_first_thinking_chunk;
            self.is_first_thinking_chunk = false;
        }
        result
    }

    pub fn finalize(&mut self) -> ParseResult {
        let mut result = ParseResult::default();
        if !self.thinking_buffer.is_empty() {
            if self.state == State::InThinking {
                result.thinking_content = Some(std::mem::take(&mut self.thinking_buffer));
                result.is_first_thinking_chunk = self.is_first_thinking_chunk;
                result.is_last_thinking_chunk = true;
            } else {
                result.regular_content = Some(std::mem::take(&mut self.thinking_buffer));
            }
            self.thinking_buffer.clear();
        }
        if !self.initial_buffer.is_empty() {
            let mut reg = result.regular_content.unwrap_or_default();
            reg.push_str(&self.initial_buffer);
            result.regular_content = Some(reg);
            self.initial_buffer.clear();
        }
        result
    }

    /// Apply the handling mode to thinking content. Returns None for "remove".
    pub fn process_for_output(
        &self,
        content: &str,
        is_first: bool,
        is_last: bool,
    ) -> Option<String> {
        if content.is_empty() {
            return None;
        }
        match self.handling_mode.as_str() {
            "remove" => None,
            "pass" => {
                let prefix = if is_first {
                    self.open_tag.unwrap_or("")
                } else {
                    ""
                };
                let suffix = if is_last {
                    self.close_tag.as_deref().unwrap_or("")
                } else {
                    ""
                };
                Some(format!("{prefix}{content}{suffix}"))
            }
            // "strip_tags" and "as_reasoning_content" both return raw content.
            _ => Some(content.to_string()),
        }
    }
}

fn could_be_tag_prefix(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    OPEN_TAGS.iter().any(|tag| tag.starts_with(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_thinking_across_chunks() {
        let mut p = ThinkingParser::new("as_reasoning_content");
        let mut thinking = String::new();
        let mut regular = String::new();
        for chunk in [
            "<think",
            "ing>reason",
            "ing here</thinking>",
            "final answer",
        ] {
            let r = p.feed(chunk);
            if let Some(t) = r.thinking_content {
                thinking.push_str(&t);
            }
            if let Some(c) = r.regular_content {
                regular.push_str(&c);
            }
        }
        let f = p.finalize();
        if let Some(t) = f.thinking_content {
            thinking.push_str(&t);
        }
        if let Some(c) = f.regular_content {
            regular.push_str(&c);
        }
        assert!(p.found_thinking_block());
        assert_eq!(thinking, "reasoning here");
        assert_eq!(regular, "final answer");
    }

    #[test]
    fn passes_through_plain_content() {
        let mut p = ThinkingParser::new("as_reasoning_content");
        let r = p.feed("just a normal answer with no tags at all");
        assert_eq!(
            r.regular_content.as_deref(),
            Some("just a normal answer with no tags at all")
        );
        assert!(!p.found_thinking_block());
    }
}
