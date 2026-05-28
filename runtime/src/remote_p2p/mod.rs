mod peer;
mod transport;

pub use transport::{RemoteP2PHostTransport, RemoteP2PLane, RemoteP2PSignal};

const UPLOAD_CHANNEL_LABEL: &str = "codux-upload";
const TERMINAL_BUFFERED_AMOUNT_HIGH_WATERMARK: u32 = 192 * 1024;
const UPLOAD_BUFFERED_AMOUNT_HIGH_WATERMARK: u32 = 512 * 1024;

fn remote_p2p_ice_server_urls() -> Vec<String> {
    let domestic = vec!["stun:stun.miwifi.com:3478".to_string()];
    let global = vec![
        "stun:stun.l.google.com:19302".to_string(),
        "stun:global.stun.twilio.com:3478".to_string(),
    ];
    if prefers_domestic_stun() {
        domestic.into_iter().chain(global).collect()
    } else {
        global.into_iter().chain(domestic).collect()
    }
}

fn prefers_domestic_stun() -> bool {
    std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .map(|value| value.to_ascii_lowercase().starts_with("zh"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stun_order_prefers_global_for_non_chinese_locale() {
        unsafe {
            std::env::set_var("LANG", "en_US.UTF-8");
        }
        let urls = remote_p2p_ice_server_urls();
        assert_eq!(
            urls.first().map(String::as_str),
            Some("stun:stun.l.google.com:19302")
        );
    }
}
