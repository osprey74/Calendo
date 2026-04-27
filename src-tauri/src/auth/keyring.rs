use crate::error::{AppError, AppResult};
use crate::models::CalendarSourceId;
use chrono::Utc;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "calendo";

fn entry(source_id: CalendarSourceId, key: &str) -> AppResult<keyring::Entry> {
    let user = format!("{}.{}", source_id.as_str(), key);
    keyring::Entry::new(SERVICE, &user).map_err(AppError::from)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds since epoch) at which `access_token` expires.
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

/// Inner struct for the access-token slot. Splitting access_token and refresh_token
/// across two keyring entries keeps each one within the Windows Credential Manager
/// 2560-UTF-16-char (5120-byte) password limit — Microsoft Graph access tokens alone
/// can be 2-3 KB and refresh tokens add another 1-2 KB.
#[derive(Debug, Serialize, Deserialize)]
struct AccessSlot {
    access_token: String,
    expires_at: i64,
}

pub fn save_tokens(source_id: CalendarSourceId, tokens: &StoredTokens) -> AppResult<()> {
    let access = AccessSlot {
        access_token: tokens.access_token.clone(),
        expires_at: tokens.expires_at,
    };
    entry(source_id, "access")?.set_password(&serde_json::to_string(&access)?)?;

    let refresh_entry = entry(source_id, "refresh")?;
    if let Some(rt) = tokens.refresh_token.as_ref() {
        refresh_entry.set_password(rt)?;
    } else {
        match refresh_entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub fn load_tokens(source_id: CalendarSourceId) -> AppResult<Option<StoredTokens>> {
    let access_json = match entry(source_id, "access")?.get_password() {
        Ok(s) => s,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => return Err(AppError::from(e)),
    };
    let access: AccessSlot = serde_json::from_str(&access_json)?;

    let refresh_token = match entry(source_id, "refresh")?.get_password() {
        Ok(s) => Some(s),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => return Err(AppError::from(e)),
    };

    Ok(Some(StoredTokens {
        access_token: access.access_token,
        refresh_token,
        expires_at: access.expires_at,
    }))
}

pub fn delete_tokens(source_id: CalendarSourceId) -> AppResult<()> {
    for slot in ["access", "refresh", "tokens"] {
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
