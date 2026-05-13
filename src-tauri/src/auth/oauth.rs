use crate::auth::keyring::{load_tokens, save_tokens, StoredTokens};
use crate::error::{AppError, AppResult};
use crate::models::CalendarSourceId;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct OAuthProvider {
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scope: &'static str,
    pub client_id: &'static str,
    pub client_secret: Option<&'static str>,
    /// Extra query params for the authorization URL (e.g. Google's `access_type=offline`).
    pub extra_auth_params: &'static [(&'static str, &'static str)],
}

pub fn provider_for(source_id: CalendarSourceId) -> AppResult<OAuthProvider> {
    match source_id {
        CalendarSourceId::Ms365Work1 => Ok(OAuthProvider {
            auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            scope: "Calendars.ReadWrite offline_access User.Read",
            client_id: option_env!("MS_CLIENT_ID")
                .filter(|s| !s.is_empty())
                .ok_or(AppError::MissingCredential("MS_CLIENT_ID"))?,
            client_secret: None,
            extra_auth_params: &[("prompt", "select_account")],
        }),
        CalendarSourceId::GoogleGws => Ok(OAuthProvider {
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            scope: "https://www.googleapis.com/auth/calendar",
            client_id: option_env!("GOOGLE_CLIENT_ID")
                .filter(|s| !s.is_empty())
                .ok_or(AppError::MissingCredential("GOOGLE_CLIENT_ID"))?,
            client_secret: Some(
                option_env!("GOOGLE_CLIENT_SECRET")
                    .filter(|s| !s.is_empty())
                    .ok_or(AppError::MissingCredential("GOOGLE_CLIENT_SECRET"))?,
            ),
            extra_auth_params: &[
                ("access_type", "offline"),
                ("prompt", "consent"),
            ],
        }),
        CalendarSourceId::Icloud => Err(AppError::Other(
            "iCloud uses CalDAV with app-specific password — not OAuth".into(),
        )),
    }
}

fn random_url_safe(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

const CALLBACK_TIMEOUT_SECS: u64 = 300;

pub async fn run_oauth_flow(source_id: CalendarSourceId) -> AppResult<StoredTokens> {
    let provider = provider_for(source_id)?;

    let verifier = random_url_safe(64);
    let challenge = pkce_challenge(&verifier);
    let state = random_url_safe(24);

    let (tx, rx) = oneshot::channel::<String>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_clone = tx.clone();

    let port = tauri_plugin_oauth::start(move |url| {
        if let Ok(mut guard) = tx_clone.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(url);
            }
        }
    })
    .map_err(|e| AppError::OAuth(format!("failed to start oauth listener: {e}")))?;

    let redirect_uri = format!("http://localhost:{port}");

    let mut auth_url = url::Url::parse(provider.auth_url)?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", provider.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", provider.scope)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");

    for (k, v) in provider.extra_auth_params {
        auth_url.query_pairs_mut().append_pair(k, v);
    }

    open::that(auth_url.as_str())
        .map_err(|e| AppError::OAuth(format!("failed to open browser: {e}")))?;

    let redirect_url = tokio::time::timeout(Duration::from_secs(CALLBACK_TIMEOUT_SECS), rx)
        .await
        .map_err(|_| AppError::CallbackTimeout)?
        .map_err(|_| AppError::OAuth("oauth callback channel closed".into()))?;

    let parsed = url::Url::parse(&redirect_url)?;
    let mut code: Option<String> = None;
    let mut returned_state: Option<String> = None;
    let mut err_param: Option<String> = None;
    let mut err_desc: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => returned_state = Some(v.into_owned()),
            "error" => err_param = Some(v.into_owned()),
            "error_description" => err_desc = Some(v.into_owned()),
            _ => {}
        }
    }

    if let Some(e) = err_param {
        return Err(AppError::OAuth(format!(
            "{e}: {}",
            err_desc.unwrap_or_default()
        )));
    }

    if returned_state.as_deref() != Some(state.as_str()) {
        return Err(AppError::StateMismatch);
    }
    let code = code.ok_or_else(|| AppError::OAuth("missing authorization code".into()))?;

    let tokens = exchange_code(&provider, &code, &verifier, &redirect_uri).await?;
    save_tokens(source_id, &tokens)?;
    Ok(tokens)
}

async fn exchange_code(
    provider: &OAuthProvider,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> AppResult<StoredTokens> {
    let mut form: HashMap<&str, &str> = HashMap::new();
    form.insert("grant_type", "authorization_code");
    form.insert("code", code);
    form.insert("redirect_uri", redirect_uri);
    form.insert("client_id", provider.client_id);
    form.insert("code_verifier", verifier);
    if let Some(secret) = provider.client_secret {
        form.insert("client_secret", secret);
    }

    post_token(provider.token_url, &form).await
}

pub async fn refresh(source_id: CalendarSourceId) -> AppResult<StoredTokens> {
    let provider = provider_for(source_id)?;
    let stored = load_tokens(source_id)?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;
    let refresh_token = stored
        .refresh_token
        .as_deref()
        .ok_or(AppError::TokenExpired)?;

    let mut form: HashMap<&str, &str> = HashMap::new();
    form.insert("grant_type", "refresh_token");
    form.insert("refresh_token", refresh_token);
    form.insert("client_id", provider.client_id);
    if let Some(secret) = provider.client_secret {
        form.insert("client_secret", secret);
    }

    let mut new_tokens = post_token(provider.token_url, &form).await?;
    if new_tokens.refresh_token.is_none() {
        new_tokens.refresh_token = stored.refresh_token.clone();
    }
    save_tokens(source_id, &new_tokens)?;
    Ok(new_tokens)
}

async fn post_token(url: &str, form: &HashMap<&str, &str>) -> AppResult<StoredTokens> {
    let client = reqwest::Client::new();
    let resp = client.post(url).form(form).send().await?;
    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        let parsed: Result<TokenError, _> = serde_json::from_str(&body);
        let msg = match parsed {
            Ok(e) => format!("{}: {}", e.error, e.error_description.unwrap_or_default()),
            Err(_) => body,
        };
        return Err(AppError::OAuth(format!("token endpoint {status}: {msg}")));
    }

    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::OAuth(format!("failed to parse token response: {e}")))?;

    Ok(StoredTokens::from_response(
        parsed.access_token,
        parsed.refresh_token,
        parsed.expires_in,
    ))
}

/// Ensures we have a fresh access token, refreshing if needed.
pub async fn ensure_fresh(source_id: CalendarSourceId) -> AppResult<StoredTokens> {
    let stored = load_tokens(source_id)?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;
    if stored.is_expired() {
        refresh(source_id).await
    } else {
        Ok(stored)
    }
}

/// Send an authenticated HTTP request with automatic 401 → refresh → retry.
///
/// `build_req` is invoked with the current access token. If the server replies 401
/// (token revoked, clock skew, or just-rotated), we refresh once and rebuild the request
/// with the new token. If the second response is still 401 — i.e., refresh succeeded
/// but the new access token also got rejected — we surface `AppError::AuthRequired` so
/// the UI can prompt re-authentication for that specific source.
///
/// The closure is `Fn` rather than `FnOnce` because reqwest's `RequestBuilder` isn't
/// `Clone` — re-issuing the request requires reconstructing it.
pub async fn send_with_refresh<F>(
    source_id: CalendarSourceId,
    build_req: F,
) -> AppResult<reqwest::Response>
where
    F: Fn(&str) -> reqwest::RequestBuilder,
{
    let tokens = ensure_fresh(source_id).await?;
    let resp = build_req(&tokens.access_token).send().await?;
    if resp.status().as_u16() != 401 {
        return Ok(resp);
    }
    // Drain the 401 body so the connection can be returned to the pool cleanly.
    let _ = resp.bytes().await;
    // Refresh failures themselves are already structured (TokenExpired / NotAuthenticated
    // / OAuth) — propagate as-is so the frontend can distinguish "refresh impossible"
    // from "refresh ok but server still says no".
    let refreshed = refresh(source_id).await?;
    let retry = build_req(&refreshed.access_token).send().await?;
    if retry.status().as_u16() == 401 {
        let _ = retry.bytes().await;
        return Err(AppError::AuthRequired(source_id.as_str().into()));
    }
    Ok(retry)
}
