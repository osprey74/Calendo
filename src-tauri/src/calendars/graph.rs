use crate::auth::oauth::send_with_refresh;
use crate::calendars::util::percent_encode_segment;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, EventDraft, UnifiedEvent};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
const JST_OFFSET_SECS: i32 = 9 * 3600;

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
    #[serde(rename = "@odata.nextLink", default)]
    next_link: Option<String>,
}

pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    let client = reqwest::Client::new();

    // Default page size for /me/calendars is 10. Bump to 100 and walk nextLink so
    // mailboxes with many shared/own calendars surface every entry.
    let mut url = Some(format!("{GRAPH_BASE}/me/calendars?$top=100"));
    let mut calendars: Vec<CalendarMeta> = Vec::new();

    while let Some(next) = url.take() {
        let resp = send_with_refresh(source_id, |token| {
            client.get(&next).bearer_auth(token)
        })
        .await?
        .error_for_status()?;
        let body: GraphCalendarList = resp.json().await?;
        for c in body.value {
            calendars.push(CalendarMeta {
                id: c.id,
                source_id,
                name: c.name,
                is_primary: c.is_default_calendar,
                color: graph_color(c.color),
                is_writable: c.can_edit,
                enabled: true,
            });
        }
        url = body.next_link;
    }

    Ok(calendars)
}

#[derive(Debug, Deserialize)]
struct GraphDateTime {
    #[serde(rename = "dateTime")]
    date_time: String,
    #[serde(rename = "timeZone", default)]
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphLocation {
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphBody {
    #[serde(default)]
    content: Option<String>,
    #[serde(rename = "contentType", default)]
    _content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphEvent {
    id: String,
    #[serde(default)]
    subject: Option<String>,
    start: GraphDateTime,
    end: GraphDateTime,
    #[serde(rename = "isAllDay", default)]
    is_all_day: bool,
    #[serde(default)]
    location: Option<GraphLocation>,
    #[serde(default)]
    body: Option<GraphBody>,
    #[serde(rename = "type", default)]
    event_type: Option<String>,
    #[serde(rename = "seriesMasterId", default)]
    series_master_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphEventList {
    value: Vec<GraphEvent>,
    #[serde(rename = "@odata.nextLink", default)]
    next_link: Option<String>,
}

pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    let client = reqwest::Client::new();

    let start = format!("{date_from}T00:00:00");
    let end = format!("{date_to}T23:59:59");

    let calendar_seg = percent_encode_segment(calendar_id);
    let initial = format!(
        "{GRAPH_BASE}/me/calendars/{calendar_seg}/calendarView\
         ?startDateTime={start}&endDateTime={end}\
         &$top=200\
         &$select=id,subject,start,end,isAllDay,location,body,type,seriesMasterId"
    );

    let mut url = Some(initial);
    let mut events: Vec<UnifiedEvent> = Vec::new();

    while let Some(next) = url.take() {
        let resp = send_with_refresh(source_id, |token| {
            client
                .get(&next)
                .bearer_auth(token)
                // Outlook: tell Graph to return times in UTC for predictable parsing.
                .header("Prefer", r#"outlook.timezone="UTC""#)
        })
        .await?
        .error_for_status()?;
        let body: GraphEventList = resp.json().await?;

        for e in body.value {
            if let Some(unified) = graph_event_to_unified(source_id, calendar_id, e) {
                events.push(unified);
            }
        }

        url = body.next_link;
    }

    Ok(events)
}

fn graph_event_to_unified(
    source_id: CalendarSourceId,
    calendar_id: &str,
    e: GraphEvent,
) -> Option<UnifiedEvent> {
    let start_iso = parse_graph_datetime(&e.start, e.is_all_day)?;
    let end_iso = parse_graph_datetime(&e.end, e.is_all_day)?;

    let is_recurring_instance = e
        .event_type
        .as_deref()
        .map(|t| matches!(t, "occurrence" | "exception"))
        .unwrap_or(false);

    Some(UnifiedEvent {
        id: e.id,
        source_id,
        calendar_id: calendar_id.to_string(),
        title: e.subject.unwrap_or_default(),
        start: start_iso,
        end: end_iso,
        is_all_day: e.is_all_day,
        location: e.location.and_then(|l| l.display_name).filter(|s| !s.is_empty()),
        body: e
            .body
            .and_then(|b| b.content)
            .map(strip_html)
            .filter(|s| !s.is_empty()),
        is_recurring: is_recurring_instance,
        recurring_event_id: e.series_master_id,
        recurrence_rule: None,
    })
}

fn parse_graph_datetime(dt: &GraphDateTime, is_all_day: bool) -> Option<String> {
    if is_all_day {
        // Graph returns "2025-11-01T00:00:00.0000000" with timeZone "UTC" for all-day.
        let date_part = dt.date_time.split('T').next()?;
        return Some(date_part.to_string());
    }

    // Trim Graph's high-precision fractional seconds (".0000000") which `chrono` doesn't accept.
    let trimmed = dt.date_time.split('.').next().unwrap_or(&dt.date_time);
    let naive = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S").ok()?;
    let jst = FixedOffset::east_opt(JST_OFFSET_SECS).unwrap();

    let dt_jst: DateTime<FixedOffset> = match dt.time_zone.as_deref() {
        Some("UTC") | Some("Etc/UTC") | None => {
            Utc.from_utc_datetime(&naive).with_timezone(&jst)
        }
        Some(tz_name) => match tz_name.parse::<Tz>() {
            Ok(tz) => tz
                .from_local_datetime(&naive)
                .single()
                .map(|local| local.with_timezone(&jst))
                .unwrap_or_else(|| Utc.from_utc_datetime(&naive).with_timezone(&jst)),
            Err(_) => Utc.from_utc_datetime(&naive).with_timezone(&jst),
        },
    };

    Some(dt_jst.format("%Y-%m-%dT%H:%M:%S%:z").to_string())
}

fn strip_html(s: String) -> String {
    let mut out = String::with_capacity(s.len());
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Outlook returns named colors like "auto", "lightBlue". We pass them through as-is —
/// the UI palette handler is responsible for mapping or falling back to source default.
fn graph_color(c: Option<String>) -> Option<String> {
    c.filter(|s| !s.is_empty() && s != "auto")
}

// --- Write operations -------------------------------------------------------

/// Build the JSON payload for `POST /me/calendars/{id}/events` and `PATCH /me/events/{id}`.
/// Both endpoints accept the same shape, so the same builder serves create and update.
fn build_event_payload(draft: &EventDraft) -> AppResult<JsonValue> {
    let (start, end) = build_datetime_pair(draft)?;
    let mut obj = json!({
        "subject": draft.title,
        "start": start,
        "end": end,
        "isAllDay": draft.is_all_day,
    });
    if let Some(loc) = &draft.location {
        if !loc.is_empty() {
            obj["location"] = json!({ "displayName": loc });
        }
    }
    if let Some(body) = &draft.body {
        if !body.is_empty() {
            obj["body"] = json!({ "contentType": "text", "content": body });
        }
    }
    if let Some(rrule) = &draft.recurrence_rule {
        if let Some(recurrence) = build_graph_recurrence(rrule, &draft.start, draft.is_all_day)? {
            obj["recurrence"] = recurrence;
        }
    }
    Ok(obj)
}

/// Translate a RFC 5545 RRULE (the small subset Calendo's UI emits) into Graph's nested
/// `recurrence` JSON. Returns `None` for unsupported RRULE shapes — callers can decide to
/// drop the recurrence rather than failing the request outright.
fn build_graph_recurrence(
    rrule: &str,
    start_iso: &str,
    is_all_day: bool,
) -> AppResult<Option<JsonValue>> {
    let parts = parse_rrule(rrule);
    let Some(freq) = parts.get("FREQ").map(|s| s.as_str()) else {
        return Ok(None);
    };
    let start_date = parse_start_date(start_iso, is_all_day)?;

    let pattern = match freq {
        "DAILY" => json!({ "type": "daily", "interval": 1 }),
        "WEEKLY" => {
            let days = parts
                .get("BYDAY")
                .map(|byday| {
                    byday
                        .split(',')
                        .filter_map(byday_to_graph_name)
                        .collect::<Vec<_>>()
                })
                .filter(|v: &Vec<_>| !v.is_empty())
                .unwrap_or_else(|| vec![weekday_to_graph_name(start_date.weekday())]);
            json!({
                "type": "weekly",
                "interval": 1,
                "daysOfWeek": days,
                "firstDayOfWeek": "sunday",
            })
        }
        "MONTHLY" => json!({
            "type": "absoluteMonthly",
            "interval": 1,
            "dayOfMonth": start_date.day(),
        }),
        "YEARLY" => json!({
            "type": "absoluteYearly",
            "interval": 1,
            "dayOfMonth": start_date.day(),
            "month": start_date.month(),
        }),
        _ => return Ok(None),
    };

    let start_str = start_date.format("%Y-%m-%d").to_string();
    let range = match parts.get("UNTIL").and_then(|u| parse_until_date(u)) {
        Some(end_date) => json!({
            "type": "endDate",
            "startDate": start_str,
            "endDate": end_date.format("%Y-%m-%d").to_string(),
        }),
        None => json!({ "type": "noEnd", "startDate": start_str }),
    };

    Ok(Some(json!({ "pattern": pattern, "range": range })))
}

fn parse_rrule(rrule: &str) -> std::collections::HashMap<String, String> {
    rrule
        .split(';')
        .filter_map(|part| {
            let (k, v) = part.split_once('=')?;
            Some((k.trim().to_uppercase(), v.trim().to_string()))
        })
        .collect()
}

fn parse_start_date(iso: &str, is_all_day: bool) -> AppResult<NaiveDate> {
    if is_all_day {
        NaiveDate::parse_from_str(iso, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {iso}: {e}")))
    } else {
        let prefix = iso.get(..10).unwrap_or(iso);
        NaiveDate::parse_from_str(prefix, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid datetime {iso}: {e}")))
    }
}

fn parse_until_date(until: &str) -> Option<NaiveDate> {
    // Accept both YYYYMMDD and YYYYMMDDTHHMMSSZ; we only need the date portion for Graph.
    let date_part = until.split('T').next()?;
    NaiveDate::parse_from_str(date_part, "%Y%m%d").ok()
}

fn byday_to_graph_name(day: &str) -> Option<&'static str> {
    match day.trim().to_uppercase().as_str() {
        "MO" => Some("monday"),
        "TU" => Some("tuesday"),
        "WE" => Some("wednesday"),
        "TH" => Some("thursday"),
        "FR" => Some("friday"),
        "SA" => Some("saturday"),
        "SU" => Some("sunday"),
        _ => None,
    }
}

fn weekday_to_graph_name(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
}

fn build_datetime_pair(draft: &EventDraft) -> AppResult<(JsonValue, JsonValue)> {
    if draft.is_all_day {
        // Graph requires UTC midnight + isAllDay=true. Draft's `end` is inclusive (form
        // convention); convert to exclusive by adding one day.
        let start_date = NaiveDate::parse_from_str(&draft.start, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {}: {e}", draft.start)))?;
        let end_inc = NaiveDate::parse_from_str(&draft.end, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {}: {e}", draft.end)))?;
        let end_exc = end_inc
            .succ_opt()
            .ok_or_else(|| AppError::Other(format!("date overflow at {}", draft.end)))?;
        Ok((
            json!({
                "dateTime": format!("{}T00:00:00", start_date.format("%Y-%m-%d")),
                "timeZone": "UTC",
            }),
            json!({
                "dateTime": format!("{}T00:00:00", end_exc.format("%Y-%m-%d")),
                "timeZone": "UTC",
            }),
        ))
    } else {
        // Drafts arrive as JST RFC3339 (`...+09:00`). Strip the offset and tag the value
        // with `timeZone: "Asia/Tokyo"` — Graph accepts naive datetimes paired with TZ.
        Ok((
            json!({
                "dateTime": strip_offset(&draft.start),
                "timeZone": "Asia/Tokyo",
            }),
            json!({
                "dateTime": strip_offset(&draft.end),
                "timeZone": "Asia/Tokyo",
            }),
        ))
    }
}

fn strip_offset(iso: &str) -> String {
    // "2026-05-15T10:00:00+09:00" or "...Z" → "2026-05-15T10:00:00".
    if iso.len() >= 19 {
        iso[..19].to_string()
    } else {
        iso.to_string()
    }
}

pub async fn create_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let calendar_seg = percent_encode_segment(calendar_id);
    let url = format!("{GRAPH_BASE}/me/calendars/{calendar_seg}/events");
    let payload = build_event_payload(draft)?;
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client
            .post(&url)
            .bearer_auth(token)
            .header("Prefer", r#"outlook.timezone="UTC""#)
            .json(&payload)
    })
    .await?
    .error_for_status()?;
    let event: GraphEvent = resp.json().await?;
    graph_event_to_unified(source_id, calendar_id, event)
        .ok_or_else(|| AppError::Other("Graph create returned event with invalid datetimes".into()))
}

pub async fn update_event(
    source_id: CalendarSourceId,
    event_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let event_seg = percent_encode_segment(event_id);
    // /me/events/{id} edits any event the user owns regardless of which calendar it's in,
    // so we don't need calendar_id in the URL — but we still want the response to carry
    // the calendar_id forward in the UnifiedEvent so the UI keeps it grouped correctly.
    let url = format!("{GRAPH_BASE}/me/events/{event_seg}");
    let payload = build_event_payload(draft)?;
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client
            .patch(&url)
            .bearer_auth(token)
            .header("Prefer", r#"outlook.timezone="UTC""#)
            .json(&payload)
    })
    .await?
    .error_for_status()?;
    let event: GraphEvent = resp.json().await?;
    graph_event_to_unified(source_id, &draft.calendar_id, event)
        .ok_or_else(|| AppError::Other("Graph update returned event with invalid datetimes".into()))
}

pub async fn delete_event(source_id: CalendarSourceId, event_id: &str) -> AppResult<()> {
    let event_seg = percent_encode_segment(event_id);
    let url = format!("{GRAPH_BASE}/me/events/{event_seg}");
    let client = reqwest::Client::new();
    let resp = send_with_refresh(source_id, |token| {
        client.delete(&url).bearer_auth(token)
    })
    .await?;
    let status = resp.status();
    if !status.is_success() && status.as_u16() != 404 {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "Graph DELETE {url} → {status}: {text}"
        )));
    }
    Ok(())
}
