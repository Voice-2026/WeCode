use crate::hub::{PeerSnapshot, peer_protocol};
use codux_protocol::RemoteRelayEnvelope;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use tracing::warn;

#[derive(Debug)]
pub struct StatsRecorder {
    path: PathBuf,
    flush_interval: Duration,
    inner: Mutex<StatsInner>,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct Counters {
    connections_total: u64,
    disconnections_total: u64,
    messages_total: u64,
    messages_forwarded: u64,
    messages_dropped: u64,
    bytes_total: u64,
    rate_limited_total: u64,
    oversized_total: u64,
    upload_blocked_total: u64,
}

#[derive(Debug)]
struct StatsInner {
    file: File,
    counters: Counters,
    last_flush: Instant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsEntry<'a> {
    time: i64,
    event: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    counters: Option<Counters>,
}

impl StatsRecorder {
    pub fn open(path: &Path, flush_interval: Duration) -> anyhow::Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            flush_interval,
            inner: Mutex::new(StatsInner {
                file,
                counters: Counters::default(),
                last_flush: Instant::now(),
            }),
        })
    }

    pub fn record_connect(&self, peer: &PeerSnapshot) {
        self.with_inner(|inner| {
            inner.counters.connections_total += 1;
            write_entry(
                &self.path,
                inner,
                StatsEntry::peer("connect", peer).with_counters(None),
            );
            self.maybe_snapshot(inner);
        });
    }

    pub fn record_disconnect(&self, peer: &PeerSnapshot) {
        self.with_inner(|inner| {
            inner.counters.disconnections_total += 1;
            write_entry(
                &self.path,
                inner,
                StatsEntry::peer("disconnect", peer).with_counters(None),
            );
            self.maybe_snapshot(inner);
        });
    }

    pub fn record_message(&self, size: usize) {
        self.with_inner(|inner| {
            inner.counters.messages_total += 1;
            inner.counters.bytes_total += size as u64;
            self.maybe_snapshot(inner);
        });
    }

    pub fn record_forwarded(&self, count: usize) {
        self.with_inner(|inner| {
            inner.counters.messages_forwarded += count as u64;
            self.maybe_snapshot(inner);
        });
    }

    pub fn record_dropped(
        &self,
        peer: &PeerSnapshot,
        envelope: &RemoteRelayEnvelope,
        reason: &str,
        size: usize,
    ) {
        self.with_inner(|inner| {
            inner.counters.messages_dropped += 1;
            match reason {
                "rate_limited" => inner.counters.rate_limited_total += 1,
                "message_too_large" => inner.counters.oversized_total += 1,
                "upload_requires_p2p" => inner.counters.upload_blocked_total += 1,
                _ => {}
            }
            write_entry(
                &self.path,
                inner,
                StatsEntry::peer("drop", peer)
                    .with_kind(&envelope.kind)
                    .with_reason(reason)
                    .with_bytes(size),
            );
            self.maybe_snapshot(inner);
        });
    }

    pub fn close(&self) {
        self.with_inner(|inner| {
            write_entry(&self.path, inner, StatsEntry::snapshot(inner.counters));
        });
    }

    fn maybe_snapshot(&self, inner: &mut StatsInner) {
        if inner.last_flush.elapsed() < self.flush_interval {
            return;
        }
        inner.last_flush = Instant::now();
        write_entry(&self.path, inner, StatsEntry::snapshot(inner.counters));
    }

    fn with_inner(&self, f: impl FnOnce(&mut StatsInner)) {
        match self.inner.lock() {
            Ok(mut inner) => f(&mut inner),
            Err(error) => warn!(
                "stats lock poisoned path={} error={error}",
                self.path.display()
            ),
        }
    }
}

impl<'a> StatsEntry<'a> {
    fn peer(event: &'a str, peer: &'a PeerSnapshot) -> Self {
        Self {
            time: crate::store::now_millis(),
            event,
            protocol: Some(peer_protocol(peer)),
            role: Some(peer.role.as_str()),
            host_id: Some(&peer.host_id),
            device_id: if peer.device_id.is_empty() {
                None
            } else {
                Some(&peer.device_id)
            },
            kind: None,
            reason: None,
            bytes: None,
            counters: None,
        }
    }

    fn snapshot(counters: Counters) -> Self {
        Self {
            time: crate::store::now_millis(),
            event: "snapshot",
            protocol: None,
            role: None,
            host_id: None,
            device_id: None,
            kind: None,
            reason: None,
            bytes: None,
            counters: Some(counters),
        }
    }

    fn with_kind(mut self, kind: &'a str) -> Self {
        self.kind = Some(kind);
        self
    }

    fn with_reason(mut self, reason: &'a str) -> Self {
        self.reason = Some(reason);
        self
    }

    fn with_bytes(mut self, bytes: usize) -> Self {
        self.bytes = Some(bytes);
        self
    }

    fn with_counters(mut self, counters: Option<Counters>) -> Self {
        self.counters = counters;
        self
    }
}

fn write_entry(path: &Path, inner: &mut StatsInner, entry: StatsEntry<'_>) {
    let Ok(data) = serde_json::to_vec(&entry) else {
        return;
    };
    if let Err(error) = inner
        .file
        .write_all(&data)
        .and_then(|_| inner.file.write_all(b"\n"))
    {
        warn!("stats write failed path={} error={error}", path.display());
    }
}
