use super::*;

impl RemoteHostRuntime {
    /// Serve the host's saved SSH profiles (lean, no secrets). The host owns the
    /// profiles, so it just sends its own list as the shared DTO.
    pub(super) fn handle_ssh_list(&self, envelope: &RemoteEnvelope) {
        self.send_ssh_list(envelope.device_id.as_deref());
    }

    /// Reply with the saved SSH profiles as secret-free summaries.
    pub(super) fn send_ssh_list(&self, device_id: Option<&str>) {
        let service =
            crate::ssh::SSHService::new(self.support_dir.clone(), std::path::PathBuf::new());
        let profiles: Vec<wecode_protocol::RemoteSshProfileSummary> = service
            .summary()
            .profiles
            .into_iter()
            .map(|profile| wecode_protocol::RemoteSshProfileSummary {
                id: profile.id,
                name: profile.name,
                endpoint: profile.endpoint,
                credential: profile.credential_kind,
            })
            .collect();
        self.send(
            REMOTE_SSH_LIST_RESULT,
            device_id,
            None,
            json!({ "profiles": profiles }),
        );
    }

    /// Add or update a saved SSH profile, then reply with the refreshed list.
    pub(super) fn handle_ssh_upsert(&self, envelope: &RemoteEnvelope) {
        let request: crate::ssh::SSHProfileUpsertRequest =
            match serde_json::from_value(envelope.payload.clone()) {
                Ok(request) => request,
                Err(error) => {
                    self.send_error(envelope, &format!("Invalid SSH profile: {error}"));
                    return;
                }
            };
        let store = crate::ssh::SSHStore::from_support_dir(self.support_dir.clone());
        match store.upsert(request) {
            Ok(_) => self.send_ssh_list(envelope.device_id.as_deref()),
            Err(error) => self.send_error(envelope, &error),
        }
    }

    /// Remove a saved SSH profile by id, then reply with the refreshed list.
    pub(super) fn handle_ssh_remove(&self, envelope: &RemoteEnvelope) {
        let id = envelope
            .payload
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if id.trim().is_empty() {
            self.send_error(envelope, "SSH profile id is required.");
            return;
        }
        let store = crate::ssh::SSHStore::from_support_dir(self.support_dir.clone());
        match store.delete(id) {
            Ok(_) => self.send_ssh_list(envelope.device_id.as_deref()),
            Err(error) => self.send_error(envelope, &error),
        }
    }
}
