#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TerminalProgressOsc {
    Started,
    Completed,
}

#[derive(Debug, Default)]
pub(super) struct TerminalProgressOscParser {
    scan_tail: Vec<u8>,
}

impl TerminalProgressOscParser {
    pub(super) fn push(&mut self, bytes: &[u8]) -> Vec<TerminalProgressOsc> {
        if bytes.is_empty() {
            return Vec::new();
        }
        if self.scan_tail.is_empty() && !bytes.contains(&0x1b) {
            return Vec::new();
        }
        let mut scan = Vec::with_capacity(self.scan_tail.len() + bytes.len());
        scan.extend_from_slice(&self.scan_tail);
        scan.extend_from_slice(bytes);

        let mut events = Vec::new();
        let mut index = 0;
        let mut consumed_until = 0;
        while index < scan.len() {
            let Some(relative) = scan[index..].iter().position(|byte| *byte == 0x1b) else {
                consumed_until = scan.len();
                break;
            };
            index += relative;
            let Some(rest) = scan.get(index..) else {
                break;
            };
            if b"\x1b]9;4;".starts_with(rest) {
                consumed_until = index;
                break;
            }
            let Some(body) = rest.strip_prefix(b"\x1b]9;4;") else {
                index += 1;
                consumed_until = index;
                continue;
            };
            let Some((value, terminator_len)) = terminal_progress_osc_value(body) else {
                consumed_until = index;
                break;
            };
            match value {
                b'1' => events.push(TerminalProgressOsc::Started),
                b'0' => events.push(TerminalProgressOsc::Completed),
                _ => {}
            }
            index += b"\x1b]9;4;".len() + 1 + terminator_len;
            consumed_until = index;
        }

        let tail = &scan[consumed_until.min(scan.len())..];
        let tail_len = tail.len().min(32);
        self.scan_tail.clear();
        self.scan_tail
            .extend_from_slice(&tail[tail.len().saturating_sub(tail_len)..]);
        events
    }
}

pub(super) fn terminal_progress_osc_value(body: &[u8]) -> Option<(u8, usize)> {
    let value = *body.first()?;
    let rest = &body[1..];
    if rest.first().copied() == Some(0x07) {
        return Some((value, 1));
    }
    if rest.starts_with(b"\x1b\\") {
        return Some((value, 2));
    }
    None
}
