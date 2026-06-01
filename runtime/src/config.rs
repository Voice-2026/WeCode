use parking_lot::{Mutex, RwLock};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
};

static CONFIG_STORES: OnceLock<Mutex<HashMap<PathBuf, Arc<ConfigStore>>>> = OnceLock::new();

pub struct ConfigStore {
    path: PathBuf,
    snapshot: Arc<RwLock<Map<String, Value>>>,
    write_tx: flume::Sender<()>,
}

impl ConfigStore {
    pub fn for_support_dir(support_dir: impl Into<PathBuf>) -> Arc<Self> {
        Self::for_file(state_file_path(support_dir))
    }

    pub fn for_settings_dir(support_dir: impl Into<PathBuf>) -> Arc<Self> {
        Self::for_file(settings_file_path(support_dir))
    }

    pub fn for_file(path: impl Into<PathBuf>) -> Arc<Self> {
        let path = path.into();
        let stores = CONFIG_STORES.get_or_init(|| Mutex::new(HashMap::new()));
        let mut stores = stores.lock();
        if let Some(store) = stores.get(&path) {
            return store.clone();
        }

        let store = Self::load(path.clone());
        stores.insert(path, store.clone());
        store
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        self.snapshot.read().clone()
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.snapshot.read().get(key).cloned()
    }

    pub fn get_as<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get(key)
            .and_then(|value| serde_json::from_value::<T>(value).ok())
    }

    pub fn get_path(&self, path: &[&str]) -> Option<Value> {
        let snapshot = self.snapshot.read();
        get_path_value(&snapshot, path).cloned()
    }

    pub fn get_path_as<T: DeserializeOwned>(&self, path: &[&str]) -> Option<T> {
        self.get_path(path)
            .and_then(|value| serde_json::from_value::<T>(value).ok())
    }

    pub fn set(&self, key: impl Into<String>, value: Value) -> Result<(), String> {
        self.update(|snapshot| {
            snapshot.insert(key.into(), value);
            Ok(())
        })
    }

    pub fn set_as<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<(), String> {
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        self.set(key, value)
    }

    pub fn set_path(&self, path: &[&str], value: Value) -> Result<(), String> {
        if path.is_empty() {
            return Err("config path is empty.".to_string());
        }
        self.update(|snapshot| {
            set_path_value(snapshot, path, value)?;
            Ok(())
        })
    }

    pub fn del(&self, key: &str) -> Result<Option<Value>, String> {
        self.update(|snapshot| Ok(snapshot.remove(key)))
    }

    pub fn del_path(&self, path: &[&str]) -> Result<Option<Value>, String> {
        if path.is_empty() {
            return Err("config path is empty.".to_string());
        }
        self.update(|snapshot| Ok(remove_path_value(snapshot, path)))
    }

    pub fn save_snapshot(&self, snapshot: &Map<String, Value>) -> Result<(), String> {
        *self.snapshot.write() = snapshot.clone();
        self.schedule_write()
    }

    pub fn update<R>(
        &self,
        update: impl FnOnce(&mut Map<String, Value>) -> Result<R, String>,
    ) -> Result<R, String> {
        let result = {
            let mut snapshot = self.snapshot.write();
            update(&mut snapshot)?
        };
        self.schedule_write()?;
        Ok(result)
    }

    fn load(path: PathBuf) -> Arc<Self> {
        let initial = read_snapshot(&path);
        let snapshot = Arc::new(RwLock::new(initial));
        let (write_tx, write_rx) = flume::bounded::<()>(1);
        let writer_snapshot = snapshot.clone();
        let writer_path = path.clone();
        thread::Builder::new()
            .name("codux-state-json-writer".to_string())
            .spawn(move || {
                while write_rx.recv().is_ok() {
                    while write_rx.recv_timeout(Duration::from_millis(80)).is_ok() {}
                    let snapshot = writer_snapshot.read().clone();
                    if let Err(error) = write_snapshot(&writer_path, &snapshot) {
                        crate::runtime_trace::runtime_trace(
                            "config",
                            &format!(
                                "failed to write {}: {error}",
                                writer_path.display()
                            ),
                        );
                    }
                }
            })
            .expect("spawn state json writer");

        Arc::new(Self {
            path,
            snapshot,
            write_tx,
        })
    }

    fn schedule_write(&self) -> Result<(), String> {
        match self.write_tx.try_send(()) {
            Ok(()) | Err(flume::TrySendError::Full(_)) => Ok(()),
            Err(flume::TrySendError::Disconnected(_)) => {
                let snapshot = self.snapshot.read().clone();
                write_snapshot(&self.path, &snapshot)
            }
        }
    }
}

pub fn state_file_path(support_dir: impl Into<PathBuf>) -> PathBuf {
    support_dir.into().join("state.json")
}

pub fn settings_file_path(support_dir: impl Into<PathBuf>) -> PathBuf {
    support_dir.into().join("settings.json")
}

pub fn raw_state_snapshot(path: &Path) -> Map<String, Value> {
    ConfigStore::for_file(path.to_path_buf()).snapshot()
}

pub fn save_raw_state_snapshot(
    path: &Path,
    snapshot: &Map<String, Value>,
) -> Result<(), String> {
    ConfigStore::for_file(path.to_path_buf()).save_snapshot(snapshot)
}

fn get_path_value<'a>(snapshot: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let mut value = snapshot.get(*first)?;
    for key in rest {
        value = value.as_object()?.get(*key)?;
    }
    Some(value)
}

fn set_path_value(
    snapshot: &mut Map<String, Value>,
    path: &[&str],
    value: Value,
) -> Result<(), String> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| "config path is empty.".to_string())?;
    let mut current = snapshot;
    for key in parents {
        if !matches!(current.get(*key), Some(Value::Object(_))) {
            current.insert((*key).to_string(), Value::Object(Map::new()));
        }
        current = current
            .get_mut(*key)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("{key} is not an object."))?;
    }
    current.insert((*last).to_string(), value);
    Ok(())
}

fn remove_path_value(snapshot: &mut Map<String, Value>, path: &[&str]) -> Option<Value> {
    let (last, parents) = path.split_last()?;
    let mut current = snapshot;
    for key in parents {
        current = current.get_mut(*key)?.as_object_mut()?;
    }
    current.remove(*last)
}

fn read_snapshot(path: &Path) -> Map<String, Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn write_snapshot(path: &Path, snapshot: &Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?;
    fs::write(path, format!("{content}\n")).map_err(|error| error.to_string())
}
