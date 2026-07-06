use super::*;

pub(crate) fn remote_file_list(path: Option<&str>, purpose: Option<&str>) -> Value {
    runtime_file::file_list_payload(path, purpose)
}

pub(crate) fn remote_file_read(path: &str) -> Result<Value, String> {
    runtime_file::file_read_payload(path)
}

pub(crate) fn remote_file_write(path: &str, content: &str) -> Result<(), String> {
    runtime_file::file_write(path, content)
}

pub(crate) fn remote_file_rename(path: &str, new_path: &str) -> Result<(), String> {
    runtime_file::file_rename(path, new_path)
}

impl RemoteHostRuntime {
    pub(super) fn handle_file_read(&self, envelope: &RemoteEnvelope) {
        let Some(path) = envelope.payload.get("path").and_then(Value::as_str) else {
            self.send_error(envelope, "File path is required.");
            return;
        };
        match remote_file_read(path) {
            Ok(payload) => self.send(
                REMOTE_FILE_READ,
                envelope.device_id.as_deref(),
                None,
                payload,
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_write(&self, envelope: &RemoteEnvelope) {
        let Some(path) = envelope.payload.get("path").and_then(Value::as_str) else {
            self.send_error(envelope, "File path is required.");
            return;
        };
        let Some(content) = envelope.payload.get("content").and_then(Value::as_str) else {
            self.send_error(envelope, "File content is required.");
            return;
        };
        match remote_file_write(path, content) {
            Ok(()) => self.send(
                REMOTE_FILE_WRITTEN,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_rename(&self, envelope: &RemoteEnvelope) {
        let Some(path) = envelope.payload.get("path").and_then(Value::as_str) else {
            self.send_error(envelope, "File path is required.");
            return;
        };
        let Some(new_path) = envelope.payload.get("newPath").and_then(Value::as_str) else {
            self.send_error(envelope, "New file path is required.");
            return;
        };
        match remote_file_rename(path, new_path) {
            Ok(()) => self.send(
                REMOTE_FILE_RENAMED,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": path, "newPath": new_path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_delete(&self, envelope: &RemoteEnvelope) {
        let Some(path) = envelope.payload.get("path").and_then(Value::as_str) else {
            self.send_error(envelope, "File path is required.");
            return;
        };
        match fs::remove_file(path).or_else(|_| fs::remove_dir_all(path)) {
            Ok(()) => self.send(
                REMOTE_FILE_DELETED,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": path }),
            ),
            Err(error) => self.send_error(envelope, &error.to_string()),
        }
    }

    pub(super) fn handle_file_create_directory(&self, envelope: &RemoteEnvelope) {
        let Some(path) = envelope.payload.get("path").and_then(Value::as_str) else {
            self.send_error(envelope, "Directory path is required.");
            return;
        };
        match runtime_file::file_make_directory(path) {
            Ok(()) => self.send(
                REMOTE_FILE_DIRECTORY_CREATED,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_copy(&self, envelope: &RemoteEnvelope) {
        let path = envelope
            .payload
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let target = envelope
            .payload
            .get("targetDir")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match runtime_file::file_copy(path, target) {
            Ok(new_path) => self.send(
                REMOTE_FILE_COPIED,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": new_path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_move(&self, envelope: &RemoteEnvelope) {
        let path = envelope
            .payload
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let target = envelope
            .payload
            .get("targetDir")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let overwrite = envelope
            .payload
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        match runtime_file::file_move(path, target, overwrite) {
            Ok(new_path) => self.send(
                REMOTE_FILE_MOVED,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": new_path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    pub(super) fn handle_file_write_bytes(&self, envelope: &RemoteEnvelope) {
        use base64::Engine;
        let directory = envelope
            .payload
            .get("directory")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let name = envelope
            .payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let bytes = envelope
            .payload
            .get("bytes")
            .and_then(Value::as_str)
            .and_then(|encoded| {
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .ok()
            })
            .unwrap_or_default();
        match runtime_file::file_write_bytes(directory, name, &bytes) {
            Ok(new_path) => self.send(
                REMOTE_FILE_BYTES_WRITTEN,
                envelope.device_id.as_deref(),
                None,
                json!({ "path": new_path }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }
}
