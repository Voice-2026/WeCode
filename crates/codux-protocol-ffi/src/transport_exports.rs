use crate::common::{
    FfiControllerTransport, c_to_string, clear_last_error, controller_transport_config_from_json,
    controller_transport_ref, panic_payload_message, push_transport_event, set_last_error,
    string_to_c,
};
use codux_remote_transport::{
    RemoteControllerTransportConfig, RemoteTransport, RemoteTransportFactory,
    preferred_controller_transport_kind, preferred_pairing_transport_kind,
    remote_client_websocket_url, remote_pairing_code_url, remote_pairing_ticket_url,
    remote_pairing_websocket_url, remote_relay_url_for_preset, remote_server_url, remote_stun_urls,
};
use serde_json::json;
use std::collections::VecDeque;
use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_server_url(base: *const c_char) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    string_to_c(remote_server_url(&base))
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_relay_url_for_preset(
    preset: *const c_char,
    custom_url: *const c_char,
) -> *mut c_char {
    let Some(preset) = c_to_string(preset) else {
        return ptr::null_mut();
    };
    let custom_url = c_to_string(custom_url).unwrap_or_default();
    string_to_c(remote_relay_url_for_preset(&preset, &custom_url))
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_pairing_ticket_url(
    base: *const c_char,
    ticket: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(ticket) = c_to_string(ticket) else {
        return ptr::null_mut();
    };
    string_to_c(remote_pairing_ticket_url(&base, &ticket).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_pairing_code_url(
    base: *const c_char,
    code: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(code) = c_to_string(code) else {
        return ptr::null_mut();
    };
    string_to_c(remote_pairing_code_url(&base, &code).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_pairing_websocket_url(
    base: *const c_char,
    host_id: *const c_char,
    device_public_key: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(host_id) = c_to_string(host_id) else {
        return ptr::null_mut();
    };
    let Some(device_public_key) = c_to_string(device_public_key) else {
        return ptr::null_mut();
    };
    string_to_c(
        remote_pairing_websocket_url(&base, &host_id, &device_public_key).unwrap_or_default(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_client_websocket_url(
    base: *const c_char,
    host_id: *const c_char,
    device_id: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(host_id) = c_to_string(host_id) else {
        return ptr::null_mut();
    };
    let Some(device_id) = c_to_string(device_id) else {
        return ptr::null_mut();
    };
    let token = c_to_string(token).filter(|value| !value.trim().is_empty());
    string_to_c(
        remote_client_websocket_url(&base, &host_id, &device_id, token.as_deref())
            .unwrap_or_default(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_default_ice_servers_json() -> *mut c_char {
    string_to_c(json!([{ "urls": remote_stun_urls() }]).to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_preferred_kind(
    transports_json: *const c_char,
    pairing: bool,
) -> *mut c_char {
    let Some(transports_json) = c_to_string(transports_json) else {
        return ptr::null_mut();
    };
    let transports = serde_json::from_str::<serde_json::Value>(&transports_json)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let pairs = transports
        .iter()
        .map(|item| {
            (
                item.get("kind")
                    .or_else(|| item.get("transport"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                item.get("url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    let kind = if pairing {
        preferred_pairing_transport_kind(pairs.iter().copied())
    } else {
        preferred_controller_transport_kind(pairs.iter().copied())
    };
    string_to_c(kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_config_summary_json(
    config_json: *const c_char,
) -> *mut c_char {
    let Some(config_json) = c_to_string(config_json) else {
        return ptr::null_mut();
    };
    let Ok(config) = controller_transport_config_from_json(&config_json) else {
        return ptr::null_mut();
    };
    let preferred = preferred_controller_transport_kind(
        config
            .transports
            .iter()
            .map(|candidate| (candidate.kind.as_str(), candidate.url.as_str())),
    );
    string_to_c(
        json!({
            "serverUrl": remote_server_url(&config.server_url),
            "hostId": config.host_id,
            "deviceId": config.device_id,
            "transportKind": preferred,
            "transportCount": config.transports.len(),
            "stunCount": config.stun_urls.len(),
        })
        .to_string(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_connect_json(
    config_json: *const c_char,
) -> *mut FfiControllerTransport {
    clear_last_error();
    match catch_unwind(AssertUnwindSafe(|| {
        controller_transport_connect_json_inner(config_json)
    })) {
        Ok(transport) => transport,
        Err(payload) => {
            set_last_error(format!(
                "controller transport connect panicked: {}",
                panic_payload_message(payload.as_ref())
            ));
            ptr::null_mut()
        }
    }
}

fn controller_transport_connect_json_inner(
    config_json: *const c_char,
) -> *mut FfiControllerTransport {
    let Some(config_json) = c_to_string(config_json) else {
        set_last_error("missing controller transport config json");
        return ptr::null_mut();
    };
    let config = match controller_transport_config_from_json(&config_json) {
        Ok(config) => config,
        Err(error) => {
            set_last_error(format!("invalid controller transport config: {error}"));
            return ptr::null_mut();
        }
    };
    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            set_last_error(format!(
                "failed to create controller transport runtime: {error}"
            ));
            return ptr::null_mut();
        }
    };
    let runtime = Arc::new(runtime);
    let events = Arc::new(Mutex::new(VecDeque::new()));
    push_transport_event(
        &events,
        json!({
            "kind": "state",
            "state": "connecting",
        }),
    );

    let connect_result =
        runtime.block_on(connect_controller_transport(&config, Arc::clone(&events)));

    match connect_result {
        Ok(transport) => Box::into_raw(Box::new(FfiControllerTransport {
            transport: Mutex::new(transport),
            events,
            runtime,
        })),
        Err(error) => {
            set_last_error(format!("failed to connect controller transport: {error}"));
            ptr::null_mut()
        }
    }
}

async fn connect_controller_transport(
    config: &RemoteControllerTransportConfig,
    events: Arc<Mutex<VecDeque<String>>>,
) -> Result<Arc<dyn RemoteTransport>, String> {
    let events_for_message = Arc::clone(&events);
    let events_for_state = Arc::clone(&events);
    let events_for_log = Arc::clone(&events);
    RemoteTransportFactory::connect_controller(
        config,
        Arc::new(move |device_id, data| {
            let text = String::from_utf8(data).unwrap_or_default();
            push_transport_event(
                &events_for_message,
                json!({
                    "kind": "message",
                    "deviceId": device_id,
                    "data": text,
                }),
            );
        }),
        Arc::new(move |device_id, state| {
            push_transport_event(
                &events_for_state,
                json!({
                    "kind": "state",
                    "deviceId": device_id,
                    "state": state,
                }),
            );
        }),
        Some(Arc::new(move |message| {
            push_transport_event(
                &events_for_log,
                json!({
                    "kind": "log",
                    "message": message,
                }),
            );
        })),
    )
    .await
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_send_json(
    transport: *mut FfiControllerTransport,
    envelope_json: *const c_char,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(transport) = controller_transport_ref(transport) else {
            return false;
        };
        let Some(envelope_json) = c_to_string(envelope_json) else {
            return false;
        };
        transport
            .transport
            .lock()
            .ok()
            .map(|transport| transport.send(envelope_json.into_bytes(), None))
            .unwrap_or(false)
    }))
    .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_report_ping_timeout(
    transport: *mut FfiControllerTransport,
    path: *const c_char,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(transport) = controller_transport_ref(transport) else {
            return false;
        };
        let path = c_to_string(path).unwrap_or_default();
        if path != "direct" {
            return false;
        }
        transport
            .transport
            .lock()
            .ok()
            .map(|transport| transport.mark_direct_unhealthy())
            .unwrap_or(false)
    }))
    .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_probe_preferred_route(
    transport: *mut FfiControllerTransport,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(transport) = controller_transport_ref(transport) else {
            return false;
        };
        transport
            .transport
            .lock()
            .ok()
            .map(|transport| transport.probe_preferred_route())
            .unwrap_or(false)
    }))
    .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_poll_event_json(
    transport: *mut FfiControllerTransport,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(transport) = controller_transport_ref(transport) else {
            return ptr::null_mut();
        };
        let event = transport
            .events
            .lock()
            .ok()
            .and_then(|mut events| events.pop_front());
        match event {
            Some(event) => string_to_c(event),
            None => ptr::null_mut(),
        }
    }))
    .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_controller_transport_close(transport: *mut FfiControllerTransport) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if transport.is_null() {
            return;
        }
        let transport = unsafe { Box::from_raw(transport) };
        let runtime = Arc::clone(&transport.runtime);
        runtime.block_on(async {
            let current = transport
                .transport
                .lock()
                .ok()
                .map(|transport| Arc::clone(&transport));
            if let Some(current) = current {
                current.shutdown().await;
            }
        });
    }));
}
