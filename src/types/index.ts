export type CalendarSourceId = "ms365_work1" | "google_gws" | "icloud";

export type ProtocolKind = "graph" | "gcal" | "caldav";

export type AuthStatus = {
  sourceId: CalendarSourceId;
  connected: boolean;
  expiresAt?: number;
};

export type CalendarMeta = {
  id: string;
  sourceId: CalendarSourceId;
  name: string;
  isPrimary: boolean;
  color?: string;
  isWritable: boolean;
  enabled: boolean;
};

export type UnifiedEvent = {
  id: string;
  sourceId: CalendarSourceId;
  calendarId: string;
  title: string;
  /** ISO 8601 in JST (date-only `YYYY-MM-DD` for all-day events). */
  start: string;
  /** ISO 8601 in JST (date-only `YYYY-MM-DD` for all-day events). */
  end: string;
  isAllDay: boolean;
  location?: string;
  body?: string;
  isRecurring: boolean;
  recurringEventId?: string;
  recurrenceRule?: string;
};

export type ClientDebugInfo = {
  msClientId: string | null;
  googleClientId: string | null;
  googleClientSecretConfigured: boolean;
};

export type SourceDescriptor = {
  id: CalendarSourceId;
  label: string;
  color: string;
  protocol: ProtocolKind;
};

export const DEFAULT_SOURCES: SourceDescriptor[] = [
  { id: "ms365_work1", label: "仕事 (Microsoft 365)", color: "#0582AF", protocol: "graph" },
  { id: "google_gws", label: "Google カレンダー", color: "#2E7D32", protocol: "gcal" },
  { id: "icloud", label: "プライベート (iCloud)", color: "#888780", protocol: "caldav" },
];

export type CalendarView = "day" | "week";

export type EventDraft = {
  sourceId: CalendarSourceId;
  calendarId: string;
  title: string;
  /** ISO 8601 in JST. For all-day events, `YYYY-MM-DD` with INCLUSIVE end (form convention). */
  start: string;
  end: string;
  isAllDay: boolean;
  location?: string;
  body?: string;
  /** RFC 5545 RRULE (without the `RRULE:` prefix). Calendo presets emit
   *  `FREQ=DAILY`, `FREQ=WEEKLY`, `FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR`,
   *  `FREQ=MONTHLY`, `FREQ=YEARLY`, optionally with `;UNTIL=YYYYMMDD[T235959Z]`. */
  recurrenceRule?: string;
};

export type RecurringEditScope = "this" | "this_and_following" | "all";

export type EventUpdateRequest = {
  draft: EventDraft;
  recurringScope?: RecurringEditScope;
};

/** Per-calendar user overrides for display attributes (color, label).
 *  Keyed by `${sourceId}|${calendarId}` in the persistence layer. */
export type CalendarOverride = {
  /** When set, replaces the provider-supplied color in UI rendering.
   *  Must be a `#RRGGBB` hex string so existing `hexToBg()` helpers keep working. */
  color?: string;
  /** When set, replaces the provider-supplied calendar name. */
  label?: string;
};

/** Non-fatal problem from a single source/calendar during `events_fetch`. */
export type FetchWarning = {
  sourceId: string;
  calendarId?: string;
  kind: string;
  message: string;
};

export type EventsFetchResult = {
  events: UnifiedEvent[];
  warnings: FetchWarning[];
};
