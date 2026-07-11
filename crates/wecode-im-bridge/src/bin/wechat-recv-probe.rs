//! Live receive probe: long-polls `getupdates` with saved credentials and
//! prints every raw message, so we can see exactly what the server delivers.
//!
//! Run: WECODE_WECHAT_CREDS=~/Library/.../credentials.json cargo run --bin wechat-recv-probe

use wecode_im_bridge::wechat::{ILinkClient, WeChatCredentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("WECODE_WECHAT_CREDS")?;
    let creds: WeChatCredentials = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    println!(
        "[probe] bot_id={} base_url={}",
        creds.ilink_bot_id, creds.base_url
    );
    let client = ILinkClient::new(&creds);
    let mut buf = String::new();
    println!("[probe] polling; send a WeChat message to the bot now ...");
    loop {
        match client.get_updates(&buf).await {
            Ok(resp) => {
                println!(
                    "[probe] ret={} errcode={:?} errmsg={:?} msgs={} buf_len={}",
                    resp.ret,
                    resp.errcode,
                    resp.errmsg,
                    resp.msgs.len(),
                    resp.get_updates_buf.len()
                );
                if !resp.get_updates_buf.is_empty() {
                    buf = resp.get_updates_buf.clone();
                }
                for msg in &resp.msgs {
                    println!(
                        "[probe] MSG type={} state={} from={} items={} text={:?}",
                        msg.message_type,
                        msg.message_state,
                        msg.from_user_id,
                        msg.item_list.len(),
                        wecode_im_bridge::wechat::message_text(msg)
                    );
                    // Try replying to verify the send path too.
                    match client
                        .send_text(
                            &msg.from_user_id,
                            "✅ WeCode 探针收到你的消息",
                            &msg.context_token,
                        )
                        .await
                    {
                        Ok(r) => println!("[probe] reply sent ret={} errmsg={:?}", r.ret, r.errmsg),
                        Err(e) => println!("[probe] reply FAILED: {e}"),
                    }
                }
            }
            Err(e) => {
                println!("[probe] poll error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}
