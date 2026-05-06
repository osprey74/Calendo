pub mod caldav;
pub mod gcal;
pub mod graph;
pub mod ical;
pub mod util;
pub mod xmlnode;

use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, Protocol, UnifiedEvent};

pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    match source_id.protocol() {
        Protocol::Graph => graph::fetch_calendars(source_id).await,
        Protocol::GCal => gcal::fetch_calendars(source_id).await,
        Protocol::CalDav => caldav::fetch_calendars(source_id).await,
    }
}

pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<UnifiedEvent>> {
    match source_id.protocol() {
        Protocol::Graph => graph::fetch_events(source_id, calendar_id, date_from, date_to).await,
        Protocol::GCal => gcal::fetch_events(source_id, calendar_id, date_from, date_to).await,
        Protocol::CalDav => caldav::fetch_events(source_id, calendar_id, date_from, date_to).await,
    }
}
