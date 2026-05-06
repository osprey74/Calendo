use crate::calendars;
use crate::error::{AppError, AppResult};
use crate::models::{
    CalendarMeta, CalendarSourceId, EventDraft, EventUpdateRequest, RecurringEditScope,
    UnifiedEvent,
};
use std::collections::HashMap;

fn parse_source(s: &str) -> AppResult<CalendarSourceId> {
    CalendarSourceId::from_str(s).ok_or_else(|| AppError::UnknownSource(s.to_string()))
}

#[tauri::command]
pub async fn calendars_fetch(source_id: String) -> AppResult<Vec<CalendarMeta>> {
    let id = parse_source(&source_id)?;
    calendars::fetch_calendars(id).await
}

/// Fetch events across multiple sources/calendars and merge them.
///
/// `calendar_ids` is a per-source map: each key is a `CalendarSourceId` string (e.g.,
/// `"ms365_work1"`) and the value is the list of calendar IDs from that source to query.
/// A `None` map means "fetch every calendar of every source in `source_ids`". Mixing
/// calendar IDs across sources here would be a routing bug — calendar IDs are scoped
/// to their owning source's API.
///
/// Per-calendar failures (4xx, CalDAV malformed responses) are logged and skipped so a
/// single broken calendar doesn't suppress the rest. Whole-source auth/network failures
/// still propagate so the UI can prompt re-auth.
#[tauri::command]
pub async fn events_fetch(
    source_ids: Vec<String>,
    calendar_ids: Option<HashMap<String, Vec<String>>>,
    date_from: String,
    date_to: String,
) -> AppResult<Vec<UnifiedEvent>> {
    let mut events: Vec<UnifiedEvent> = Vec::new();

    for source_str in source_ids {
        let source_id = parse_source(&source_str)?;

        let target_calendar_ids: Vec<String> = match calendar_ids
            .as_ref()
            .and_then(|m| m.get(&source_str))
        {
            Some(filter) => filter.clone(),
            None => match calendars::fetch_calendars(source_id).await {
                Ok(list) => list.into_iter().map(|c| c.id).collect(),
                Err(e) if is_disconnected_source(&e) => {
                    log::warn!("skipping disconnected source {source_id:?}: {e}");
                    continue;
                }
                Err(e) => {
                    if is_recoverable_per_calendar(&e) {
                        log::warn!("calendars_fetch failed for {source_id:?}: {e}");
                        continue;
                    }
                    return Err(e);
                }
            },
        };

        let mut source_disconnected = false;
        for cal_id in target_calendar_ids {
            if source_disconnected {
                break;
            }
            match calendars::fetch_events(source_id, &cal_id, &date_from, &date_to).await {
                Ok(mut from_source) => events.append(&mut from_source),
                Err(e) if is_disconnected_source(&e) => {
                    log::warn!(
                        "source {source_id:?} disconnected mid-fetch; skipping rest: {e}"
                    );
                    source_disconnected = true;
                }
                Err(e) => {
                    if is_recoverable_per_calendar(&e) {
                        log::warn!(
                            "events fetch failed for {source_id:?} calendar {cal_id}: {e}"
                        );
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    events.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(events)
}

/// True when the error is plausibly per-calendar (HTTP 4xx) rather than a global auth or
/// network outage. Auth/network errors should still propagate so the UI can react.
fn is_recoverable_per_calendar(e: &AppError) -> bool {
    match e {
        AppError::Http(err) => err
            .status()
            .map(|s| s.is_client_error())
            .unwrap_or(false),
        AppError::CalDav(_) => true,
        _ => false,
    }
}

/// True when the source's auth is gone (revoked, never set, or refresh expired). The UI
/// already tracks per-source connection state via `auth_status`, so events_fetch should
/// silently skip these rather than erroring the whole fetch — otherwise disconnecting
/// one source poisons the view for the others that are still connected.
fn is_disconnected_source(e: &AppError) -> bool {
    matches!(
        e,
        AppError::NotAuthenticated(_) | AppError::TokenExpired
    )
}

#[tauri::command]
pub async fn event_create(
    source_id: String,
    calendar_id: String,
    draft: EventDraft,
) -> AppResult<UnifiedEvent> {
    let id = parse_source(&source_id)?;
    calendars::create_event(id, &calendar_id, &draft).await
}

/// Update an existing event. The recurring-scope dialog (this-only / this-and-following /
/// all) is deferred to Phase 4 — for now `recurring_scope` is accepted but ignored, and
/// updates always target the event identified by `event_id` (which on Graph/GCal is the
/// API's per-instance id, on CalDAV the .ics resource URL of the master VEVENT).
#[tauri::command]
#[allow(unused_variables)]
pub async fn event_update(
    source_id: String,
    event_id: String,
    request: EventUpdateRequest,
) -> AppResult<UnifiedEvent> {
    let id = parse_source(&source_id)?;
    calendars::update_event(id, &request.draft.calendar_id, &event_id, &request.draft).await
}

/// Delete an event. As with `event_update`, recurring-scope routing is Phase 4 — current
/// behaviour deletes the event/resource identified by `event_id` directly.
#[tauri::command]
#[allow(unused_variables)]
pub async fn event_delete(
    source_id: String,
    calendar_id: String,
    event_id: String,
    recurring_scope: Option<RecurringEditScope>,
) -> AppResult<()> {
    let id = parse_source(&source_id)?;
    calendars::delete_event(id, &calendar_id, &event_id).await
}
