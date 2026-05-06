use crate::auth::oauth::ensure_fresh;
use crate::calendars::util::percent_encode_segment;
use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Deserialize;

const GCAL_BASE: &str = "https://www.googleapis.com/calendar/v3";
const JST_OFFSET_SECS: i32 = 9 * 3600;

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
        .get(format!("{GCAL_BASE}/users/me/calendarList"))
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

#[derive(Debug, Deserialize)]
struct GCalEventDateTime {
    #[serde(default)]
    date: Option<String>,
    #[serde(rename = "dateTime", default)]
    date_time: Option<String>,
    #[serde(rename = "timeZone", default)]
    _time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GCalEvent {
    id: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    location: Option<String>,
    start: GCalEventDateTime,
    end: GCalEventDateTime,
    #[serde(rename = "recurringEventId", default)]
    recurring_event_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GCalEventList {
    #[serde(default)]
    items: Vec<GCalEvent>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    let tokens = ensure_fresh(source_id).await?;
    let client = reqwest::Client::new();

    // Google requires RFC3339 timeMin/timeMax. Use UTC bounds spanning the requested dates.
    let time_min = format!("{date_from}T00:00:00Z");
    let time_max = format!("{date_to}T23:59:59Z");
    let calendar_path = percent_encode_segment(calendar_id);

    let mut events: Vec<UnifiedEvent> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = format!(
            "{GCAL_BASE}/calendars/{calendar_path}/events\
             ?singleEvents=true&orderBy=startTime\
             &timeMin={time_min}&timeMax={time_max}\
             &maxResults=250"
        );
        if let Some(token) = &page_token {
            url.push_str(&format!("&pageToken={}", percent_encode_segment(token)));
        }

        let resp = client
            .get(&url)
            .bearer_auth(&tokens.access_token)
            .send()
            .await?
            .error_for_status()?;
        let body: GCalEventList = resp.json().await?;

        for e in body.items {
            if e.status.as_deref() == Some("cancelled") {
                continue;
            }
            if let Some(unified) = gcal_event_to_unified(source_id, calendar_id, e) {
                events.push(unified);
            }
        }

        page_token = body.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(events)
}

fn gcal_event_to_unified(
    source_id: CalendarSourceId,
    calendar_id: &str,
    e: GCalEvent,
) -> Option<UnifiedEvent> {
    let (start_iso, is_all_day) = parse_gcal_datetime(&e.start)?;
    let (end_iso, _) = parse_gcal_datetime(&e.end)?;

    Some(UnifiedEvent {
        id: e.id,
        source_id,
        calendar_id: calendar_id.to_string(),
        title: e.summary.unwrap_or_default(),
        start: start_iso,
        end: end_iso,
        is_all_day,
        location: e.location.filter(|s| !s.is_empty()),
        body: e.description.filter(|s| !s.is_empty()),
        is_recurring: e.recurring_event_id.is_some(),
        recurring_event_id: e.recurring_event_id,
        recurrence_rule: None,
    })
}

fn parse_gcal_datetime(dt: &GCalEventDateTime) -> Option<(String, bool)> {
    if let Some(date) = &dt.date {
        let _ = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        return Some((date.clone(), true));
    }
    let raw = dt.date_time.as_deref()?;
    // Google returns RFC3339 with offset (e.g. "2025-11-01T18:00:00+09:00" or "...Z").
    let parsed = DateTime::parse_from_rfc3339(raw).ok()?;
    let jst = FixedOffset::east_opt(JST_OFFSET_SECS).unwrap();
    let in_jst = parsed.with_timezone(&jst);
    Some((in_jst.format("%Y-%m-%dT%H:%M:%S%:z").to_string(), false))
}

