use crate::auth::keyring::{load_icloud, ICloudCredentials};
use crate::calendars::ical::{parse_vevents, ICalDateTime, VEvent};
use crate::calendars::xmlnode::XmlNode;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarSourceId, CalendarMeta, EventDraft, UnifiedEvent};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rand::RngCore;
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
        // 401 against CalDAV means the stored app-specific password is no longer valid
        // (revoked from appleid.apple.com or rotated). Surface as AuthRequired so the
        // frontend can prompt the user to re-enter credentials for iCloud specifically.
        if status.as_u16() == 401 {
            return Err(AppError::AuthRequired("icloud".into()));
        }
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            message: format!(
                "{method} {url} → {status} (final {final_url}) body={}",
                truncate(&text, 300)
            ),
        });
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

        let raw_href = resp
            .find("href")
            .map(|n| n.text.trim().to_string())
            .unwrap_or_default();
        // Resolve the href against the calendar URL so the resulting id is a full URL
        // we can later PUT/DELETE against. iCloud REPORT returns absolute paths
        // (e.g. `/12345/calendars/abc/uuid.ics`) without the host.
        let resource_href = resolve_href(calendar_id, &raw_href).unwrap_or(raw_href);

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

    // Use the .ics resource URL as the canonical id so write commands (PUT/DELETE) can
    // target it directly. Expanded recurring instances share the same resource URL, so
    // append a `::<recurrence-id>` discriminator to keep them distinct in the UI store.
    let id = match &ve.recurrence_id {
        Some(rid) => format!("{}::{}", resource_href, rid_key(rid)),
        None => resource_href.to_string(),
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

// --- Write operations -------------------------------------------------------

/// Strip the optional `::recurrence-id` discriminator we append to recurring instance ids.
/// Write operations always target the underlying .ics resource URL.
fn caldav_resource_url(event_id: &str) -> &str {
    event_id.split("::").next().unwrap_or(event_id)
}

fn generate_uid() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut hex = String::with_capacity(32);
    for b in &bytes {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("{}@calendo", hex)
}

fn ical_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

/// Convert a JST ISO 8601 string ("2026-05-15T10:00:00+09:00") to UTC iCalendar
/// form ("20260515T010000Z"). UTC is preferred over TZID so we don't need to emit
/// a VTIMEZONE block alongside every event.
fn jst_iso_to_ical_utc(iso: &str) -> AppResult<String> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return Ok(dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string());
    }
    // Fallback: naive datetime is assumed to be JST (matches our store convention).
    let naive = if iso.len() >= 19 { &iso[..19] } else { iso };
    let n = NaiveDateTime::parse_from_str(naive, "%Y-%m-%dT%H:%M:%S")
        .map_err(|e| AppError::Other(format!("invalid datetime {iso}: {e}")))?;
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    let dt = jst
        .from_local_datetime(&n)
        .single()
        .ok_or_else(|| AppError::Other(format!("ambiguous local time {iso}")))?;
    Ok(dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string())
}

/// Convert an inclusive YYYY-MM-DD all-day end into the exclusive iCalendar/RFC 5545
/// form (next day in YYYYMMDD). Drafts use inclusive ends so the form layer matches
/// what the user typed; iCalendar / Graph / GCal all expect exclusive ends on the wire.
fn inclusive_end_to_ical_date(date: &str) -> AppResult<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| AppError::Other(format!("invalid date {date}: {e}")))?;
    let next = d
        .succ_opt()
        .ok_or_else(|| AppError::Other(format!("date overflow at {date}")))?;
    Ok(next.format("%Y%m%d").to_string())
}

fn build_vcalendar(uid: &str, draft: &EventDraft, sequence: u32) -> AppResult<String> {
    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//Calendo//Calendo 0.1//EN\r\n");
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("BEGIN:VEVENT\r\n");
    out.push_str(&format!("UID:{}\r\n", uid));
    out.push_str(&format!("DTSTAMP:{}\r\n", dtstamp));
    out.push_str(&format!("SEQUENCE:{}\r\n", sequence));
    out.push_str(&format!("SUMMARY:{}\r\n", ical_escape(&draft.title)));

    if draft.is_all_day {
        let start = NaiveDate::parse_from_str(&draft.start, "%Y-%m-%d")
            .map_err(|e| AppError::Other(format!("invalid date {}: {e}", draft.start)))?;
        let end_exc = inclusive_end_to_ical_date(&draft.end)?;
        out.push_str(&format!(
            "DTSTART;VALUE=DATE:{}\r\n",
            start.format("%Y%m%d")
        ));
        out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end_exc));
    } else {
        let start_utc = jst_iso_to_ical_utc(&draft.start)?;
        let end_utc = jst_iso_to_ical_utc(&draft.end)?;
        out.push_str(&format!("DTSTART:{}\r\n", start_utc));
        out.push_str(&format!("DTEND:{}\r\n", end_utc));
    }

    if let Some(loc) = &draft.location {
        if !loc.is_empty() {
            out.push_str(&format!("LOCATION:{}\r\n", ical_escape(loc)));
        }
    }
    if let Some(body) = &draft.body {
        if !body.is_empty() {
            out.push_str(&format!("DESCRIPTION:{}\r\n", ical_escape(body)));
        }
    }
    if let Some(rrule) = &draft.recurrence_rule {
        if !rrule.is_empty() {
            out.push_str(&format!("RRULE:{}\r\n", rrule));
        }
    }

    out.push_str("END:VEVENT\r\n");
    out.push_str("END:VCALENDAR\r\n");
    Ok(out)
}

async fn put_ics(creds: &ICloudCredentials, url: &str, ics: &str, expect_new: bool) -> AppResult<()> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let mut req = client
        .put(url)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(ics.to_string());
    if expect_new {
        // If-None-Match: * makes PUT atomic-create — server rejects if a resource already
        // exists at that URL, so a UID collision can't accidentally overwrite something.
        req = req.header("If-None-Match", "*");
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err(AppError::AuthRequired("icloud".into()));
        }
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            message: format!("PUT {url} → {status}: {}", truncate(&text, 300)),
        });
    }
    Ok(())
}

async fn fetch_existing_ics(creds: &ICloudCredentials, url: &str) -> AppResult<String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let resp = client
        .get(url)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err(AppError::AuthRequired("icloud".into()));
        }
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            message: format!(
                "GET {url} → {status} (cannot read existing event for update)"
            ),
        });
    }
    Ok(resp.text().await?)
}

fn extract_uid(ics: &str) -> Option<String> {
    for line in ics.lines() {
        if let Some(rest) = line.strip_prefix("UID:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn extract_rrule(ics: &str) -> Option<String> {
    for line in ics.lines() {
        if let Some(rest) = line.strip_prefix("RRULE:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

pub async fn create_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    let uid = generate_uid();
    let resource_url = format!(
        "{}/{}.ics",
        calendar_id.trim_end_matches('/'),
        urlencoded_filename(&uid)
    );
    let ics = build_vcalendar(&uid, draft, 0)?;
    put_ics(&creds, &resource_url, &ics, true).await?;

    Ok(unified_from_draft(source_id, calendar_id, &resource_url, draft))
}

pub async fn update_event(
    source_id: CalendarSourceId,
    event_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    let resource_url = caldav_resource_url(event_id).to_string();

    // Re-use the existing UID so the server treats this as an update of the same event
    // rather than a new event sharing a URL. If the GET fails (e.g., the resource was
    // already deleted server-side) generate a fresh UID and PUT as a create.
    let existing = fetch_existing_ics(&creds, &resource_url).await.ok();
    let uid = existing
        .as_deref()
        .and_then(extract_uid)
        .unwrap_or_else(generate_uid);

    // Preserve the original RRULE if the draft doesn't supply one. Without this every
    // update would silently flatten a recurring event into a single occurrence because
    // build_vcalendar rebuilds the .ics from scratch.
    let mut effective = draft.clone();
    if effective.recurrence_rule.is_none() {
        if let Some(existing_rrule) = existing.as_deref().and_then(extract_rrule) {
            effective.recurrence_rule = Some(existing_rrule);
        }
    }

    let ics = build_vcalendar(&uid, &effective, 1)?;
    put_ics(&creds, &resource_url, &ics, false).await?;

    Ok(unified_from_draft(
        source_id,
        &draft.calendar_id,
        &resource_url,
        draft,
    ))
}

pub async fn delete_event(source_id: CalendarSourceId, event_id: &str) -> AppResult<()> {
    let creds = load_icloud()?
        .ok_or_else(|| AppError::NotAuthenticated(source_id.as_str().into()))?;

    let resource_url = caldav_resource_url(event_id);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let resp = client
        .delete(resource_url)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .send()
        .await?;

    let status = resp.status();
    // 404 is acceptable for delete (someone else removed it; goal achieved).
    if !status.is_success() && status.as_u16() != 404 {
        if status.as_u16() == 401 {
            return Err(AppError::AuthRequired("icloud".into()));
        }
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            message: format!("DELETE {resource_url} → {status}"),
        });
    }
    Ok(())
}

fn urlencoded_filename(uid: &str) -> String {
    // UID with `@` is fine inside a path segment, but encode just to be defensive about
    // any other reserved chars iCloud might balk at.
    crate::calendars::util::percent_encode_segment(uid)
}

/// Build a UnifiedEvent from a draft after a successful write. We keep the API contract
/// of returning the canonical event so the frontend can splice it into the store without
/// always reloading. End timestamps follow the read-side convention (exclusive for
/// all-day, RFC3339 with offset for timed).
fn unified_from_draft(
    source_id: CalendarSourceId,
    calendar_id: &str,
    resource_url: &str,
    draft: &EventDraft,
) -> UnifiedEvent {
    let end = if draft.is_all_day {
        // Convert inclusive (form) → exclusive (read-side convention) so DayView/WeekView
        // overlap checks behave consistently after a create/edit.
        match NaiveDate::parse_from_str(&draft.end, "%Y-%m-%d") {
            Ok(d) => d
                .succ_opt()
                .map(|n| n.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| draft.end.clone()),
            Err(_) => draft.end.clone(),
        }
    } else {
        draft.end.clone()
    };
    UnifiedEvent {
        id: resource_url.to_string(),
        source_id,
        calendar_id: calendar_id.to_string(),
        title: draft.title.clone(),
        start: draft.start.clone(),
        end,
        is_all_day: draft.is_all_day,
        location: draft.location.clone(),
        body: draft.body.clone(),
        is_recurring: false,
        recurring_event_id: None,
        recurrence_rule: None,
    }
}

