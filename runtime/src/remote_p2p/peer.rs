use super::{
    TERMINAL_BUFFERED_AMOUNT_HIGH_WATERMARK, UPLOAD_BUFFERED_AMOUNT_HIGH_WATERMARK,
    transport::RemoteP2PLane,
};
use bytes::BytesMut;
use std::{sync::Arc, sync::Mutex, time::Duration};
use webrtc::{
    data_channel::{DataChannel, RTCDataChannelState},
    peer_connection::PeerConnection,
};

pub(super) struct RemoteP2PPeer {
    pub(super) pc: Arc<dyn PeerConnection>,
    terminal_channel: Mutex<Option<Arc<dyn DataChannel>>>,
    upload_channel: Mutex<Option<Arc<dyn DataChannel>>>,
}

impl RemoteP2PPeer {
    pub(super) fn new(pc: Arc<dyn PeerConnection>) -> Self {
        Self {
            pc,
            terminal_channel: Mutex::new(None),
            upload_channel: Mutex::new(None),
        }
    }

    pub(super) async fn close(&self) {
        let terminal = self
            .terminal_channel
            .lock()
            .ok()
            .and_then(|mut value| value.take());
        let upload = self
            .upload_channel
            .lock()
            .ok()
            .and_then(|mut value| value.take());
        if let Some(channel) = terminal {
            let _ = channel.close().await;
        }
        if let Some(channel) = upload {
            let _ = channel.close().await;
        }
        let _ = self.pc.close().await;
    }

    pub(super) async fn is_open(&self) -> bool {
        let channel = self
            .terminal_channel
            .lock()
            .ok()
            .and_then(|value| value.clone());
        match channel {
            Some(channel) => channel.ready_state().await.ok() == Some(RTCDataChannelState::Open),
            None => false,
        }
    }

    pub(super) async fn send(&self, data: Vec<u8>, lane: RemoteP2PLane) -> bool {
        let Some(channel) = self.channel(lane) else {
            return false;
        };
        if channel.ready_state().await.ok() != Some(RTCDataChannelState::Open) {
            return false;
        }
        tokio::time::timeout(
            Duration::from_millis(250),
            channel.send(BytesMut::from(data.as_slice())),
        )
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false)
    }

    pub(super) fn set_channel(&self, upload: bool, channel: Arc<dyn DataChannel>) {
        let target = if upload {
            &self.upload_channel
        } else {
            &self.terminal_channel
        };
        if let Ok(mut current) = target.lock() {
            *current = Some(channel);
        }
    }

    pub(super) fn clear_channel(&self, upload: bool) {
        let target = if upload {
            &self.upload_channel
        } else {
            &self.terminal_channel
        };
        if let Ok(mut current) = target.lock() {
            *current = None;
        }
    }

    fn channel(&self, lane: RemoteP2PLane) -> Option<Arc<dyn DataChannel>> {
        if matches!(lane, RemoteP2PLane::Upload) {
            return self
                .upload_channel
                .lock()
                .ok()
                .and_then(|value| value.clone());
        }
        self.terminal_channel
            .lock()
            .ok()
            .and_then(|value| value.clone())
    }
}

pub(super) fn channel_high_watermark(upload: bool) -> u32 {
    if upload {
        UPLOAD_BUFFERED_AMOUNT_HIGH_WATERMARK
    } else {
        TERMINAL_BUFFERED_AMOUNT_HIGH_WATERMARK
    }
}
