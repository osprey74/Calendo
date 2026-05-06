//! Minimal iCalendar (RFC 5545) parser scoped to what Calendo needs:
//! VEVENT properties (UID, SUMMARY, DESCRIPTION, LOCATION, DTSTART, DTEND,
//! RECURRENCE-ID, RRULE) with TZID parameter handling. Recurrence expansion
//! is delegated to the CalDAV server (`<C:expand>`), so we only parse single
//! instances here.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ICalProperty {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct VEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart: Option<ICalDateTime>,
    pub dtend: Option<ICalDateTime>,
    pub recurrence_id: Option<ICalDateTime>,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ICalDateTime {
    Date(NaiveDate),
    DateTimeUtc(DateTime<Utc>),
    DateTimeLocal {
        naive: NaiveDateTime,
        tzid: Option<String>,
    },
}

const JST_OFFSET_SECS: i32 = 9 * 3600;

impl ICalDateTime {
    /// Convert to a JST ISO 8601 string. Date-only values keep `YYYY-MM-DD`.
    pub fn to_iso_jst(&self) -> String {
        match self {
            ICalDateTime::Date(d) => d.format("%Y-%m-%d").to_string(),
            ICalDateTime::DateTimeUtc(dt) => {
                let jst = chrono::FixedOffset::east_opt(JST_OFFSET_SECS).unwrap();
                dt.with_timezone(&jst)
                    .format("%Y-%m-%dT%H:%M:%S%:z")
                    .to_string()
            }
            ICalDateTime::DateTimeLocal { naive, tzid } => {
                let jst = chrono::FixedOffset::east_opt(JST_OFFSET_SECS).unwrap();
                let dt_jst = match tzid.as_deref() {
                    Some(tz_name) => match tz_name.parse::<Tz>() {
                        Ok(tz) => match tz.from_local_datetime(naive).single() {
                            Some(local) => local.with_timezone(&jst),
                            None => Utc
                                .from_utc_datetime(naive)
                                .with_timezone(&jst),
                        },
                        Err(_) => jst.from_local_datetime(naive).single().unwrap_or_else(|| {
                            Utc.from_utc_datetime(naive).with_timezone(&jst)
                        }),
                    },
                    // Floating local time: assume JST (the user's locale).
                    None => jst.from_local_datetime(naive).single().unwrap_or_else(|| {
                        Utc.from_utc_datetime(naive).with_timezone(&jst)
                    }),
                };
                dt_jst.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
            }
        }
    }

    pub fn is_all_day(&self) -> bool {
        matches!(self, ICalDateTime::Date(_))
    }
}

/// Unfold continuation lines (RFC 5545 §3.1: a CRLF + space/tab continues the previous line).
fn unfold(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed
            .chars()
            .next()
            .map(|c| c == ' ' || c == '\t')
            .unwrap_or(false)
        {
            out.push_str(&trimmed[1..]);
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(trimmed);
        }
    }
    out
}

fn parse_property(line: &str) -> Option<ICalProperty> {
    let colon = line.find(':')?;
    let lhs = &line[..colon];
    let value = line[colon + 1..].to_string();

    let mut parts = lhs.split(';');
    let name = parts.next()?.trim().to_uppercase();
    let mut params: HashMap<String, String> = HashMap::new();
    for p in parts {
        if let Some(eq) = p.find('=') {
            let k = p[..eq].trim().to_uppercase();
            let v = p[eq + 1..].trim().trim_matches('"').to_string();
            params.insert(k, v);
        }
    }
    Some(ICalProperty { name, params, value })
}

fn parse_datetime(prop: &ICalProperty) -> Option<ICalDateTime> {
    let value_type = prop.params.get("VALUE").map(|s| s.as_str());
    if value_type == Some("DATE") {
        let d = NaiveDate::parse_from_str(&prop.value, "%Y%m%d").ok()?;
        return Some(ICalDateTime::Date(d));
    }

    let raw = prop.value.trim();
    if let Some(stripped) = raw.strip_suffix('Z') {
        let n = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").ok()?;
        return Some(ICalDateTime::DateTimeUtc(Utc.from_utc_datetime(&n)));
    }
    let n = NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").ok()?;
    Some(ICalDateTime::DateTimeLocal {
        naive: n,
        tzid: prop.params.get("TZID").cloned(),
    })
}

fn unescape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse all VEVENTs from an iCalendar payload (one or more VCALENDAR blocks accepted).
pub fn parse_vevents(input: &str) -> Vec<VEvent> {
    let unfolded = unfold(input);
    let mut events = Vec::new();
    let mut current: Option<VEvent> = None;

    for line in unfolded.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            current = Some(VEvent::default());
            continue;
        }
        if line.eq_ignore_ascii_case("END:VEVENT") {
            if let Some(ev) = current.take() {
                events.push(ev);
            }
            continue;
        }

        let Some(ev) = current.as_mut() else { continue };
        let Some(prop) = parse_property(line) else { continue };

        match prop.name.as_str() {
            "UID" => ev.uid = prop.value.clone(),
            "SUMMARY" => ev.summary = unescape_text(&prop.value),
            "DESCRIPTION" => ev.description = Some(unescape_text(&prop.value)),
            "LOCATION" => ev.location = Some(unescape_text(&prop.value)),
            "DTSTART" => ev.dtstart = parse_datetime(&prop),
            "DTEND" => ev.dtend = parse_datetime(&prop),
            "RECURRENCE-ID" => ev.recurrence_id = parse_datetime(&prop),
            "RRULE" => ev.rrule = Some(prop.value.clone()),
            _ => {}
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_vevent() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:abc\r\nSUMMARY:Hello\r\nDTSTART:20251101T120000Z\r\nDTEND:20251101T130000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let evs = parse_vevents(ics);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].uid, "abc");
        assert_eq!(evs[0].summary, "Hello");
        assert!(matches!(evs[0].dtstart, Some(ICalDateTime::DateTimeUtc(_))));
    }

    #[test]
    fn parse_all_day() {
        let ics = "BEGIN:VEVENT\r\nUID:1\r\nSUMMARY:Day\r\nDTSTART;VALUE=DATE:20251103\r\nDTEND;VALUE=DATE:20251104\r\nEND:VEVENT";
        let evs = parse_vevents(ics);
        assert!(evs[0].dtstart.as_ref().unwrap().is_all_day());
    }

    #[test]
    fn parse_tzid() {
        let ics = "BEGIN:VEVENT\r\nUID:2\r\nSUMMARY:Tz\r\nDTSTART;TZID=Asia/Tokyo:20251101T120000\r\nDTEND;TZID=Asia/Tokyo:20251101T130000\r\nEND:VEVENT";
        let evs = parse_vevents(ics);
        let s = evs[0].dtstart.as_ref().unwrap().to_iso_jst();
        assert!(s.starts_with("2025-11-01T12:00:00+09:00"));
    }

    #[test]
    fn unfolds_continuation_lines() {
        // RFC 5545 §3.1: CRLF + single SP/HTAB is the fold marker (only the marker is removed).
        let ics = "BEGIN:VEVENT\r\nUID:3\r\nSUMMARY:Long\r\n word\r\nDTSTART:20251101T120000Z\r\nDTEND:20251101T130000Z\r\nEND:VEVENT";
        let evs = parse_vevents(ics);
        assert_eq!(evs[0].summary, "Longword");
    }
}
