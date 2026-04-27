use crate::auth::{icloud, keyring, oauth};
use crate::error::{AppError, AppResult};
use crate::models::{AuthStatus, CalendarSourceId};

fn parse_source(s: &str) -> AppResult<CalendarSourceId> {
    CalendarSourceId::from_str(s).ok_or_else(|| AppError::UnknownSource(s.to_string()))
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
                expires_at: Some(t.expires_at),
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
