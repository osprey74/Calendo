use crate::auth::{icloud, keyring, oauth};
use crate::error::{AppError, AppResult};
use crate::models::{AuthStatus, CalendarSourceId};
use serde::Serialize;

fn parse_source(s: &str) -> AppResult<CalendarSourceId> {
    CalendarSourceId::from_str(s).ok_or_else(|| AppError::UnknownSource(s.to_string()))
}

fn mask(value: Option<&str>) -> Option<String> {
    value.filter(|s| !s.is_empty()).map(|s| {
        let len = s.chars().count();
        if len <= 12 {
            format!("(len={len})")
        } else {
            let head: String = s.chars().take(6).collect();
            let tail: String = s.chars().skip(len - 4).collect();
            format!("{head}…{tail} (len={len})")
        }
    })
}

#[derive(Debug, Serialize)]
pub struct ClientDebugInfo {
    #[serde(rename = "msClientId")]
    ms_client_id: Option<String>,
    #[serde(rename = "googleClientId")]
    google_client_id: Option<String>,
    #[serde(rename = "googleClientSecretConfigured")]
    google_client_secret_configured: bool,
}

#[tauri::command]
pub fn auth_debug_clients() -> ClientDebugInfo {
    ClientDebugInfo {
        ms_client_id: mask(option_env!("MS_CLIENT_ID")),
        google_client_id: mask(option_env!("GOOGLE_CLIENT_ID")),
        google_client_secret_configured: option_env!("GOOGLE_CLIENT_SECRET")
            .map(|s| !s.is_empty())
            .unwrap_or(false),
    }
}

#[tauri::command]
pub async fn auth_start(source_id: String) -> AppResult<AuthStatus> {
    let id = parse_source(&source_id)?;
    if matches!(id, CalendarSourceId::Icloud) {
        return Err(AppError::Other(
            "use auth_icloud_save for iCloud authentication".into(),
        ));
    }
    let tokens = oauth::run_oauth_flow(id).await?;
    Ok(AuthStatus {
        source_id: id,
        connected: true,
        expires_at: Some(tokens.expires_at),
    })
}

#[tauri::command]
pub async fn auth_refresh(source_id: String) -> AppResult<AuthStatus> {
    let id = parse_source(&source_id)?;
    if matches!(id, CalendarSourceId::Icloud) {
        return Ok(AuthStatus {
            source_id: id,
            connected: icloud::is_connected()?,
            expires_at: None,
        });
    }
    let tokens = oauth::refresh(id).await?;
    Ok(AuthStatus {
        source_id: id,
        connected: true,
        expires_at: Some(tokens.expires_at),
    })
}

#[tauri::command]
pub async fn auth_revoke(source_id: String) -> AppResult<()> {
    let id = parse_source(&source_id)?;
    match id {
        CalendarSourceId::Icloud => icloud::revoke(),
        _ => keyring::delete_tokens(id),
    }
}

#[tauri::command]
pub async fn auth_status(source_id: String) -> AppResult<AuthStatus> {
    let id = parse_source(&source_id)?;
    match id {
        CalendarSourceId::Icloud => Ok(AuthStatus {
            source_id: id,
            connected: icloud::is_connected()?,
            expires_at: None,
        }),
        _ => match keyring::load_tokens(id)? {
            Some(t) => Ok(AuthStatus {
                source_id: id,
                connected: true,
                // expires_at is 0 when only the refresh token is on disk and
                // no in-memory access token has been minted yet (typical right
                // after app launch). Surface that as None to the UI.
                expires_at: Some(t.expires_at).filter(|x| *x > 0),
            }),
            None => Ok(AuthStatus {
                source_id: id,
                connected: false,
                expires_at: None,
            }),
        },
    }
}

#[tauri::command]
pub async fn auth_icloud_save(apple_id: String, app_password: String) -> AppResult<AuthStatus> {
    icloud::save_and_verify(apple_id, app_password).await?;
    Ok(AuthStatus {
        source_id: CalendarSourceId::Icloud,
        connected: true,
        expires_at: None,
    })
}
