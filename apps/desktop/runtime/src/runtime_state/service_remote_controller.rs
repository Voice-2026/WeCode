// Desktop-as-controller domain: pair with remote hosts and drive their domains
// over the controller transport. Browsing/creating directories on a host backs
// the add-project remote flow; routing a hosted project's other domains builds
// on `controller_for`.

impl RuntimeService {
    /// Pair with a remote host from a pasted `codux://pair` ticket, persist it,
    /// and cache the live connection.
    pub fn pair_remote_host(
        &self,
        ticket: &str,
        device_name: &str,
    ) -> Result<crate::remote::SavedRemoteHost, String> {
        self.remote_controllers.pair(ticket, device_name)
    }

    /// Every host this desktop has paired with and can reconnect to.
    pub fn saved_remote_hosts(&self) -> Vec<crate::remote::SavedRemoteHost> {
        self.remote_controllers.saved_hosts()
    }

    /// Drop a paired host and any live connection to it.
    pub fn forget_remote_host(
        &self,
        device_id: &str,
    ) -> Result<Vec<crate::remote::SavedRemoteHost>, String> {
        self.remote_controllers.forget(device_id)
    }

    /// List a directory on a remote host (for the add-project remote browser),
    /// parsed into a typed listing so the UI never touches the wire JSON.
    pub fn remote_browse_directory(
        &self,
        device_id: &str,
        path: Option<&str>,
    ) -> Result<crate::remote::RemoteDirectoryListing, String> {
        self.remote_controllers
            .controller_for(device_id)?
            .browse_directory(path)
    }

    /// Create a directory on a remote host (for the add-project remote flow).
    pub fn remote_create_directory(
        &self,
        device_id: &str,
        path: &str,
    ) -> Result<serde_json::Value, String> {
        self.remote_controllers
            .controller_for(device_id)?
            .create_directory(path)
    }

    /// Fetch a remote host's identity/capabilities (also a reachability check).
    pub fn remote_host_info(&self, device_id: &str) -> Result<serde_json::Value, String> {
        self.remote_controllers.controller_for(device_id)?.host_info()
    }

    /// The device hosting the project at `project_path`, if it is a remote
    /// project. Used to route a project's domains over the controller.
    pub(crate) fn host_device_for_project_path(&self, project_path: &str) -> Option<String> {
        crate::project_store::ProjectStore::new(self.support_dir.clone())
            .projects_snapshot()
            .into_iter()
            .find(|project| project.path == project_path)
            .and_then(|project| project.host_device_id)
    }

    /// List a directory of a remote-hosted project as the file panel's
    /// `FileEntry`s, mapped from the host's `file.list` payload (capped to 80 to
    /// match the local loader).
    pub(crate) fn remote_project_files(
        &self,
        device_id: &str,
        project_path: &str,
        directory_path: Option<&str>,
    ) -> Result<Vec<FileEntry>, String> {
        // The UI works in project-relative paths; the host lists by absolute.
        let listing_dir = remote_absolute_path(project_path, directory_path);
        let value = self
            .remote_controllers
            .controller_for(device_id)?
            .file_list(Some(&listing_dir), Some("projectFiles"))?;
        Ok(value
            .get("entries")
            .and_then(|entries| entries.as_array())
            .map(|entries| {
                entries
                    .iter()
                    .take(80)
                    .map(|entry| remote_file_entry(project_path, entry))
                    .collect()
            })
            .unwrap_or_default())
    }
}

/// Resolve a project-relative path (as the UI uses) to the host's absolute path.
/// An empty/`None` relative path means the project root.
pub(crate) fn remote_absolute_path(project_path: &str, relative: Option<&str>) -> String {
    let root = project_path.trim_end_matches('/');
    match relative.map(str::trim).filter(|value| !value.is_empty()) {
        Some(relative) => format!("{root}/{}", relative.trim_start_matches('/')),
        None => root.to_string(),
    }
}

/// Build the file panel's `FileEntry` from one host `file.list` entry, computing
/// the project-relative path the panel expects.
fn remote_file_entry(project_path: &str, entry: &serde_json::Value) -> FileEntry {
    let path = entry
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let is_directory = entry
        .get("isDirectory")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let relative_path = path
        .strip_prefix(project_path)
        .unwrap_or(path)
        .trim_start_matches('/')
        .to_string();
    FileEntry {
        name: entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        relative_path,
        kind: if is_directory {
            FileKind::Directory
        } else {
            FileKind::File
        },
        size: entry
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
    }
}
