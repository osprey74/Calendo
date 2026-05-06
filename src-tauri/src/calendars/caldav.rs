use crate::auth::keyring::{load_icloud, ICloudCredentials};
use crate::calendars::ical::{parse_vevents, ICalDateTime, VEvent};
use crate::calendars::xmlnode::XmlNode;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, UnifiedEvent};
use chrono::NaiveDate;
use reqwest::Method;
use url::Url;

const ICLOUD_BASE: &str = "https://caldav.icloud.com";

fn propfind_method() -> Method {
    Method::from_bytes(b"PROPFIND").unwrap()
}

fn report_method() -> Method {
    Method::from_bytes(b"REPORT").unwrap()
}

async fn caldav_request(
    method: Method,
    url: &str,
    creds: &ICloudCredentials,
    depth: &str,
    body: &str,
) -> AppResult<String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let resp = client
        .request(method.clone(), url)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .header("Depth", depth)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body.to_string())
        .send()
        .await?;

    let status = resp.status();
    let final_url = resp.url().clone();
    let text = resp.text().await?;

    if status.as_u16() != 207 && !status.is_success() {
        return Err(AppError::CalDav(format!(
            "{method} {url} → {status} (final {final_url}) body={}",
            truncate(&text, 300)
        )));
    }
    Ok(text)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

/// Resolve a possibly-relative href against `base`. iCloud sometimes returns absolute
/// URLs (calendar-home-set on a sharded host like p25-caldav.icloud.com), and sometimes
/// relative paths (current-user-principal).
fn resolve_href(base: &str, href: &str) -> AppResult<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(href.to_string());
    }
    let base_url = Url::parse(base)?;
    let resolved = base_url.join(href)?;
    Ok(resolved.to_string())
}

async fn discover_principal(creds: &ICloudCredentials) -> AppResult<String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal/>
  </d:prop>
</d:propfind>"#;

    let xml = caldav_request(propfind_method(), ICLOUD_BASE, creds, "0", body).await?;
    let tree = XmlNode::parse(&xml)?;

    let principal = tree
        .find("current-user-principal")
        .and_then(|n| n.find("href"))
        .map(|h| h.text.trim().to_string())
        .ok_or_else(|| AppError::CalDav("principal href missing".into()))?;

    resolve_href(ICLOUD_BASE, &principal)
}

async fn discover_calendar_home(creds: &ICloudCredentials, principal_url: &str) -> AppResult<String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <c:calendar-home-set/>
  </d:prop>
</d:propfind>"#;

    let xml = caldav_request(propfind_method(), principal_url, creds, "0", body).await?;
    let tree = XmlNode::parse(&xml)?;

    let home = tree
        .find("calendar-home-set")
        .and_then(|n| n.find("href"))
        .map(|h| h.text.trim().to_string())
        .ok_or_else(|| AppError::CalDav("calendar-home-set href missing".into()))?;

    resolve_href(principal_url, &home)
}

#[derive(Debug, Clone)]
struct CalDavCollection {
    href: String,
    display_name: String,
    color: Option<String>,
    is_writable: bool,
    supports_vevent: bool,
}

async fn enumerate_calendars(
    creds: &ICloudCredentials,
    home_url: &str,
) -> AppResult<Vec<CalDavCollection>> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://apple.com/ns/ical/">
  <d:prop>
    <d:resourcetype/>
    <d:displayname/>
    <d:current-user-privilege-set/>
    <c:supported-calendar-component-set/>
    <cs:calendar-color/>
  </d:prop>
</d:propfind>"#;

    let xml = caldav_request(propfind_method(), home_url, creds, "1", body).await?;
    let tree = XmlNode::parse(&xml)?;

    let mut responses: Vec<&XmlNode> = Vec::new();
    tree.find_all("response", &mut responses);

    let mut out = Vec::new();
    for resp in responses {
        let Some(href_node) = resp.find("href") else {
            continue;
        };
        let href = href_node.text.trim();
        if href.is_empty() {
            continue;
        }

        let resourcetype = resp.find("resourcetype");
        let is_calendar = resourcetype.map(|n| n.has_child("calendar")).unwrap_or(false);
        if !is_calendar {
            continue;
        }

        let supported = resp.find("supported-calendar-component-set");
        let supports_vevent = supported
            .map(|n| {
                let mut comps: Vec<&XmlNode> = Vec::new();
                n.find_all("comp", &mut comps);
                comps.iter().any(|c| {
                    // `<comp name="VEVENT"/>` doesn't have text; the XmlNode parser drops attrs,
                    // so we conservatively treat any present <comp> as VEVENT-capable on iCloud
                    // (which only exposes VEVENT/VTODO collections separately).
                    let _ = c;
                    true
                }) || !comps.is_empty()
            })
            .unwrap_or(true);
        if !supports_vevent {
            continue;
        }

        let display_name = resp
            .find("displayname")
            .map(|n| n.text.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| href.trim_end_matches('/').rsplit('/').next().unwrap_or(href).to_string());

        let color = resp
            .find("calendar-color")
            .map(|n| n.text.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(normalize_color);

        let privileges = resp.find("current-user-privilege-set");
        let is_writable = privileges
            .map(|n| {
                let mut privs: Vec<&XmlNode> = Vec::new();
                n.find_all("privilege", &mut privs);
                privs
                    .iter()
                    .any(|p| p.has_child("write") || p.has_child("write-content"))
            })
            .unwrap_or(false);

        let absolute_href = resolve_href(home_url, href).unwrap_or_else(|_| href.to_string());

        out.push(CalDavCollection {
            href: absolute_href,
            display_name,
            color,
            is_writable,
            supports_vevent: true,
        });
    }

    Ok(out)
}

/// Apple's `cs:calendar-color` is `#RRGGBBAA`. Strip the alpha byte for HEX hex compatibility.
fn normalize_color(raw: String) -> String {
    if raw.starts_with('#') && raw.len() == 9 {
        raw[..7].to_string()
    } else {
        raw
    }
}

pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    let principal = discover_principal(&creds).await?;
    let home = discover_calendar_home(&creds, &principal).await?;
    let collections = enumerate_calendars(&creds, &home).await?;

    let metas = collections
        .into_iter()
        .filter(|c| c.supports_vevent)
        .map(|c| CalendarMeta {
            id: c.href,
            source_id,
            name: c.display_name,
            is_primary: false,
            color: c.color,
            is_writable: c.is_writable,
            enabled: true,
        })
        .collect();
    Ok(metas)
}

pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    // CalDAV time-range filter accepts UTC timestamps. Convert YYYY-MM-DD → 00:00:00Z.
    let start_z = format_caldav_utc(date_from, false)?;
    let end_z = format_caldav_utc(date_to, true)?;

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data>
      <c:expand start="{start_z}" end="{end_z}"/>
    </c:calendar-data>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        <c:time-range start="{start_z}" end="{end_z}"/>
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#,
    );

    let xml = caldav_request(report_method(), calendar_id, &creds, "1", &body).await?;
    let tree = XmlNode::parse(&xml)?;

    let mut responses: Vec<&XmlNode> = Vec::new();
    tree.find_all("response", &mut responses);

    let mut events = Vec::new();
    for resp in responses {
        let Some(cal_data) = resp.find("calendar-data") else {
            continue;
        };
        let raw = cal_data.text.trim();
        if raw.is_empty() {
            continue;
        }

        let resource_href = resp
            .find("href")
            .map(|n| n.text.trim().to_string())
            .unwrap_or_default();

        for ve in parse_vevents(raw) {
            if let Some(unified) = vevent_to_unified(source_id, calendar_id, &resource_href, ve) {
                events.push(unified);
            }
        }
    }

    Ok(events)
}

fn format_caldav_utc(date: &str, end_of_day: bool) -> AppResult<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| AppError::Other(format!("invalid date {date}: {e}")))?;
    let suffix = if end_of_day { "T235959Z" } else { "T000000Z" };
    Ok(format!("{}{suffix}", d.format("%Y%m%d")))
}

fn vevent_to_unified(
    source_id: CalendarSourceId,
    calendar_id: &str,
    resource_href: &str,
    ve: VEvent,
) -> Option<UnifiedEvent> {
    let dtstart = ve.dtstart?;
    let dtend = ve.dtend.unwrap_or_else(|| dtstart.clone());

    let is_all_day = dtstart.is_all_day();
    let start = dtstart.to_iso_jst();
    let end = dtend.to_iso_jst();

    // Use UID + RECURRENCE-ID (if any) so expanded instances get unique IDs.
    let id = match &ve.recurrence_id {
        Some(rid) => format!("{}::{}", ve.uid, rid_key(rid)),
        None => {
            if !ve.uid.is_empty() {
                ve.uid.clone()
            } else {
                resource_href.to_string()
            }
        }
    };

    Some(UnifiedEvent {
        id,
        source_id,
        calendar_id: calendar_id.to_string(),
        title: ve.summary,
        start,
        end,
        is_all_day,
        location: ve.location,
        body: ve.description,
        is_recurring: ve.rrule.is_some() || ve.recurrence_id.is_some(),
        recurring_event_id: ve.recurrence_id.is_some().then(|| ve.uid.clone()),
        recurrence_rule: ve.rrule,
    })
}

fn rid_key(d: &ICalDateTime) -> String {
    match d {
        ICalDateTime::Date(d) => d.format("%Y%m%d").to_string(),
        ICalDateTime::DateTimeUtc(dt) => dt.format("%Y%m%dT%H%M%SZ").to_string(),
        ICalDateTime::DateTimeLocal { naive, .. } => naive.format("%Y%m%dT%H%M%S").to_string(),
    }
}
