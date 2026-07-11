impl RuntimeService {
    pub fn ai_session_metadata(&self) -> HashMap<String, AISessionMetadata> {
        AISessionMetadataService::new(self.support_dir.clone()).list()
    }

    pub fn set_ai_session_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<AISessionMetadata, String> {
        AISessionMetadataService::new(self.support_dir.clone()).set_pinned(session_id, pinned)
    }

    pub fn set_ai_session_retention(
        &self,
        session_id: &str,
        retention: &str,
    ) -> Result<AISessionMetadata, String> {
        AISessionMetadataService::new(self.support_dir.clone())
            .set_retention(session_id, retention)
    }

    pub fn set_ai_session_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<AISessionMetadata, String> {
        AISessionMetadataService::new(self.support_dir.clone()).set_archived(session_id, archived)
    }
}
