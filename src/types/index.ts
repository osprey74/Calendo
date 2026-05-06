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
