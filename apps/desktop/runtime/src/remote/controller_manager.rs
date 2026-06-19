//! Pools live `RemoteController` connections to paired hosts, keyed by the
//! device id we minted during pairing. Lazily connects from the saved-host
//! store on first use, and bridges the async controller API into the
//! synchronous `RuntimeService` domain methods via `async_runtime::block_on`.

use super::controller::{parse_pairing_ticket, RemoteController};
use super::controller_store::{RemoteControllerStore, SavedRemoteHost};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct RemoteControllerManager {
    store: RemoteControllerStore,
    connections: Mutex<HashMap<String, Arc<RemoteController>>>,
}

impl RemoteControllerManager {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            store: RemoteControllerStore::new(support_dir),
            connections: Mutex::new(HashMap::new()),
        }
    }

    pub fn saved_hosts(&self) -> Vec<SavedRemoteHost> {
        self.store.list()
    }

    /// Pair with a host from a pasted ticket, persist it, and cache the live
    /// connection so the new device is immediately usable.
    pub fn pair(&self, ticket_input: &str, device_name: &str) -> Result<SavedRemoteHost, String> {
        let ticket = parse_pairing_ticket(ticket_input)?;
        let (controller, saved) =
            crate::async_runtime::block_on(RemoteController::pair(&ticket, device_name))?;
        self.store.upsert(saved.clone())?;
        if let Ok(mut connections) = self.connections.lock() {
            connections.insert(saved.device_id.clone(), Arc::new(controller));
        }
        Ok(saved)
    }

    /// Get (or lazily establish) the controller for a paired device.
    pub fn controller_for(&self, device_id: &str) -> Result<Arc<RemoteController>, String> {
        if let Ok(connections) = self.connections.lock() {
            if let Some(controller) = connections.get(device_id).cloned() {
                return Ok(controller);
            }
        }
        let host = self
            .store
            .find(device_id)
            .ok_or_else(|| format!("No saved remote host for device {device_id}."))?;
        let controller = Arc::new(crate::async_runtime::block_on(
            RemoteController::connect_saved(&host),
        )?);
        if let Ok(mut connections) = self.connections.lock() {
            connections.insert(device_id.to_string(), Arc::clone(&controller));
        }
        Ok(controller)
    }

    /// Drop a paired host and any live connection to it.
    pub fn forget(&self, device_id: &str) -> Result<Vec<SavedRemoteHost>, String> {
        if let Ok(mut connections) = self.connections.lock() {
            connections.remove(device_id);
        }
        self.store.remove(device_id)
    }
}
