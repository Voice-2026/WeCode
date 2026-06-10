use super::summary::remote_summary_from_settings;
use super::types::{RemotePairingInfo, RemotePendingPairing, RemoteSettings, RemoteSummary};

pub(crate) fn remote_summary_show_pending_pairing(
    settings: RemoteSettings,
    active_pairing: &RemotePairingInfo,
    pairing_id: String,
    device_name: String,
    device_public_key: String,
    pairing_code: String,
    _pairing_secret: String,
) -> RemoteSummary {
    let mut summary = remote_summary_from_settings(settings.clone());
    if pairing_id.trim().is_empty() {
        summary.pairing = Some(active_pairing.clone());
        return summary;
    }

    summary.status = "connected".to_string();
    summary.message = "Confirm device pairing.".to_string();
    summary.pending_pairing_list.push(RemotePendingPairing {
        id: pairing_id,
        device_name,
        device_public_key,
        code: pairing_code,
    });
    summary.pending_pairings = summary.pending_pairing_list.len();
    summary
}
