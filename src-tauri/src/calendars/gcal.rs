use crate::auth::oauth::send_with_refresh;
use crate::calendars::util::percent_encode_segment;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, EventDraft, UnifiedEvent};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};

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
    let client = reqwest::Client::new();
    let url = format!("{GCAL_BASE}/users/me/calendarList");
    let resp = send_with_refresh(source_id, |token| {
        client.get(&url).bearer_auth(token)
    })
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

        let resp = send_with_refresh(source_id, |token| {
            client.get(&url).bearer_auth(token)
        })
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

// --- Write operations -------------------------------------------------------

fn build_event_payload(draft: &EventDraft) -> AppResult<JsonValue> {
    let mut obj = json!({
        "summary": draft.title,
    });
    if let Some(loc) = &draft.location {
        if !loc.is_empty() {
            obj["location"] = json!(loc);
        }
    }
    if let Some(body) = &draft.body {
        if !body.is_empty() {
            obj["description"] = json!(body);
        }
    }
    if draft.is_all_day {
        // Google all-day uses {date: "YYYY-MM-DD"} with EXCLUSIVE end. Drafts use inclusive
        // end (form convention) so we add one day.
        let _ = NaiveDate::parse_from_str(&draft.start, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {}: {e}", draft.start)))?;
        let end_inc = NaiveDate::parse_from_str(&draft.end, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {}: {e}", draft.end)))?;
        let end_exc = end_inc
            .succ_opt()
            .ok_or_else(|| AppError::Other(format!("date overflow at {}", draft.end)))?
            .format("%Y-%m-%d")
            .to_string();
        obj["start"] = json!({ "date": draft.start });
        obj["end"] = json!({ "date": end_exc });
    } else {
        // Drafts already carry RFC3339 with `+09:00`, which Google accepts directly.
        obj["start"] = json!({ "dateTime": draft.start, "timeZone": "Asia/Tokyo" });
        obj["end"] = json!({ "dateTime": draft.end, "timeZone": "Asia/Tokyo" });
    }
    Ok(obj)
}

pub async fn create_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let calendar_seg = percent_encode_segment(calendar_id);
    let url = format!("{GCAL_BASE}/calendars/{calendar_seg}/events");
    let payload = build_event_payload(draft)?;
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client.post(&url).bearer_auth(token).json(&payload)
    })
    .await?
    .error_for_status()?;
    let event: GCalEvent = resp.json().await?;
    gcal_event_to_unified(source_id, calendar_id, event)
        .ok_or_else(|| AppError::Other("GCal create returned event with invalid datetimes".into()))
}

pub async fn update_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    event_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let calendar_seg = percent_encode_segment(calendar_id);
    let event_seg = percent_encode_segment(event_id);
    let url = format!("{GCAL_BASE}/calendars/{calendar_seg}/events/{event_seg}");
    let payload = build_event_payload(draft)?;
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client.patch(&url).bearer_auth(token).json(&payload)
    })
    .await?
    .error_for_status()?;
    let event: GCalEvent = resp.json().await?;
    gcal_event_to_unified(source_id, calendar_id, event)
        .ok_or_else(|| AppError::Other("GCal update returned event with invalid datetimes".into()))
}

pub async fn delete_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    event_id: &str,
) -> AppResult<()> {
    let calendar_seg = percent_encode_segment(calendar_id);
    let event_seg = percent_encode_segment(event_id);
    // sendUpdates=none avoids notifying attendees of cancellations the user is just
    // removing locally; the Phase 4 SettingsModal can expose this as a preference later.
    let url = format!(
        "{GCAL_BASE}/calendars/{calendar_seg}/events/{event_seg}?sendUpdates=none"
    );
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client.delete(&url).bearer_auth(token)
    })
    .await?;
    let status = resp.status();
    if !status.is_success() && status.as_u16() != 404 && status.as_u16() != 410 {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "GCal DELETE {url} → {status}: {text}"
        )));
    }
    Ok(())
}

