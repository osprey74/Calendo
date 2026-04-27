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

pub fn save_tokens(source_id: CalendarSourceId, tokens: &StoredTokens) -> AppResult<()> {
    let json = serde_json::to_string(tokens)?;
    entry(source_id, "tokens")?.set_password(&json)?;
    Ok(())
}

pub fn load_tokens(source_id: CalendarSourceId) -> AppResult<Option<StoredTokens>> {
    match entry(source_id, "tokens")?.get_password() {
        Ok(json) => {
            let tokens = serde_json::from_str::<StoredTokens>(&json)?;
            Ok(Some(tokens))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::from(e)),
    }
}

pub fn delete_tokens(source_id: CalendarSourceId) -> AppResult<()> {
    match entry(source_id, "tokens")?.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::from(e)),
    }
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
