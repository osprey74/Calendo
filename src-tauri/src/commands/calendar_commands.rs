use crate::calendars;
use crate::error::{AppError, AppResult};
use crate::models::{CalendarMeta, CalendarSourceId, EventDraft, EventUpdateRequest, RecurringEditScope, UnifiedEvent};

fn parse_source(s: &str) -> AppResult<CalendarSourceId> {
    CalendarSourceId::from_str(s).ok_or_else(|| AppError::UnknownSource(s.to_string()))
}

#[tauri::command]
pub async fn calendars_fetch(source_id: String) -> AppResult<Vec<CalendarMeta>> {
    let id = parse_source(&source_id)?;
    calendars::fetch_calendars(id).await
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn events_fetch(
    source_ids: Vec<String>,
    calendar_ids: Option<Vec<String>>,
    date_from: String,
    date_to: String,
) -> AppResult<Vec<UnifiedEvent>> {
    // Phase 2: aggregate events across the provided source/calendar ids.
    Ok(Vec::new())
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn event_create(
    source_id: String,
    calendar_id: String,
    draft: EventDraft,
) -> AppResult<UnifiedEvent> {
    Err(AppError::Other("event_create not yet implemented (Phase 3)".into()))
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn event_update(
    source_id: String,
    event_id: String,
    request: EventUpdateRequest,
) -> AppResult<UnifiedEvent> {
    Err(AppError::Other("event_update not yet implemented (Phase 3)".into()))
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn event_delete(
    source_id: String,
    event_id: String,
    recurring_scope: Option<RecurringEditScope>,
) -> AppResult<()> {
    Err(AppError::Other("event_delete not yet implemented (Phase 3)".into()))
}
