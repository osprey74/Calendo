use crate::auth::oauth::ensure_fresh;
use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GCalCalendarListItem {
    id: String,
    summary: String,
    #[serde(default)]
    primary: bool,
    #[serde(rename = "backgroundColor", default)]
    background_color: Option<String>,
    #[serde(rename = "accessRole", default)]
    access_role: String,
}

#[derive(Debug, Deserialize)]
struct GCalCalendarList {
    items: Vec<GCalCalendarListItem>,
}

pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    let tokens = ensure_fresh(source_id).await?;
    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(&tokens.access_token)
        .send()
        .await?
        .error_for_status()?;
    let body: GCalCalendarList = resp.json().await?;

    let calendars = body
        .items
        .into_iter()
        .map(|c| {
            let writable = matches!(c.access_role.as_str(), "owner" | "writer");
            CalendarMeta {
                id: c.id,
                source_id,
                name: c.summary,
                is_primary: c.primary,
                color: c.background_color,
                is_writable: writable,
                enabled: true,
            }
        })
        .collect();
    Ok(calendars)
}

#[allow(unused_variables)]
pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    // TODO: Phase 2 — implement /calendars/{calendarId}/events
    Ok(Vec::new())
}
