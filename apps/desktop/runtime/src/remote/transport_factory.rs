use super::types::RemoteSettings;
use codux_remote_transport::{
    RemoteHostTransportConfig, RemoteTransport, RemoteTransportFactory as SharedTransportFactory,
    RemoteTransportMessageHandler, RemoteTransportPairingHandler, RemoteTransportStateHandler,
    remote_stun_urls,
};
use std::sync::Arc;

pub(crate) struct RemoteTransportFactory;

impl RemoteTransportFactory {
    pub(crate) async fn connect_host(
        settings: &RemoteSettings,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_pairing: RemoteTransportPairingHandler,
    ) -> Result<Arc<dyn RemoteTransport>, String> {
        SharedTransportFactory::connect_host(
            &RemoteHostTransportConfig {
                server_url: settings.server_url.clone(),
                host_id: settings.host_id.clone(),
                host_token: settings.host_token.clone(),
                stun_urls: remote_stun_urls(),
            },
            on_message,
            on_state,
            on_pairing,
            Some(Arc::new(|message| {
                crate::runtime_trace::runtime_trace("remote", &message);
            })),
        )
        .await
    }
}
