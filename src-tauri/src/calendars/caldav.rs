use crate::auth::keyring::load_icloud;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};

const ICLOUD_BASE: &str = "https://caldav.icloud.com";

/// Fetches the list of CalDAV calendars by walking principal → calendar-home-set → calendars.
///
/// Phase 1: returns at minimum the user's principal-derived calendar(s).
/// Phase 2 will enumerate sub-calendars and parse `<displayname>` / `<calendar-color>`.
pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal/>
  </d:prop>
</d:propfind>"#;

    let client = reqwest::Client::new();
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), ICLOUD_BASE)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .header("Depth", "0")
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(propfind_body)
        .send()
        .await?;

    if !resp.status().is_success() && resp.status().as_u16() != 207 {
        return Err(AppError::CalDav(format!(
            "PROPFIND principal returned {}",
            resp.status()
        )));
    }

    // TODO: Phase 2 — parse principal URL, follow calendar-home-set, enumerate calendars.
    // Returning an empty list for now to keep Phase 1 scope honest about CalDAV being a
    // "connection verified" milestone rather than a full implementation.
    Ok(Vec::new())
}

#[allow(unused_variables)]
pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    // TODO: Phase 2 — REPORT calendar-query with time-range, parse VEVENT, expand RRULE.
    Ok(Vec::new())
}
