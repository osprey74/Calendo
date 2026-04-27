import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus, CalendarMeta, CalendarSourceId } from "../types";

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
