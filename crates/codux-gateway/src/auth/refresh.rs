use chrono::{Duration, Utc};
use serde_json::{json, Value};

use super::sqlite;
use super::{Credentials, KiroAuth};
use crate::config::{aws_sso_oidc_url, kiro_refresh_url, CredentialSource};
use crate::error::GatewayError;

/// Refresh via Kiro Desktop auth endpoint.
pub async fn refresh_kiro_desktop(
    auth: &KiroAuth,
    creds: &mut Credentials,
) -> Result<(), GatewayError> {
    let refresh_token = creds
        .refresh_token
        .clone()
        .ok_or_else(|| GatewayError::Auth("refresh token is not set".into()))?;

    let url = kiro_refresh_url(auth.sso_region());
    let resp = auth
        .http()
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            format!("KiroIDE-0.7.45-{}", auth.fingerprint()),
        )
        .json(&json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .map_err(|e| GatewayError::Network(format!("refresh request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(GatewayError::Upstream {
            status: status.as_u16(),
            body,
        });
    }
    let data: Value = serde_json::from_str(&body)
        .map_err(|e| GatewayError::Auth(format!("invalid refresh response: {e}")))?;

    let access = data
        .get("accessToken")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::Auth(format!("response missing accessToken: {body}")))?;
    creds.access_token = Some(access.to_string());
    if let Some(v) = data.get("refreshToken").and_then(Value::as_str) {
        creds.refresh_token = Some(v.to_string());
    }
    if let Some(v) = data.get("profileArn").and_then(Value::as_str) {
        creds.profile_arn = Some(v.to_string());
    }
    let expires_in = data
        .get("expiresIn")
        .and_then(Value::as_i64)
        .unwrap_or(3600);
    creds.expires_at = Some(Utc::now() + Duration::seconds(expires_in - 60));
    Ok(())
}

/// Refresh via AWS SSO OIDC (kiro-cli). On a 400, reload creds from SQLite and retry once.
pub async fn refresh_aws_sso_oidc(
    auth: &KiroAuth,
    creds: &mut Credentials,
) -> Result<(), GatewayError> {
    match do_oidc_refresh(auth, creds).await {
        Ok(()) => Ok(()),
        Err(GatewayError::Upstream { status: 400, .. }) => {
            if let CredentialSource::KiroCli { path, .. } = auth.source() {
                tracing::warn!("OIDC refresh 400, reloading credentials from sqlite and retrying");
                let db_path = sqlite::resolve_db_path(path.clone());
                let _ = sqlite::load(&db_path, creds);
                do_oidc_refresh(auth, creds).await
            } else {
                Err(GatewayError::Upstream {
                    status: 400,
                    body: "OIDC refresh failed".into(),
                })
            }
        }
        Err(e) => Err(e),
    }
}

async fn do_oidc_refresh(auth: &KiroAuth, creds: &mut Credentials) -> Result<(), GatewayError> {
    let refresh_token = creds
        .refresh_token
        .clone()
        .ok_or_else(|| GatewayError::Auth("refresh token is not set".into()))?;
    let client_id = creds
        .client_id
        .clone()
        .ok_or_else(|| GatewayError::Auth("client id is not set (AWS SSO OIDC)".into()))?;
    let client_secret = creds
        .client_secret
        .clone()
        .ok_or_else(|| GatewayError::Auth("client secret is not set (AWS SSO OIDC)".into()))?;

    let sso_region = creds
        .sso_region
        .clone()
        .unwrap_or_else(|| auth.sso_region().to_string());
    let url = aws_sso_oidc_url(&sso_region);

    // AWS SSO OIDC CreateToken: JSON body with camelCase keys.
    let payload = json!({
        "grantType": "refresh_token",
        "clientId": client_id,
        "clientSecret": client_secret,
        "refreshToken": refresh_token,
    });

    let resp = auth
        .http()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| GatewayError::Network(format!("OIDC refresh request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(GatewayError::Upstream {
            status: status.as_u16(),
            body,
        });
    }
    let data: Value = serde_json::from_str(&body)
        .map_err(|e| GatewayError::Auth(format!("invalid OIDC response: {e}")))?;

    let access = data
        .get("accessToken")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::Auth(format!("OIDC response missing accessToken: {body}")))?;
    creds.access_token = Some(access.to_string());
    if let Some(v) = data.get("refreshToken").and_then(Value::as_str) {
        creds.refresh_token = Some(v.to_string());
    }
    let expires_in = data
        .get("expiresIn")
        .and_then(Value::as_i64)
        .unwrap_or(3600);
    creds.expires_at = Some(Utc::now() + Duration::seconds(expires_in - 60));
    Ok(())
}
