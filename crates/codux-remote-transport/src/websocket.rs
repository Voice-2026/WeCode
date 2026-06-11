use crate::control_messages::{pairing_handshake_from_envelope, transport_pong_for_ping};
use crate::{
    RemoteTransport, RemoteTransportControlHandler, RemoteTransportLogHandler,
    RemoteTransportMessageHandler, RemoteTransportPairingHandler, RemoteTransportStateHandler,
};
use async_trait::async_trait;
use codux_protocol::{RemoteEnvelope, RemoteTransportKind};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WebSocketMessage;

pub struct RemoteWebSocketHostTransport {
    pub(crate) tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pub(crate) on_message: RemoteTransportMessageHandler,
    pub(crate) on_state: RemoteTransportStateHandler,
    pub(crate) on_pairing: RemoteTransportPairingHandler,
    pub(crate) on_control: Option<RemoteTransportControlHandler>,
    pub(crate) on_log: Option<RemoteTransportLogHandler>,
}

impl RemoteWebSocketHostTransport {
    pub async fn connect(
        ws_url: String,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_pairing: RemoteTransportPairingHandler,
        on_control: Option<RemoteTransportControlHandler>,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<Self>, String> {
        let (socket, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|error| error.to_string())?;
        let (mut write, mut read) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let transport = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            on_message,
            on_state,
            on_pairing,
            on_control,
            on_log,
        });

        let writer = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if write
                    .send(WebSocketMessage::Text(message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            writer.close_sender();
        });

        let reader = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(WebSocketMessage::Text(text)) => {
                        reader.handle_text(text.to_string());
                    }
                    Ok(WebSocketMessage::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        reader.log(format!("websocket_recv failed error={error}"));
                        break;
                    }
                }
            }
            reader.close_sender();
            (reader.on_state)(String::new(), "closed".to_string());
        });

        Ok(transport)
    }

    pub(crate) fn handle_text(&self, text: String) {
        let Ok(raw) = serde_json::from_str::<RemoteEnvelope>(&text) else {
            self.log("websocket_recv drop reason=decode".to_string());
            return;
        };
        if raw.kind == "pairing.request" {
            if let Some(handshake) = pairing_handshake_from_envelope(&raw) {
                (self.on_pairing)(handshake);
            }
        }
        let device_id = raw
            .device_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_default();
        if !device_id.is_empty() {
            (self.on_state)(device_id.clone(), "connected".to_string());
        }
        if let Some(pong) = transport_pong_for_ping(&raw, Some(&device_id)) {
            let _ = self.send(pong.into_bytes(), None);
            return;
        }
        if self
            .on_control
            .as_ref()
            .map(|handler| handler(device_id.clone(), raw))
            .unwrap_or(false)
        {
            return;
        }
        (self.on_message)(device_id, text.into_bytes());
    }

    pub(crate) fn close_sender(&self) {
        if let Ok(mut tx) = self.tx.lock() {
            *tx = None;
        }
    }

    fn log(&self, message: String) {
        if let Some(on_log) = self.on_log.as_ref() {
            on_log(message);
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteWebSocketHostTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebSocketRelay
    }

    fn send(&self, data: Vec<u8>, _device_id: Option<&str>) -> bool {
        let Ok(text) = String::from_utf8(data) else {
            return false;
        };
        self.tx
            .lock()
            .ok()
            .and_then(|tx| tx.clone())
            .map(|tx| tx.send(text).is_ok())
            .unwrap_or(false)
    }

    async fn shutdown(&self) {
        self.close_sender();
    }
}

pub struct RemoteWebSocketControllerTransport {
    pub(crate) tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pub(crate) closed: AtomicBool,
    pub(crate) on_message: RemoteTransportMessageHandler,
    pub(crate) on_state: RemoteTransportStateHandler,
    pub(crate) on_control: Mutex<Option<RemoteTransportControlHandler>>,
    pub(crate) on_log: Option<RemoteTransportLogHandler>,
}

impl RemoteWebSocketControllerTransport {
    pub async fn connect(
        ws_url: String,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<Self>, String> {
        let (socket, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|error| error.to_string())?;
        let (mut write, mut read) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let transport = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            closed: AtomicBool::new(false),
            on_message,
            on_state,
            on_control: Mutex::new(None),
            on_log,
        });

        let writer = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if write
                    .send(WebSocketMessage::Text(message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            writer.close_sender();
        });

        let reader = Arc::clone(&transport);
        tokio::spawn(async move {
            (reader.on_state)(String::new(), "connected".to_string());
            while let Some(message) = read.next().await {
                match message {
                    Ok(WebSocketMessage::Text(text)) => {
                        reader.handle_text(text.to_string());
                    }
                    Ok(WebSocketMessage::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        reader.log(format!("controller_websocket_recv failed error={error}"));
                        break;
                    }
                }
            }
            reader.close_sender();
            (reader.on_state)(String::new(), "closed".to_string());
        });

        Ok(transport)
    }

    pub fn set_control_handler(&self, handler: Option<RemoteTransportControlHandler>) {
        if let Ok(mut current) = self.on_control.lock() {
            *current = handler;
        }
    }

    pub(crate) fn handle_text(&self, text: String) {
        let control_handled = serde_json::from_str::<RemoteEnvelope>(&text)
            .ok()
            .and_then(|envelope| {
                self.on_control
                    .lock()
                    .ok()
                    .and_then(|handler| handler.clone())
                    .map(|handler| handler(String::new(), envelope))
            })
            .unwrap_or(false);
        if control_handled {
            return;
        }
        (self.on_message)(String::new(), text.into_bytes());
    }

    pub(crate) fn close_sender(&self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Ok(mut tx) = self.tx.lock() {
            *tx = None;
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
            && self.tx.lock().ok().and_then(|tx| tx.clone()).is_some()
    }

    fn log(&self, message: String) {
        if let Some(on_log) = self.on_log.as_ref() {
            on_log(message);
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteWebSocketControllerTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebSocketRelay
    }

    fn send(&self, data: Vec<u8>, _device_id: Option<&str>) -> bool {
        if self.closed.load(Ordering::SeqCst) {
            return false;
        }
        let Ok(text) = String::from_utf8(data) else {
            return false;
        };
        self.tx
            .lock()
            .ok()
            .and_then(|tx| tx.clone())
            .map(|tx| tx.send(text).is_ok())
            .unwrap_or(false)
    }

    async fn shutdown(&self) {
        self.close_sender();
    }
}
