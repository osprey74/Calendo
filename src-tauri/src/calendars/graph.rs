use crate::auth::oauth::ensure_fresh;
use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GraphCalendar {
    id: String,
    name: String,
    #[serde(rename = "isDefaultCalendar", default)]
    is_default_calendar: bool,
    #[serde(default)]
    color: Option<String>,
    #[serde(rename = "canEdit", default)]
    can_edit: bool,
}

#[derive(Debug, Deserialize)]
struct GraphCalendarList {
    value: Vec<GraphCalendar>,
}

/// Fetches the list of calendars (including shared sub-calendars) for a Microsoft 365 account.
pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    let tokens = ensure_fresh(source_id).await?;
    let client = reqwest::Client::new();
    let resp = client
        .get("https://graph.microsoft.com/v1.0/me/calendars")
        .bearer_auth(&tokens.access_token)
        .send()
        .await?
        .error_for_status()?;
    let body: GraphCalendarList = resp.json().await?;

    let calendars = body
        .value
        .into_iter()
        .map(|c| CalendarMeta {
            id: c.id,
            source_id,
            name: c.name,
            is_primary: c.is_default_calendar,
            color: c.color,
            is_writable: c.can_edit,
            enabled: true,
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
    // TODO: Phase 2 — implement /me/calendars/{id}/calendarView
    Ok(Vec::new())
}
