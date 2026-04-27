pub mod caldav;
pub mod gcal;
pub mod graph;

use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, Protocol};

pub async fn fetch_calendars(source_id: CalendarSourceId) -> AppResult<Vec<CalendarMeta>> {
    match source_id.protocol() {
        Protocol::Graph => graph::fetch_calendars(source_id).await,
        Protocol::GCal => gcal::fetch_calendars(source_id).await,
        Protocol::CalDav => caldav::fetch_calendars(source_id).await,
    }
}

#[allow(dead_code)]
pub async fn fetch_events(
    source_id: CalendarSourceId,
    calendar_id: &str,
    date_from: &str,
    date_to: &str,
) -> AppResult<Vec<crate::models::UnifiedEvent>> {
    match source_id.protocol() {
        Protocol::Graph => graph::fetch_events(source_id, calendar_id, date_from, date_to).await,
        Protocol::GCal => gcal::fetch_events(source_id, calendar_id, date_from, date_to).await,
        Protocol::CalDav => caldav::fetch_events(source_id, calendar_id, date_from, date_to).await,
    }
}

#[allow(dead_code)]
pub fn unsupported_for_now(_source_id: CalendarSourceId) -> AppError {
    AppError::Other("operation not yet implemented for this source".into())
}
