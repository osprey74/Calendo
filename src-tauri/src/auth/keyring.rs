use crate::error::{AppError, AppResult};
use crate::models::CalendarSourceId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "calendo";

fn entry(source_id: CalendarSourceId, key: &str) -> AppResult<keyring::Entry> {
    let user = format!("{}.{}", source_id.as_str(), key);
    keyring::Entry::new(SERVICE, &user).map_err(AppError::from)
}

/// In-memory cache for access tokens. Microsoft Graph access tokens are JWTs
/// that can exceed 2560 UTF-16 chars on their own — beyond the Windows
/// Credential Manager limit — so we keep them in memory only and persist only
/// the (much smaller) refresh token to the OS keychain. Access tokens are
/// re-issued via the refresh token on first use after each app launch.
#[derive(Clone)]
struct AccessCacheEntry {
    access_token: String,
    expires_at: i64,
}

fn access_cache() -> &'static Mutex<HashMap<CalendarSourceId, AccessCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<CalendarSourceId, AccessCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds since epoch) at which `access_token` expires.
    /// Will be 0 if loaded from a fresh process where only the refresh token
    /// is on disk — `is_expired()` treats this as "needs refresh".
    pub expires_at: i64,
}

impl StoredTokens {
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.expires_at - 30
    }

    pub fn from_response(access_token: String, refresh_token: Option<String>, expires_in_secs: i64) -> Self {
        let expires_at = Utc::now().timestamp() + expires_in_secs;
        Self { access_token, refresh_token, expires_at }
    }
}

pub fn save_tokens(source_id: CalendarSourceId, tokens: &StoredTokens) -> AppResult<()> {
    // Persist refresh_token only — access tokens go to in-memory cache.
    let refresh_entry = entry(source_id, "refresh")?;
    if let Some(rt) = tokens.refresh_token.as_ref() {
        refresh_entry.set_password(rt)?;
    }

    if !tokens.access_token.is_empty() {
        access_cache().lock().unwrap().insert(
            source_id,
            AccessCacheEntry {
                access_token: tokens.access_token.clone(),
                expires_at: tokens.expires_at,
            },
        );
    }
    Ok(())
}

pub fn load_tokens(source_id: CalendarSourceId) -> AppResult<Option<StoredTokens>> {
    let refresh_token = match entry(source_id, "refresh")?.get_password() {
        Ok(s) => Some(s),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => return Err(AppError::from(e)),
    };

    if refresh_token.is_none() {
        return Ok(None);
    }

    let cached = access_cache().lock().unwrap().get(&source_id).cloned();
    let (access_token, expires_at) = match cached {
        Some(c) => (c.access_token, c.expires_at),
        // No in-memory cache yet (likely first call after app launch). Caller
        // is expected to invoke ensure_fresh() which will refresh on demand.
        None => (String::new(), 0),
    };

    Ok(Some(StoredTokens {
        access_token,
        refresh_token,
        expires_at,
    }))
}

pub fn delete_tokens(source_id: CalendarSourceId) -> AppResult<()> {
    access_cache().lock().unwrap().remove(&source_id);
    // Clean up legacy entries from earlier Phase 1 iterations as well.
    for slot in ["refresh", "access", "tokens"] {
        match entry(source_id, slot)?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(AppError::from(e)),
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICloudCredentials {
    pub apple_id: String,
    pub app_password: String,
}

pub fn save_icloud(creds: &ICloudCredentials) -> AppResult<()> {
    let json = serde_json::to_string(creds)?;
    entry(CalendarSourceId::Icloud, "credentials")?.set_password(&json)?;
    Ok(())
}

pub fn load_icloud() -> AppResult<Option<ICloudCredentials>> {
    match entry(CalendarSourceId::Icloud, "credentials")?.get_password() {
        Ok(json) => Ok(Some(serde_json::from_str::<ICloudCredentials>(&json)?)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::from(e)),
    }
}

pub fn delete_icloud() -> AppResult<()> {
    match entry(CalendarSourceId::Icloud, "credentials")?.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::from(e)),
    }
}
