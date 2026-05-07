import { invoke } from "@tauri-apps/api/core";
import type {
  AuthStatus,
  CalendarMeta,
  CalendarSourceId,
  ClientDebugInfo,
  EventDraft,
  EventUpdateRequest,
  UnifiedEvent,
} from "../types";

export const authStart = (sourceId: CalendarSourceId) =>
  invoke<AuthStatus>("auth_start", { sourceId });

export const authRefresh = (sourceId: CalendarSourceId) =>
  invoke<AuthStatus>("auth_refresh", { sourceId });

export const authRevoke = (sourceId: CalendarSourceId) =>
  invoke<void>("auth_revoke", { sourceId });

export const authStatus = (sourceId: CalendarSourceId) =>
  invoke<AuthStatus>("auth_status", { sourceId });

export const authIcloudSave = (appleId: string, appPassword: string) =>
  invoke<AuthStatus>("auth_icloud_save", { appleId, appPassword });

export const calendarsFetch = (sourceId: CalendarSourceId) =>
  invoke<CalendarMeta[]>("calendars_fetch", { sourceId });

export const eventsFetch = (
  sourceIds: CalendarSourceId[],
  /** Per-source filter map. Pass `undefined` to query every calendar of every source. */
  calendarIds: Partial<Record<CalendarSourceId, string[]>> | undefined,
  dateFrom: string,
  dateTo: string,
) =>
  invoke<UnifiedEvent[]>("events_fetch", {
    sourceIds,
    calendarIds: calendarIds ?? null,
    dateFrom,
    dateTo,
  });

export const authDebugClients = () => invoke<ClientDebugInfo>("auth_debug_clients");

export const eventCreate = (
  sourceId: CalendarSourceId,
  calendarId: string,
  draft: EventDraft,
) => invoke<UnifiedEvent>("event_create", { sourceId, calendarId, draft });

export const eventUpdate = (
  sourceId: CalendarSourceId,
  eventId: string,
  request: EventUpdateRequest,
) => invoke<UnifiedEvent>("event_update", { sourceId, eventId, request });

export const eventDelete = (
  sourceId: CalendarSourceId,
  calendarId: string,
  eventId: string,
  recurringScope?: import("../types").RecurringEditScope,
) =>
  invoke<void>("event_delete", {
    sourceId,
    calendarId,
    eventId,
    recurringScope: recurringScope ?? null,
  });
