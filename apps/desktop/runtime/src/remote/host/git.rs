use super::*;

impl RemoteHostRuntime {
    pub(super) fn handle_git_status(&self, envelope: &RemoteEnvelope) {
        let project_id = envelope
            .payload
            .get("projectId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let project_path = envelope.payload.get("projectPath").and_then(Value::as_str);
        let project_store = ProjectStore::new(self.support_dir.clone());
        let project = project_path
            .filter(|path| !path.trim().is_empty())
            .map(|path| (project_id.to_string(), path.to_string()))
            .or_else(|| {
                project_store
                    .projects_snapshot()
                    .into_iter()
                    .find(|project| project.id == project_id)
                    .or_else(|| project_store.projects_snapshot().into_iter().next())
                    .map(|project| (project.id, project.path))
            });
        let Some((project_id, project_path)) = project else {
            self.send_error(envelope, "Unable to load Git status.");
            return;
        };
        let summary = crate::git::GitService::status(&project_path);
        self.broadcast_resource_payload(
            REMOTE_GIT_STATUS,
            REMOTE_RESOURCE_GIT_STATUS,
            envelope.device_id.as_deref(),
            Some(&project_id),
            None,
            remote_git_status_payload(project_id.clone(), project_path, summary),
        );
    }

    /// Generic git mutation (`git.invoke`) → GitService, then reply with
    /// refreshed status (the controller maps it back into a GitSummary).
    pub(super) fn handle_git_invoke(&self, envelope: &RemoteEnvelope) {
        let project_path = envelope
            .payload
            .get("projectPath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if project_path.trim().is_empty() {
            self.send_error(envelope, "Project path is required.");
            return;
        }
        let op = envelope
            .payload
            .get("op")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let args = envelope.payload.get("args").cloned().unwrap_or(Value::Null);
        match crate::git::wire::invoke(project_path.as_str(), op, &args) {
            Ok(_) => {
                let summary = crate::git::GitService::status(project_path.as_str());
                self.send(
                    REMOTE_GIT_STATUS,
                    envelope.device_id.as_deref(),
                    None,
                    remote_git_status_payload(String::new(), project_path, summary),
                );
            }
            Err(error) => self.send_error(envelope, &error),
        }
    }

    /// Generic git read (`git.read`) → `{op, result}`.
    pub(super) fn handle_git_read(&self, envelope: &RemoteEnvelope) {
        let path = envelope
            .payload
            .get("projectPath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let op = envelope
            .payload
            .get("op")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let args = envelope.payload.get("args").cloned().unwrap_or(Value::Null);
        // `stored_state` is a full status payload (needs the project envelope),
        // so it stays host-side; every other read op shares the engine table.
        let result: Result<Value, String> = if op == "stored_state" {
            Ok(remote_git_status_payload(
                String::new(),
                path.to_string(),
                crate::git::GitService::status(path),
            ))
        } else {
            crate::git::wire::read(path, op, &args)
        };
        match result {
            Ok(result) => self.send(
                REMOTE_GIT_READ,
                envelope.device_id.as_deref(),
                None,
                json!({ "op": op, "result": result }),
            ),
            Err(error) => self.send_error(envelope, &error),
        }
    }
}

pub(crate) fn remote_git_status_payload(
    project_id: String,
    project_path: String,
    summary: crate::git::GitSummary,
) -> Value {
    runtime_git::git_status_payload(
        project_id,
        project_path,
        crate::git::wire::wire_status_summary(summary),
    )
}
