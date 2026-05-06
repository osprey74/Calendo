use crate::auth::oauth::ensure_fresh;
use crate::calendars::util::percent_encode_segment;
use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

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
    let tokens = ensure_fresh(source_id).await?;
    let client = reqwest::Client::new();

    // Default page size for /me/calendars is 10. Bump to 100 and walk nextLink so
    // mailboxes with many shared/own calendars surface every entry.
    let mut url = Some(format!("{GRAPH_BASE}/me/calendars?$top=100"));
    let mut calendars: Vec<CalendarMeta> = Vec::new();

    while let Some(next) = url.take() {
        let resp = client
            .get(&next)
            .bearer_auth(&tokens.access_token)
            .send()
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
    let tokens = ensure_fresh(source_id).await?;
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
        let resp = client
            .get(&next)
            .bearer_auth(&tokens.access_token)
            // Outlook: tell Graph to return times in UTC for predictable parsing.
            .header("Prefer", r#"outlook.timezone="UTC""#)
            .send()
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
