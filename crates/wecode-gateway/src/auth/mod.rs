mod file;
mod refresh;
mod sqlite;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use crate::config::{CredentialSource, GatewayConfig};
use crate::error::GatewayError;
use crate::util::machine_fingerprint;

pub fn kiro_app_credentials_path(path: Option<PathBuf>) -> PathBuf {
    file::resolve_kiro_app_path(path)
}

/// Which refresh flow a set of credentials uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// Kiro Desktop auth: prod.{region}.auth.desktop.kiro.dev/refreshToken.
    KiroDesktop,
    /// AWS SSO OIDC (kiro-cli): oidc.{region}.amazonaws.com/token.
    AwsSsoOidc,
}

/// The credential material and refresh state. Mutated in place on refresh.
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub profile_arn: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<Vec<String>>,

    // AWS SSO OIDC device registration.
    pub client_id: Option<String>,
    pub client_secret: Option<String>,

    /// SSO region (for OIDC refresh); may differ from API region.
    pub sso_region: Option<String>,
    /// API region auto-detected from credentials (profile ARN / region field).
    pub detected_api_region: Option<String>,

    /// Which kiro-cli SQLite key the token came from (for write-back).
    pub sqlite_token_key: Option<String>,
}

impl Credentials {
    fn auth_type(&self) -> AuthType {
        if self.client_id.is_some() && self.client_secret.is_some() {
            AuthType::AwsSsoOidc
        } else {
            AuthType::KiroDesktop
        }
    }
}

/// Manages the Kiro token lifecycle for a single account.
pub struct KiroAuth {
    source: CredentialSource,
    creds: Mutex<Credentials>,
    fingerprint: String,
    /// SSO region used for the OIDC refresh endpoint.
    sso_region: String,
    /// Final API region for runtime.{region}.kiro.dev.
    api_region: String,
    api_host: String,
    token_refresh_threshold_secs: i64,
    http: reqwest::Client,
}

impl KiroAuth {
    pub fn from_config(
        config: &GatewayConfig,
        http: reqwest::Client,
    ) -> Result<Arc<Self>, GatewayError> {
        Self::new(
            config.credentials.clone(),
            config.region.clone(),
            config.api_region.clone(),
            config.token_refresh_threshold_secs,
            http,
        )
    }

    /// Build a single account's auth manager.
    pub fn new(
        source: CredentialSource,
        region: String,
        api_region: Option<String>,
        token_refresh_threshold_secs: u64,
        http: reqwest::Client,
    ) -> Result<Arc<Self>, GatewayError> {
        let mut creds = Credentials::default();

        // Credential-load failures are non-fatal so the server (and /health) can
        // still start; requests will surface the auth error when a token is needed.
        match &source {
            CredentialSource::File { path } => {
                if let Err(e) = file::load(path, &mut creds) {
                    tracing::warn!("failed to load credentials file: {e}");
                }
            }
            CredentialSource::KiroApp { path } => {
                let token_path = file::resolve_kiro_app_path(path.clone());
                if let Err(e) = file::load(&token_path, &mut creds) {
                    tracing::warn!("failed to load Kiro App credentials: {e}");
                }
            }
            CredentialSource::KiroCli { path, .. } => {
                let db_path = sqlite::resolve_db_path(path.clone());
                if let Err(e) = sqlite::load(&db_path, &mut creds) {
                    tracing::warn!("failed to load kiro-cli credentials: {e}");
                }
            }
            CredentialSource::RefreshToken {
                refresh_token,
                profile_arn,
                region,
            } => {
                creds.refresh_token = Some(refresh_token.clone());
                creds.profile_arn = profile_arn.clone();
                if let Some(r) = region {
                    creds.sso_region = Some(r.clone());
                    creds.detected_api_region = Some(r.clone());
                }
            }
        }

        // API region priority: explicit override > detected > SSO region > default.
        let api_region = api_region
            .or_else(|| creds.detected_api_region.clone())
            .or_else(|| creds.sso_region.clone())
            .unwrap_or_else(|| region.clone());
        let sso_region = creds.sso_region.clone().unwrap_or_else(|| region.clone());
        let api_host = crate::config::kiro_api_host(&api_region);

        Ok(Arc::new(Self {
            source,
            creds: Mutex::new(creds),
            fingerprint: machine_fingerprint(),
            sso_region,
            api_region,
            api_host,
            token_refresh_threshold_secs: token_refresh_threshold_secs as i64,
            http,
        }))
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn api_host(&self) -> &str {
        &self.api_host
    }

    pub fn api_region(&self) -> &str {
        &self.api_region
    }

    pub async fn profile_arn(&self) -> Option<String> {
        self.creds.lock().await.profile_arn.clone()
    }

    fn is_expiring_soon(&self, creds: &Credentials) -> bool {
        match creds.expires_at {
            None => true,
            Some(exp) => {
                let threshold = Utc::now().timestamp() + self.token_refresh_threshold_secs;
                exp.timestamp() <= threshold
            }
        }
    }

    fn is_expired(&self, creds: &Credentials) -> bool {
        match creds.expires_at {
            None => true,
            Some(exp) => Utc::now() >= exp,
        }
    }

    /// Returns a valid access token, refreshing if necessary.
    pub async fn get_access_token(&self) -> Result<String, GatewayError> {
        let mut creds = self.creds.lock().await;

        if let Some(token) = &creds.access_token {
            if !self.is_expiring_soon(&creds) {
                return Ok(token.clone());
            }
        }

        // Kiro may rotate credentials out of band. Reload external stores before
        // refreshing so App-only and CLI-only users follow the latest login.
        if self.is_expiring_soon(&creds) && self.reload_external_credentials(&mut creds) {
            if let Some(token) = &creds.access_token {
                if !self.is_expiring_soon(&creds) {
                    return Ok(token.clone());
                }
            }
        }

        match self.refresh_locked(&mut creds).await {
            Ok(()) => {}
            Err(e) => {
                // External stores can still contain a usable access token even
                // when their refresh endpoint is temporarily unavailable.
                if matches!(
                    self.source,
                    CredentialSource::KiroApp { .. } | CredentialSource::KiroCli { .. }
                ) {
                    if let Some(token) = &creds.access_token {
                        if !self.is_expired(&creds) {
                            tracing::warn!(
                                "token refresh failed, using existing access token until expiry"
                            );
                            return Ok(token.clone());
                        }
                    }
                }
                return Err(e);
            }
        }

        creds
            .access_token
            .clone()
            .ok_or_else(|| GatewayError::Auth("failed to obtain access token".into()))
    }

    /// Force a refresh (used on HTTP 403).
    pub async fn force_refresh(&self) -> Result<String, GatewayError> {
        let mut creds = self.creds.lock().await;
        if matches!(self.source, CredentialSource::KiroApp { .. }) {
            self.reload_external_credentials(&mut creds);
        }
        self.refresh_locked(&mut creds).await?;
        creds
            .access_token
            .clone()
            .ok_or_else(|| GatewayError::Auth("failed to obtain access token".into()))
    }

    async fn refresh_locked(&self, creds: &mut Credentials) -> Result<(), GatewayError> {
        match creds.auth_type() {
            AuthType::KiroDesktop => refresh::refresh_kiro_desktop(self, creds).await,
            AuthType::AwsSsoOidc => refresh::refresh_aws_sso_oidc(self, creds).await,
        }?;
        self.write_back(creds);
        Ok(())
    }

    fn write_back(&self, creds: &Credentials) {
        match &self.source {
            CredentialSource::File { path } => {
                if let Err(e) = file::save(path, creds) {
                    tracing::warn!("failed to write credentials file: {e}");
                }
            }
            CredentialSource::KiroCli { path, readonly } => {
                if *readonly {
                    return;
                }
                let db_path = sqlite::resolve_db_path(path.clone());
                let region = creds
                    .sso_region
                    .clone()
                    .unwrap_or_else(|| self.sso_region.clone());
                if let Err(e) = sqlite::save(&db_path, creds, &region) {
                    tracing::warn!("failed to write kiro-cli sqlite credentials: {e}");
                }
            }
            CredentialSource::KiroApp { .. } => {}
            CredentialSource::RefreshToken { .. } => {}
        }
    }

    fn reload_external_credentials(&self, creds: &mut Credentials) -> bool {
        let mut latest = Credentials::default();
        let loaded = match &self.source {
            CredentialSource::KiroApp { path } => {
                let token_path = file::resolve_kiro_app_path(path.clone());
                file::load(&token_path, &mut latest)
            }
            CredentialSource::KiroCli { path, .. } => {
                let db_path = sqlite::resolve_db_path(path.clone());
                sqlite::load(&db_path, &mut latest)
            }
            _ => return false,
        };
        match loaded {
            Ok(()) => {
                *creds = latest;
                true
            }
            Err(error) => {
                tracing::warn!("failed to reload external credentials: {error}");
                false
            }
        }
    }

    // Accessors used by the refresh module.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }
    pub(crate) fn sso_region(&self) -> &str {
        &self.sso_region
    }
    pub(crate) fn source(&self) -> &CredentialSource {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kiro_app_source_loads_an_explicit_token_file() {
        let path = std::env::temp_dir().join(format!(
            "wecode-kiro-app-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"{
                "accessToken": "kiro-app-access-token",
                "refreshToken": "kiro-app-refresh-token",
                "profileArn": "arn:aws:codewhisperer:us-east-1:123456789012:profile/test",
                "region": "us-east-1",
                "expiresAt": "2099-01-01T00:00:00Z"
            }"#,
        )
        .unwrap();

        let auth = KiroAuth::new(
            CredentialSource::KiroApp {
                path: Some(path.clone()),
            },
            "us-east-1".into(),
            None,
            300,
            reqwest::Client::new(),
        )
        .unwrap();

        assert_eq!(
            auth.get_access_token().await.unwrap(),
            "kiro-app-access-token"
        );
        assert_eq!(
            auth.profile_arn().await.as_deref(),
            Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/test")
        );
        let _ = std::fs::remove_file(path);
    }
}
