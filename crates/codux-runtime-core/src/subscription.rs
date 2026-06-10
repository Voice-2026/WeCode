use codux_protocol::{
    REMOTE_RESOURCE_SUBSCRIBE, REMOTE_RESOURCE_UNSUBSCRIBE, RemoteEnvelope,
    RemoteResourceSubscriptionTarget, RemoteResourceSubscriptions,
};
use std::collections::HashSet;

#[derive(Default)]
pub struct RuntimeSubscriptionRouter {
    subscriptions: RemoteResourceSubscriptions,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSubscriptionChange {
    pub device_id: String,
    pub resource: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub baseline: bool,
}

impl RuntimeSubscriptionRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe_envelope(
        &self,
        envelope: &RemoteEnvelope,
    ) -> Result<RuntimeSubscriptionChange, String> {
        if envelope.kind != REMOTE_RESOURCE_SUBSCRIBE {
            return Err("Expected resource.subscribe envelope.".to_string());
        }
        let device_id = clean_device_id(envelope.device_id.as_deref())?;
        let target = RemoteResourceSubscriptionTarget::from_payload(
            envelope.session_id.as_deref(),
            &envelope.payload,
        )?;
        self.subscriptions.subscribe(
            &target.resource,
            target.project_id.as_deref(),
            target.session_id.as_deref(),
            &device_id,
        );
        Ok(RuntimeSubscriptionChange {
            device_id,
            resource: target.resource,
            project_id: target.project_id,
            session_id: target.session_id,
            baseline: target.baseline,
        })
    }

    pub fn unsubscribe_envelope(
        &self,
        envelope: &RemoteEnvelope,
    ) -> Result<RuntimeSubscriptionChange, String> {
        if envelope.kind != REMOTE_RESOURCE_UNSUBSCRIBE {
            return Err("Expected resource.unsubscribe envelope.".to_string());
        }
        let device_id = clean_device_id(envelope.device_id.as_deref())?;
        let target = RemoteResourceSubscriptionTarget::from_payload(
            envelope.session_id.as_deref(),
            &envelope.payload,
        )?;
        self.subscriptions.unsubscribe(
            &target.resource,
            target.project_id.as_deref(),
            target.session_id.as_deref(),
            &device_id,
        );
        Ok(RuntimeSubscriptionChange {
            device_id,
            resource: target.resource,
            project_id: target.project_id,
            session_id: target.session_id,
            baseline: target.baseline,
        })
    }

    pub fn remove_device(&self, device_id: &str) {
        self.subscriptions.remove_device(device_id);
    }

    pub fn remove_project(&self, project_id: &str) {
        self.subscriptions.remove_project(project_id);
    }

    pub fn remove_session(&self, session_id: &str) {
        self.subscriptions.remove_session(session_id);
    }

    pub fn clear(&self) {
        self.subscriptions.clear();
    }

    pub fn devices_for(
        &self,
        resource: &str,
        project_id: Option<&str>,
        session_id: Option<&str>,
    ) -> HashSet<String> {
        self.subscriptions
            .devices_for(resource, project_id, session_id)
    }
}

fn clean_device_id(device_id: Option<&str>) -> Result<String, String> {
    device_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Device id is required.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codux_protocol::{
        REMOTE_PROJECT_LIST, REMOTE_RESOURCE_GIT_STATUS, REMOTE_RESOURCE_SUBSCRIBE,
        REMOTE_RESOURCE_TERMINALS, REMOTE_RESOURCE_UNSUBSCRIBE,
    };
    use serde_json::json;

    #[test]
    fn subscribe_and_unsubscribe_envelopes_drive_resource_targets() {
        let router = RuntimeSubscriptionRouter::new();
        let subscribe = RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("device-1".to_string()),
            session_id: None,
            seq: None,
            payload: json!({
                "resource": REMOTE_RESOURCE_GIT_STATUS,
                "projectId": "project-1",
                "baseline": true,
            }),
        };

        let change = router.subscribe_envelope(&subscribe).unwrap();
        assert_eq!(change.device_id, "device-1");
        assert_eq!(change.resource, REMOTE_RESOURCE_GIT_STATUS);
        assert_eq!(change.project_id.as_deref(), Some("project-1"));
        assert!(change.baseline);
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_GIT_STATUS, Some("project-1"), None)
                .contains("device-1")
        );

        let unsubscribe = RemoteEnvelope {
            kind: REMOTE_RESOURCE_UNSUBSCRIBE.to_string(),
            payload: subscribe.payload.clone(),
            ..subscribe
        };
        router.unsubscribe_envelope(&unsubscribe).unwrap();
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_GIT_STATUS, Some("project-1"), None)
                .is_empty()
        );
    }

    #[test]
    fn remove_device_and_scope_clear_subscriptions() {
        let router = RuntimeSubscriptionRouter::new();
        router
            .subscriptions
            .subscribe(REMOTE_RESOURCE_GIT_STATUS, Some("project-1"), None, "a");
        router
            .subscriptions
            .subscribe(REMOTE_RESOURCE_GIT_STATUS, Some("project-2"), None, "b");

        router.remove_project("project-1");
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_GIT_STATUS, Some("project-1"), None)
                .is_empty()
        );
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_GIT_STATUS, Some("project-2"), None)
                .contains("b")
        );

        router.remove_device("b");
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_GIT_STATUS, Some("project-2"), None)
                .is_empty()
        );
    }

    #[test]
    fn rejects_wrong_envelope_kind_and_missing_device() {
        let router = RuntimeSubscriptionRouter::new();
        let wrong_kind = RemoteEnvelope {
            kind: REMOTE_PROJECT_LIST.to_string(),
            device_id: Some("device-1".to_string()),
            session_id: None,
            seq: None,
            payload: json!({
                "resource": REMOTE_RESOURCE_GIT_STATUS,
                "projectId": "project-1",
            }),
        };

        assert_eq!(
            router.subscribe_envelope(&wrong_kind).unwrap_err(),
            "Expected resource.subscribe envelope."
        );

        let missing_device = RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("  ".to_string()),
            ..wrong_kind
        };
        assert_eq!(
            router.subscribe_envelope(&missing_device).unwrap_err(),
            "Device id is required."
        );
    }

    #[test]
    fn routes_session_scoped_terminal_subscriptions() {
        let router = RuntimeSubscriptionRouter::new();
        let subscribe = RemoteEnvelope {
            kind: REMOTE_RESOURCE_SUBSCRIBE.to_string(),
            device_id: Some("device-1".to_string()),
            session_id: Some("session-1".to_string()),
            seq: None,
            payload: json!({
                "resource": REMOTE_RESOURCE_TERMINALS,
                "baseline": true,
            }),
        };

        let change = router.subscribe_envelope(&subscribe).unwrap();
        assert_eq!(change.session_id.as_deref(), Some("session-1"));
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_TERMINALS, None, Some("session-1"))
                .contains("device-1")
        );

        router.remove_session("session-1");
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_TERMINALS, None, Some("session-1"))
                .is_empty()
        );

        router.subscribe_envelope(&subscribe).unwrap();
        router.clear();
        assert!(
            router
                .devices_for(REMOTE_RESOURCE_TERMINALS, None, Some("session-1"))
                .is_empty()
        );
    }
}
