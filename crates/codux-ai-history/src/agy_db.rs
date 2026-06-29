use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct AgyConversation {
    pub conversation_id: Option<String>,
    pub project_path: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_input_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub first_seen_at: Option<f64>,
    pub last_seen_at: Option<f64>,
    pub last_user_at: Option<f64>,
    pub last_model_at: Option<f64>,
    pub assistant_preview: Option<String>,
    pub events: Vec<AgyConversationEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgyConversationRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct AgyConversationEvent {
    pub timestamp: f64,
    pub role: AgyConversationRole,
}

#[derive(Debug, Default, Clone)]
struct AgyStep {
    timestamp: Option<f64>,
    step_type: i64,
    status: i64,
    text: Option<String>,
}

pub fn parse_agy_conversation_db(database_path: &Path) -> Option<AgyConversation> {
    let conn =
        Connection::open_with_flags(database_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    let mut conversation = AgyConversation {
        conversation_id: database_path
            .file_stem()
            .and_then(|name| name.to_str())
            .and_then(normalized_string),
        ..Default::default()
    };
    absorb_trajectory_metadata(&conn, &mut conversation);
    absorb_gen_metadata(&conn, &mut conversation);
    absorb_steps(&conn, &mut conversation);
    conversation.has_activity().then_some(conversation)
}

impl AgyConversation {
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens + self.output_tokens + self.reasoning_output_tokens
    }

    pub fn has_token_usage(&self) -> bool {
        self.total_tokens() > 0 || self.cached_input_tokens > 0
    }

    fn has_activity(&self) -> bool {
        self.first_seen_at.is_some()
            || self.last_seen_at.is_some()
            || self.has_token_usage()
            || self.model.is_some()
            || self.project_path.is_some()
            || !self.events.is_empty()
    }
}

fn absorb_trajectory_metadata(conn: &Connection, conversation: &mut AgyConversation) {
    let Ok(mut statement) = conn.prepare("SELECT data FROM trajectory_metadata_blob") else {
        return;
    };
    let Ok(rows) = statement.query_map([], |row| row.get::<_, Vec<u8>>(0)) else {
        return;
    };
    for data in rows.flatten() {
        let fields = ProtoMessage::new(&data);
        if conversation.project_path.is_none() {
            conversation.project_path = fields
                .first_message(1)
                .and_then(|message| message.first_string(1).or_else(|| message.first_string(2)))
                .or_else(|| fields.first_string(7))
                .and_then(|value| file_uri_to_path(&value));
        }
        if let Some(timestamp) = fields.first_message(2).and_then(timestamp_message_seconds) {
            note_timestamp(conversation, timestamp);
        }
        if conversation.conversation_id.is_none() {
            conversation.conversation_id = fields.first_string(6).and_then(normalized_string);
        }
    }
}

fn absorb_gen_metadata(conn: &Connection, conversation: &mut AgyConversation) {
    let Ok(mut statement) = conn.prepare("SELECT data FROM gen_metadata ORDER BY idx") else {
        return;
    };
    let Ok(rows) = statement.query_map([], |row| row.get::<_, Vec<u8>>(0)) else {
        return;
    };
    for data in rows.flatten() {
        let fields = ProtoMessage::new(&data);
        let metadata = fields.first_message(1);
        if let Some(model) = metadata
            .and_then(|message| {
                message
                    .first_string(21)
                    .or_else(|| message.first_string(19))
            })
            .or_else(|| fields.first_string(21))
            .or_else(|| fields.first_string(19))
            .and_then(normalized_string)
        {
            conversation.model = Some(model);
        }
        if let Some(timestamp) = fields
            .first_message(1)
            .and_then(|message| message.first_message(9))
            .and_then(|message| message.first_message(4))
            .and_then(timestamp_message_seconds)
        {
            note_timestamp(conversation, timestamp);
        }
        if let Some(usage) = fields
            .first_message(1)
            .and_then(|message| message.first_message(4))
            .and_then(agy_usage_from_message)
        {
            conversation.input_tokens += usage.input_tokens;
            conversation.output_tokens += usage.output_tokens;
            conversation.cached_input_tokens += usage.cached_input_tokens;
            conversation.reasoning_output_tokens += usage.reasoning_output_tokens;
        }
    }
}

fn absorb_steps(conn: &Connection, conversation: &mut AgyConversation) {
    let Ok(mut statement) =
        conn.prepare("SELECT step_type, status, metadata, step_payload FROM steps ORDER BY idx")
    else {
        return;
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<Vec<u8>>>(2)?,
            row.get::<_, Option<Vec<u8>>>(3)?,
        ))
    }) else {
        return;
    };

    for row in rows.flatten() {
        let (step_type, status, metadata, payload) = row;
        let metadata_step = metadata.as_deref().and_then(parse_step_metadata);
        let payload_step = payload.as_deref().and_then(parse_step_payload);
        let step = merge_step(step_type, status, metadata_step, payload_step);
        if let Some(timestamp) = step.timestamp {
            note_timestamp(conversation, timestamp);
        }
        let Some(role) = agy_step_role(step.step_type, step.status, step.text.as_deref()) else {
            continue;
        };
        let timestamp = step
            .timestamp
            .or(conversation.last_seen_at)
            .unwrap_or_default();
        conversation
            .events
            .push(AgyConversationEvent { timestamp, role });
        match role {
            AgyConversationRole::User => {
                conversation.last_user_at = Some(
                    conversation
                        .last_user_at
                        .map(|current| current.max(timestamp))
                        .unwrap_or(timestamp),
                );
                if conversation.title.is_none() {
                    conversation.title = step.text.as_deref().and_then(title_from_user_text);
                }
            }
            AgyConversationRole::Assistant => {
                conversation.last_model_at = Some(
                    conversation
                        .last_model_at
                        .map(|current| current.max(timestamp))
                        .unwrap_or(timestamp),
                );
                if let Some(text) = step.text.and_then(normalized_string) {
                    conversation.assistant_preview = Some(text);
                }
            }
        }
    }
}

fn parse_step_metadata(data: &[u8]) -> Option<AgyStep> {
    let fields = ProtoMessage::new(data);
    Some(AgyStep {
        timestamp: fields.first_message(1).and_then(timestamp_message_seconds),
        step_type: fields.first_i64(3).unwrap_or(0),
        status: 0,
        text: None,
    })
}

fn parse_step_payload(data: &[u8]) -> Option<AgyStep> {
    let fields = ProtoMessage::new(data);
    let structured = AgyStep {
        timestamp: fields
            .first_message(5)
            .and_then(|message| message.first_message(1))
            .and_then(timestamp_message_seconds),
        step_type: fields.first_i64(1).unwrap_or(0),
        status: fields.first_i64(4).unwrap_or(0),
        text: match fields.first_i64(1).unwrap_or(0) {
            14 => fields
                .first_message(19)
                .and_then(|message| message.first_string(2).or_else(|| message.first_string(8))),
            15 => fields
                .first_message(20)
                .and_then(|message| message.first_string(1).or_else(|| message.first_string(8))),
            _ => None,
        }
        .and_then(normalized_string),
    };
    if agy_structured_step_type_is_known(structured.step_type)
        && (structured.timestamp.is_some() || structured.status != 0 || structured.text.is_some())
    {
        return Some(structured);
    }
    None
}

fn merge_step(
    step_type: i64,
    status: i64,
    metadata: Option<AgyStep>,
    payload: Option<AgyStep>,
) -> AgyStep {
    AgyStep {
        timestamp: payload
            .as_ref()
            .and_then(|step| step.timestamp)
            .or_else(|| metadata.as_ref().and_then(|step| step.timestamp)),
        step_type: nonzero(
            payload.as_ref().map(|step| step.step_type).unwrap_or(0),
            nonzero(
                metadata.as_ref().map(|step| step.step_type).unwrap_or(0),
                step_type,
            ),
        ),
        status: nonzero(
            payload.as_ref().map(|step| step.status).unwrap_or(0),
            nonzero(
                metadata.as_ref().map(|step| step.status).unwrap_or(0),
                status,
            ),
        ),
        text: payload
            .and_then(|step| step.text)
            .or_else(|| metadata.and_then(|step| step.text)),
    }
}

fn agy_step_role(step_type: i64, status: i64, text: Option<&str>) -> Option<AgyConversationRole> {
    if status != 0 && status != 3 {
        return None;
    }
    match step_type {
        14 => Some(AgyConversationRole::User),
        15 if text.and_then(normalized_string).is_some() => Some(AgyConversationRole::Assistant),
        _ => None,
    }
}

fn agy_structured_step_type_is_known(step_type: i64) -> bool {
    matches!(step_type, 8 | 9 | 14 | 15 | 17 | 21 | 23 | 98)
}

#[derive(Debug, Clone, Copy)]
struct AgyUsage {
    input_tokens: i64,
    output_tokens: i64,
    cached_input_tokens: i64,
    reasoning_output_tokens: i64,
}

fn agy_usage_from_message(message: ProtoMessage<'_>) -> Option<AgyUsage> {
    let input_tokens = message.first_i64(2).unwrap_or(0);
    let output_total = message.first_i64(3).unwrap_or(0);
    let cached_input_tokens = message.first_i64(5).unwrap_or(0);
    let reasoning_output_tokens = message.first_i64(9).unwrap_or(0);
    let output_tokens = message
        .first_i64(10)
        .unwrap_or_else(|| (output_total - reasoning_output_tokens).max(0));
    (input_tokens > 0
        || output_tokens > 0
        || reasoning_output_tokens > 0
        || cached_input_tokens > 0)
        .then_some(AgyUsage {
            input_tokens,
            output_tokens,
            cached_input_tokens,
            reasoning_output_tokens,
        })
}

fn note_timestamp(conversation: &mut AgyConversation, timestamp: f64) {
    if timestamp <= 0.0 {
        return;
    }
    conversation.first_seen_at = Some(
        conversation
            .first_seen_at
            .map(|current| current.min(timestamp))
            .unwrap_or(timestamp),
    );
    conversation.last_seen_at = Some(
        conversation
            .last_seen_at
            .map(|current| current.max(timestamp))
            .unwrap_or(timestamp),
    );
}

fn title_from_user_text(value: &str) -> Option<String> {
    let value = value
        .split("<USER_REQUEST>")
        .nth(1)
        .and_then(|value| value.split("</USER_REQUEST>").next())
        .unwrap_or(value)
        .trim();
    normalized_string(value).map(|value| {
        let mut title = value.chars().take(80).collect::<String>();
        if value.chars().count() > 80 {
            title.push('…');
        }
        title
    })
}

fn file_uri_to_path(value: &str) -> Option<String> {
    let value = normalized_string(value)?;
    if let Some(path) = value.strip_prefix("file://") {
        Some(path.to_string())
    } else {
        Some(value)
    }
}

fn timestamp_message_seconds(message: ProtoMessage<'_>) -> Option<f64> {
    let seconds = message.first_i64(1)?;
    let nanos = message.first_i64(2).unwrap_or(0);
    Some(seconds as f64 + nanos as f64 / 1_000_000_000.0)
}

fn normalized_string(value: impl AsRef<str>) -> Option<String> {
    let value = value.as_ref().trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn nonzero(value: i64, fallback: i64) -> i64 {
    if value != 0 { value } else { fallback }
}

#[derive(Debug, Clone, Copy)]
struct ProtoMessage<'a> {
    data: &'a [u8],
}

impl<'a> ProtoMessage<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn fields(&self) -> ProtoFields<'a> {
        ProtoFields {
            data: self.data,
            position: 0,
        }
    }

    fn first_i64(&self, number: u64) -> Option<i64> {
        self.fields().find_map(|field| match field {
            ProtoField::Varint(field_number, value) if field_number == number => {
                Some(value.min(i64::MAX as u64) as i64)
            }
            _ => None,
        })
    }

    fn first_string(&self, number: u64) -> Option<String> {
        self.fields().find_map(|field| match field {
            ProtoField::LengthDelimited(field_number, value) if field_number == number => {
                std::str::from_utf8(value).ok().and_then(normalized_string)
            }
            _ => None,
        })
    }

    fn first_message(&self, number: u64) -> Option<ProtoMessage<'a>> {
        self.fields().find_map(|field| match field {
            ProtoField::LengthDelimited(field_number, value) if field_number == number => {
                Some(ProtoMessage::new(value))
            }
            _ => None,
        })
    }
}

struct ProtoFields<'a> {
    data: &'a [u8],
    position: usize,
}

enum ProtoField<'a> {
    Varint(u64, u64),
    LengthDelimited(u64, &'a [u8]),
    Fixed32,
    Fixed64,
}

impl<'a> Iterator for ProtoFields<'a> {
    type Item = ProtoField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let key = read_varint(self.data, &mut self.position)?;
        let field_number = key >> 3;
        match key & 7 {
            0 => read_varint(self.data, &mut self.position)
                .map(|value| ProtoField::Varint(field_number, value)),
            1 => {
                let end = self.position.checked_add(8)?;
                if end > self.data.len() {
                    return None;
                }
                self.position = end;
                Some(ProtoField::Fixed64)
            }
            2 => {
                let length = read_varint(self.data, &mut self.position)? as usize;
                let end = self.position.checked_add(length)?;
                if end > self.data.len() {
                    return None;
                }
                let value = &self.data[self.position..end];
                self.position = end;
                Some(ProtoField::LengthDelimited(field_number, value))
            }
            5 => {
                let end = self.position.checked_add(4)?;
                if end > self.data.len() {
                    return None;
                }
                self.position = end;
                Some(ProtoField::Fixed32)
            }
            _ => None,
        }
    }
}

fn read_varint(data: &[u8], position: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while *position < data.len() && shift < 64 {
        let byte = data[*position];
        *position += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte < 0x80 {
            return Some(value);
        }
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(number: u64, wire_type: u64) -> Vec<u8> {
        varint((number << 3) | wire_type)
    }

    fn varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return out;
            }
        }
    }

    fn string_field(number: u64, value: &str) -> Vec<u8> {
        let mut out = field(number, 2);
        out.extend(varint(value.len() as u64));
        out.extend(value.as_bytes());
        out
    }

    fn int_field(number: u64, value: u64) -> Vec<u8> {
        let mut out = field(number, 0);
        out.extend(varint(value));
        out
    }

    fn message_field(number: u64, value: Vec<u8>) -> Vec<u8> {
        let mut out = field(number, 2);
        out.extend(varint(value.len() as u64));
        out.extend(value);
        out
    }

    #[test]
    fn parses_gen_metadata_usage_and_model() {
        let usage = [
            int_field(2, 17157),
            int_field(3, 86),
            int_field(9, 64),
            int_field(10, 22),
        ]
        .concat();
        let data = [message_field(
            1,
            [
                message_field(4, usage),
                string_field(21, "Agy 3.5 Flash (Medium)"),
            ]
            .concat(),
        )]
        .concat();
        let fields = ProtoMessage::new(&data);
        let metadata = fields.first_message(1).unwrap();
        let usage = fields
            .first_message(1)
            .and_then(|message| message.first_message(4))
            .and_then(agy_usage_from_message)
            .unwrap();

        assert_eq!(
            metadata.first_string(21).as_deref(),
            Some("Agy 3.5 Flash (Medium)")
        );
        assert_eq!(usage.input_tokens, 17157);
        assert_eq!(usage.output_tokens, 22);
        assert_eq!(usage.reasoning_output_tokens, 64);
        assert_eq!(
            usage.input_tokens + usage.output_tokens + usage.reasoning_output_tokens,
            17243
        );
    }

    #[test]
    fn parses_structured_step_payload() {
        let timestamp = [int_field(1, 1_767_225_600), int_field(2, 0)].concat();
        let metadata = [message_field(1, timestamp)].concat();
        let text = [string_field(2, "hi {x}")].concat();
        let data = [
            int_field(1, 14),
            int_field(4, 3),
            message_field(5, metadata),
            message_field(19, text),
        ]
        .concat();

        let step = parse_step_payload(&data).unwrap();

        assert_eq!(step.step_type, 14);
        assert_eq!(step.status, 3);
        assert_eq!(step.timestamp, Some(1767225600.0));
        assert_eq!(step.text.as_deref(), Some("hi {x}"));
    }
}
