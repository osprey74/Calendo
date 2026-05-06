pub mod caldav;
pub mod gcal;
pub mod graph;
pub mod ical;
pub mod util;
pub mod xmlnode;

use crate::error::AppResult;
use crate::models::{CalendarMeta, CalendarSourceId, EventDraft, Protocol, UnifiedEvent};

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

pub async fn create_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    match source_id.protocol() {
        Protocol::Graph => graph::create_event(source_id, calendar_id, draft).await,
        Protocol::GCal => gcal::create_event(source_id, calendar_id, draft).await,
        Protocol::CalDav => caldav::create_event(source_id, calendar_id, draft).await,
    }
}

pub async fn update_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    event_id: &str,
    draft: &EventDraft,
) -> AppResult<UnifiedEvent> {
    match source_id.protocol() {
        Protocol::Graph => graph::update_event(source_id, event_id, draft).await,
        Protocol::GCal => gcal::update_event(source_id, calendar_id, event_id, draft).await,
        Protocol::CalDav => caldav::update_event(source_id, event_id, draft).await,
    }
}

pub async fn delete_event(
    source_id: CalendarSourceId,
    calendar_id: &str,
    event_id: &str,
) -> AppResult<()> {
    match source_id.protocol() {
        Protocol::Graph => graph::delete_event(source_id, event_id).await,
        Protocol::GCal => gcal::delete_event(source_id, calendar_id, event_id).await,
        Protocol::CalDav => caldav::delete_event(source_id, event_id).await,
    }
}
