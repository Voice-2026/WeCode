use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default)]
pub(in crate::app) struct PetCustomInstallEvent {
    pub(in crate::app) revision: u64,
    pub(in crate::app) custom_pet_id: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::app) struct PetUpdateEvent {
    pub(in crate::app) revision: u64,
}

#[derive(Clone, Debug, Default)]
pub(in crate::app) struct SettingsUpdateEvent {
    pub(in crate::app) revision: u64,
}

#[derive(Clone, Debug, Default)]
pub(in crate::app) struct SshUpdateEvent {
    pub(in crate::app) revision: u64,
}

#[derive(Clone, Debug, Default)]
pub(in crate::app) struct MemoryUpdateEvent {
    pub(in crate::app) revision: u64,
}

static PET_CUSTOM_INSTALL_EVENT: OnceLock<Mutex<PetCustomInstallEvent>> = OnceLock::new();
static PET_UPDATE_EVENT: OnceLock<Mutex<PetUpdateEvent>> = OnceLock::new();
static SETTINGS_UPDATE_EVENT: OnceLock<Mutex<SettingsUpdateEvent>> = OnceLock::new();
static SSH_UPDATE_EVENT: OnceLock<Mutex<SshUpdateEvent>> = OnceLock::new();
static MEMORY_UPDATE_EVENT: OnceLock<Mutex<MemoryUpdateEvent>> = OnceLock::new();

fn pet_custom_install_event() -> &'static Mutex<PetCustomInstallEvent> {
    PET_CUSTOM_INSTALL_EVENT.get_or_init(|| Mutex::new(PetCustomInstallEvent::default()))
}

pub(in crate::app) fn current_pet_custom_install_event() -> PetCustomInstallEvent {
    pet_custom_install_event()
        .lock()
        .map(|event| event.clone())
        .unwrap_or_default()
}

pub(in crate::app) fn publish_pet_custom_install(custom_pet_id: String) -> u64 {
    let Ok(mut event) = pet_custom_install_event().lock() else {
        return 0;
    };
    event.revision = event.revision.saturating_add(1);
    event.custom_pet_id = Some(custom_pet_id);
    event.revision
}

fn pet_update_event() -> &'static Mutex<PetUpdateEvent> {
    PET_UPDATE_EVENT.get_or_init(|| Mutex::new(PetUpdateEvent::default()))
}

pub(in crate::app) fn current_pet_update_event() -> PetUpdateEvent {
    pet_update_event()
        .lock()
        .map(|event| event.clone())
        .unwrap_or_default()
}

pub(in crate::app) fn publish_pet_update() -> u64 {
    let Ok(mut event) = pet_update_event().lock() else {
        return 0;
    };
    event.revision = event.revision.saturating_add(1);
    event.revision
}

fn settings_update_event() -> &'static Mutex<SettingsUpdateEvent> {
    SETTINGS_UPDATE_EVENT.get_or_init(|| Mutex::new(SettingsUpdateEvent::default()))
}

pub(in crate::app) fn current_settings_update_event() -> SettingsUpdateEvent {
    settings_update_event()
        .lock()
        .map(|event| event.clone())
        .unwrap_or_default()
}

pub(in crate::app) fn publish_settings_update() -> u64 {
    let Ok(mut event) = settings_update_event().lock() else {
        return 0;
    };
    event.revision = event.revision.saturating_add(1);
    event.revision
}

fn ssh_update_event() -> &'static Mutex<SshUpdateEvent> {
    SSH_UPDATE_EVENT.get_or_init(|| Mutex::new(SshUpdateEvent::default()))
}

pub(in crate::app) fn current_ssh_update_event() -> SshUpdateEvent {
    ssh_update_event()
        .lock()
        .map(|event| event.clone())
        .unwrap_or_default()
}

pub(in crate::app) fn publish_ssh_update() -> u64 {
    let Ok(mut event) = ssh_update_event().lock() else {
        return 0;
    };
    event.revision = event.revision.saturating_add(1);
    event.revision
}

fn memory_update_event() -> &'static Mutex<MemoryUpdateEvent> {
    MEMORY_UPDATE_EVENT.get_or_init(|| Mutex::new(MemoryUpdateEvent::default()))
}

pub(in crate::app) fn current_memory_update_event() -> MemoryUpdateEvent {
    memory_update_event()
        .lock()
        .map(|event| event.clone())
        .unwrap_or_default()
}

pub(in crate::app) fn publish_memory_update() -> u64 {
    let Ok(mut event) = memory_update_event().lock() else {
        return 0;
    };
    event.revision = event.revision.saturating_add(1);
    event.revision
}
