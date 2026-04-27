export type CalendarSourceId =
  | "ms365_work1"
  | "ms365_work2"
  | "google_gws"
  | "icloud";

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

export type SourceDescriptor = {
  id: CalendarSourceId;
  label: string;
  color: string;
  protocol: "graph" | "gcal" | "caldav";
};

export const DEFAULT_SOURCES: SourceDescriptor[] = [
  { id: "ms365_work1", label: "仕事 A (Microsoft 365)", color: "#0582AF", protocol: "graph" },
  { id: "ms365_work2", label: "仕事 B (Microsoft 365)", color: "#E65C00", protocol: "graph" },
  { id: "google_gws", label: "Google カレンダー", color: "#2E7D32", protocol: "gcal" },
  { id: "icloud", label: "プライベート (iCloud)", color: "#888780", protocol: "caldav" },
];
