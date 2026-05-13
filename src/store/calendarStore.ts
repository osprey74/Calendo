import { create } from "zustand";
import type {
  CalendarMeta,
  CalendarOverride,
  CalendarSourceId,
  CalendarView,
  EventDraft,
  UnifiedEvent,
} from "../types";
import { DEFAULT_SOURCES } from "../types";
import { addDays, startOfWeekJst, ymd } from "../utils/dateUtils";
import {
  calendarsFetch,
  eventCreate,
  eventDelete,
  eventUpdate,
  eventsFetch,
} from "../lib/tauri";
import {
  loadSettings,
  saveCalendarEnabled,
  saveCalendarOverrides,
  saveHourHeightPx,
  saveSourceEnabled,
  saveView,
  saveViewHours,
} from "../lib/persistence";
import { classifyError } from "../lib/errors";
import { toast } from "./toastStore";

type RecordBy<T extends string, V> = Record<T, V>;

type FetchKey = string;

/** Vertical zoom presets — px per hour. 60 (the legacy fixed height) is the default.
 *  Anything outside this list is clamped to the nearest preset on hydrate to keep the
 *  UI predictable; the +/- buttons step one slot at a time. */
export const HOUR_HEIGHT_LEVELS: readonly number[] = [
  40, 60, 80, 100, 120, 160, 200, 240,
] as const;
export const DEFAULT_HOUR_HEIGHT_PX = 60;

/** Bounds of the visible time-of-day window in the day/week grid. Inclusive start,
 *  exclusive end (so `[0, 24]` shows the whole day). Hours-only — minute granularity
 *  is deferred until there's a demand for it. */
export const DEFAULT_VIEW_START_HOUR = 0;
export const DEFAULT_VIEW_END_HOUR = 24;

type State = {
  view: CalendarView;
  /** Anchor date (current day for DayView, any day in the week for WeekView). */
  anchor: Date;
  /** Vertical px per hour for the day/week time grid. Persisted. */
  hourHeightPx: number;
  /** First hour of the day to display (0-23). Default 0. Persisted. */
  viewStartHour: number;
  /** Exclusive end hour (1-24). Must be greater than `viewStartHour`. Default 24. */
  viewEndHour: number;

  /** Calendars known per source (null = not yet fetched). */
  calendars: RecordBy<CalendarSourceId, CalendarMeta[] | null>;
  /** Source-level enable toggle (UI filter). */
  sourceEnabled: RecordBy<CalendarSourceId, boolean>;
  /** Sub-calendar enable toggle, keyed by `${source}|${calendarId}`. */
  calendarEnabled: Record<string, boolean>;
  /** Per-calendar display overrides (color, label), keyed by `${source}|${calendarId}`.
   *  Persisted via the Tauri store and applied at render time via `effectiveCalendarMeta()`. */
  calendarOverrides: Record<string, CalendarOverride>;

  events: UnifiedEvent[];
  loadedRange: { from: string; to: string } | null;
  loading: boolean;
  error: string | null;
  /** Increments on every successful refresh — useful for views that memoize. */
  revision: number;
};

type Actions = {
  setView: (v: CalendarView) => void;
  setAnchor: (d: Date) => void;
  shiftAnchor: (delta: number) => void;
  goToToday: () => void;

  toggleSource: (id: CalendarSourceId) => void;
  toggleCalendar: (sourceId: CalendarSourceId, calendarId: string) => void;
  setAllCalendarsEnabled: (sourceId: CalendarSourceId, enabled: boolean) => void;

  /** Set the hour-height to the nearest preset of `HOUR_HEIGHT_LEVELS`. */
  setHourHeightPx: (px: number) => void;
  /** Step zoom by one preset slot. `+1` zooms in, `-1` zooms out. Out-of-range steps
   *  clamp at the endpoints rather than wrapping. */
  stepHourHeight: (delta: 1 | -1) => void;

  /** Update the visible time-of-day window. The setter enforces `start < end` and clamps
   *  to `[0, 24]`; an inverted/empty range is rejected silently so callers don't need to
   *  guard the dropdown values. */
  setViewHours: (startHour: number, endHour: number) => void;

  /** Upsert a per-calendar display override. Pass `{}` (or undefined fields) to clear. */
  setCalendarOverride: (
    sourceId: CalendarSourceId,
    calendarId: string,
    patch: CalendarOverride,
  ) => void;
  /** Drop the override entirely so the provider-supplied values take over again. */
  clearCalendarOverride: (sourceId: CalendarSourceId, calendarId: string) => void;

  loadCalendars: (sourceId: CalendarSourceId) => Promise<void>;
  loadEvents: () => Promise<void>;
  invalidate: () => void;
  /** Reset cached calendar list for a source (e.g., after disconnect) so the events
   *  fetch and the onboarding-state detection treat the source as fresh. */
  clearSourceCalendars: (sourceId: CalendarSourceId) => void;

  /** Pull persisted settings off disk and merge into state. Should be awaited before
   *  the first `loadEvents()` so the events fetch reflects user-saved filters. */
  hydrate: () => Promise<void>;

  createEvent: (draft: EventDraft) => Promise<UnifiedEvent>;
  updateEvent: (
    eventId: string,
    draft: EventDraft,
    scope?: import("../types").RecurringEditScope,
  ) => Promise<UnifiedEvent>;
  deleteEvent: (
    sourceId: CalendarSourceId,
    calendarId: string,
    eventId: string,
    scope?: import("../types").RecurringEditScope,
  ) => Promise<void>;
};

export type CalendarStore = State & Actions;

function defaultSourceEnabled(): RecordBy<CalendarSourceId, boolean> {
  return DEFAULT_SOURCES.reduce(
    (acc, s) => {
      acc[s.id] = true;
      return acc;
    },
    {} as RecordBy<CalendarSourceId, boolean>,
  );
}

function calKey(sourceId: CalendarSourceId, calendarId: string): FetchKey {
  return `${sourceId}|${calendarId}`;
}

/** Snap an arbitrary px value to the nearest preset. Persistence may round-trip values
 *  saved by older builds (or future levels) — clamping here keeps the runtime UI
 *  predictable. */
function nearestHourLevel(px: number): number {
  let best = HOUR_HEIGHT_LEVELS[0];
  let bestDelta = Math.abs(px - best);
  for (const v of HOUR_HEIGHT_LEVELS) {
    const d = Math.abs(px - v);
    if (d < bestDelta) {
      best = v;
      bestDelta = d;
    }
  }
  return best;
}

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  view: "week",
  anchor: new Date(),
  hourHeightPx: DEFAULT_HOUR_HEIGHT_PX,
  viewStartHour: DEFAULT_VIEW_START_HOUR,
  viewEndHour: DEFAULT_VIEW_END_HOUR,

  calendars: {
    ms365_work1: null,
    google_gws: null,
    icloud: null,
  },
  sourceEnabled: defaultSourceEnabled(),
  calendarEnabled: {},
  calendarOverrides: {},

  events: [],
  loadedRange: null,
  loading: false,
  error: null,
  revision: 0,

  setView: (v) => {
    if (get().view === v) return;
    set({ view: v });
    void saveView(v);
    void get().loadEvents();
  },

  setAnchor: (d) => {
    set({ anchor: d });
    void get().loadEvents();
  },

  shiftAnchor: (delta) => {
    const { view, anchor } = get();
    const step = view === "day" ? delta : delta * 7;
    set({ anchor: addDays(anchor, step) });
    void get().loadEvents();
  },

  goToToday: () => {
    set({ anchor: new Date() });
    void get().loadEvents();
  },

  toggleSource: (id) => {
    set((state) => {
      const next = { ...state.sourceEnabled, [id]: !state.sourceEnabled[id] };
      void saveSourceEnabled(next);
      return { sourceEnabled: next };
    });
  },

  toggleCalendar: (sourceId, calendarId) => {
    const key = calKey(sourceId, calendarId);
    set((state) => {
      const current = state.calendarEnabled[key] ?? true;
      const next = { ...state.calendarEnabled, [key]: !current };
      void saveCalendarEnabled(next);
      return { calendarEnabled: next };
    });
  },

  setAllCalendarsEnabled: (sourceId, enabled) => {
    set((state) => {
      const list = state.calendars[sourceId] ?? [];
      const next = { ...state.calendarEnabled };
      for (const c of list) {
        next[calKey(sourceId, c.id)] = enabled;
      }
      void saveCalendarEnabled(next);
      return { calendarEnabled: next };
    });
  },

  setHourHeightPx: (px) => {
    const clamped = nearestHourLevel(px);
    if (get().hourHeightPx === clamped) return;
    set({ hourHeightPx: clamped });
    void saveHourHeightPx(clamped);
  },

  stepHourHeight: (delta) => {
    const current = get().hourHeightPx;
    const idx = HOUR_HEIGHT_LEVELS.indexOf(nearestHourLevel(current));
    const nextIdx = Math.max(0, Math.min(HOUR_HEIGHT_LEVELS.length - 1, idx + delta));
    const next = HOUR_HEIGHT_LEVELS[nextIdx];
    if (next === current) return;
    set({ hourHeightPx: next });
    void saveHourHeightPx(next);
  },

  setViewHours: (startHour, endHour) => {
    const s = Math.max(0, Math.min(23, Math.trunc(startHour)));
    const e = Math.max(1, Math.min(24, Math.trunc(endHour)));
    if (e <= s) return; // ignore inverted/empty windows silently
    const prev = get();
    if (prev.viewStartHour === s && prev.viewEndHour === e) return;
    set({ viewStartHour: s, viewEndHour: e });
    void saveViewHours(s, e);
  },

  setCalendarOverride: (sourceId, calendarId, patch) => {
    const key = calKey(sourceId, calendarId);
    set((state) => {
      const prev = state.calendarOverrides[key] ?? {};
      const merged: CalendarOverride = { ...prev, ...patch };
      // Strip empty fields so the persisted blob stays minimal and `hasOverride`
      // checks below stay accurate.
      if (merged.color === undefined || merged.color === "") delete merged.color;
      if (merged.label === undefined || merged.label === "") delete merged.label;
      const next = { ...state.calendarOverrides };
      if (merged.color === undefined && merged.label === undefined) {
        delete next[key];
      } else {
        next[key] = merged;
      }
      void saveCalendarOverrides(next);
      return { calendarOverrides: next };
    });
  },

  clearCalendarOverride: (sourceId, calendarId) => {
    const key = calKey(sourceId, calendarId);
    set((state) => {
      if (!(key in state.calendarOverrides)) return {};
      const next = { ...state.calendarOverrides };
      delete next[key];
      void saveCalendarOverrides(next);
      return { calendarOverrides: next };
    });
  },

  loadCalendars: async (sourceId) => {
    try {
      const list = await calendarsFetch(sourceId);
      set((state) => ({
        calendars: { ...state.calendars, [sourceId]: list },
      }));
    } catch (e) {
      // Per-source errors are expected pre-auth (NotAuthenticated). Don't toast those —
      // the UI already shows the source as "未接続" via authStatus, so adding noise on
      // every refresh would be annoying. Auth-expired (auth_required) gets a one-shot
      // toast since the user thought they were connected.
      const c = classifyError(e);
      if (c.kind === "auth_required") {
        toast.error(c.userMessage);
      }
      set({ error: `${sourceId}: ${c.userMessage}` });
    }
  },

  loadEvents: async () => {
    const { anchor, view, sourceEnabled, calendarEnabled, calendars } = get();

    const enabledSources = (Object.keys(sourceEnabled) as CalendarSourceId[]).filter(
      (s) => sourceEnabled[s],
    );
    if (enabledSources.length === 0) {
      set({ events: [], loading: false, error: null, loadedRange: null });
      return;
    }

    let from: Date;
    let to: Date;
    if (view === "day") {
      from = anchor;
      to = anchor;
    } else {
      from = startOfWeekJst(anchor);
      to = addDays(from, 6);
    }
    const dateFrom = ymd(from);
    const dateTo = ymd(to);

    // Build the per-source calendar filter map. Calendar IDs are scoped to their owning
    // source (a Graph ID is meaningless to the iCloud CalDAV client), so the filter must
    // not flatten across sources.
    for (const sid of enabledSources) {
      const list = calendars[sid];
      if (!list) {
        // Calendars haven't been loaded for this source yet — fetch them on demand
        // so the user can drive the app without manually loading each side first.
        try {
          await get().loadCalendars(sid);
        } catch {
          // already surfaced via state.error
        }
      }
    }
    const calendarsAfter = get().calendars;
    const filterMap: Partial<Record<CalendarSourceId, string[]>> = {};
    let anyEnabled = false;
    for (const sid of enabledSources) {
      const list = calendarsAfter[sid] ?? [];
      const enabledIds = list
        .filter((c) => (calendarEnabled[calKey(sid, c.id)] ?? true))
        .map((c) => c.id);
      if (enabledIds.length > 0) {
        filterMap[sid] = enabledIds;
        anyEnabled = true;
      }
    }

    // When a source's calendars haven't loaded yet, leave it out of the map — the backend
    // enumerates that source's calendars via `calendars_fetch`. If no source has any
    // explicit filter (rare, but possible if `loadCalendars` failed everywhere), fall
    // through to backend enumeration entirely.
    const calendarFilter = anyEnabled ? filterMap : undefined;

    set({ loading: true, error: null });
    try {
      const result = await eventsFetch(enabledSources, calendarFilter, dateFrom, dateTo);
      set((state) => ({
        events: result.events,
        loadedRange: { from: dateFrom, to: dateTo },
        loading: false,
        revision: state.revision + 1,
      }));
      // Per-source/per-calendar warnings: things that didn't fail the whole fetch but
      // mean some events are missing. Aggregate by source so a single auth expiry on a
      // multi-calendar account becomes one toast rather than N. Auth issues get the
      // "再ログインしてください" treatment; other warnings stay informational.
      if (result.warnings.length > 0) {
        const seen = new Set<string>();
        for (const w of result.warnings) {
          const key = `${w.sourceId}|${w.kind}`;
          if (seen.has(key)) continue;
          seen.add(key);
          const c = classifyError(w);
          if (c.kind === "auth_required") {
            toast.error(c.userMessage);
          } else if (c.kind !== "not_authenticated") {
            // not_authenticated is expected for never-connected sources — suppress to
            // avoid spamming a toast every refresh.
            toast.info(c.userMessage);
          }
        }
      }
    } catch (e) {
      // Don't clobber `events` — keep the existing cache visible so a transient
      // network blip doesn't blank the calendar. The `error` field is still set so
      // AppShell can show the banner.
      const c = classifyError(e);
      set({ loading: false, error: c.userMessage });
      toast.error(c.userMessage);
    }
  },

  invalidate: () => {
    set({ events: [], loadedRange: null });
  },

  clearSourceCalendars: (sourceId) => {
    set((state) => ({
      calendars: { ...state.calendars, [sourceId]: null },
    }));
  },

  hydrate: async () => {
    const persisted = await loadSettings();
    set((state) => {
      const merged: Partial<State> = {
        view: persisted.view ?? state.view,
        sourceEnabled: persisted.sourceEnabled
          ? { ...state.sourceEnabled, ...persisted.sourceEnabled }
          : state.sourceEnabled,
        calendarEnabled: persisted.calendarEnabled ?? state.calendarEnabled,
        calendarOverrides: persisted.calendarOverrides ?? state.calendarOverrides,
        hourHeightPx:
          persisted.hourHeightPx != null
            ? nearestHourLevel(persisted.hourHeightPx)
            : state.hourHeightPx,
      };
      // Visible-window: both values must be present, otherwise we ignore and keep the
      // current (default) range. Persisted ranges that ended up inverted by a future
      // bug get silently dropped.
      if (
        typeof persisted.viewStartHour === "number" &&
        typeof persisted.viewEndHour === "number" &&
        persisted.viewEndHour > persisted.viewStartHour
      ) {
        merged.viewStartHour = Math.max(0, Math.min(23, Math.trunc(persisted.viewStartHour)));
        merged.viewEndHour = Math.max(1, Math.min(24, Math.trunc(persisted.viewEndHour)));
      }
      return merged;
    });
  },

  createEvent: async (draft) => {
    try {
      const created = await eventCreate(draft.sourceId, draft.calendarId, draft);
      // Refresh the current visible window so recurring expansions / server-side adjustments
      // (e.g., Graph rounding fractional seconds) are picked up rather than relying on the
      // optimistic local insert.
      await get().loadEvents();
      toast.success(`「${draft.title}」を作成しました`);
      return created;
    } catch (e) {
      const c = classifyError(e);
      toast.error(`作成に失敗しました: ${c.userMessage}`);
      throw e;
    }
  },

  updateEvent: async (eventId, draft, scope) => {
    try {
      const updated = await eventUpdate(draft.sourceId, eventId, {
        draft,
        recurringScope: scope,
      });
      await get().loadEvents();
      toast.success(`「${draft.title}」を更新しました`);
      return updated;
    } catch (e) {
      const c = classifyError(e);
      toast.error(`更新に失敗しました: ${c.userMessage}`);
      throw e;
    }
  },

  deleteEvent: async (sourceId, calendarId, eventId, scope) => {
    try {
      await eventDelete(sourceId, calendarId, eventId, scope);
      await get().loadEvents();
      toast.success("削除しました");
    } catch (e) {
      const c = classifyError(e);
      // not_found on delete = someone else already deleted it. The user's intent is
      // satisfied, so refresh and treat as success-ish rather than an error.
      if (c.kind === "not_found") {
        await get().loadEvents();
        toast.info("対象は既に削除されていました");
        return;
      }
      toast.error(`削除に失敗しました: ${c.userMessage}`);
      throw e;
    }
  },
}));

// Re-export for components that want the classifier without importing from `lib/errors`
// directly. Kept here so future centralization (rate-limit cooldown, retry queue)
// can stay co-located with the store actions that consume it.
export { classifyError, isAuthRequired } from "../lib/errors";

export function isCalendarEnabledIn(
  calendarEnabled: Record<string, boolean>,
  sourceId: CalendarSourceId,
  calendarId: string,
): boolean {
  const key = calKey(sourceId, calendarId);
  return calendarEnabled[key] ?? true;
}

export function filterVisible(
  events: UnifiedEvent[],
  sourceEnabled: RecordBy<CalendarSourceId, boolean>,
  calendarEnabled: Record<string, boolean>,
): UnifiedEvent[] {
  return events.filter((e) => {
    if (!sourceEnabled[e.sourceId]) return false;
    if (!isCalendarEnabledIn(calendarEnabled, e.sourceId, e.calendarId)) return false;
    return true;
  });
}

/** Returns the user-visible color for a calendar, applying override → provider color
 *  → source fallback in that order. */
export function effectiveCalendarColor(
  meta: CalendarMeta | undefined,
  overrides: Record<string, CalendarOverride>,
  sourceFallback: string,
): string {
  if (!meta) return sourceFallback;
  const o = overrides[calKey(meta.sourceId, meta.id)];
  return o?.color || meta.color || sourceFallback;
}

/** Returns the user-visible label for a calendar (override > provider name). */
export function effectiveCalendarName(
  meta: CalendarMeta | undefined,
  overrides: Record<string, CalendarOverride>,
  fallback: string,
): string {
  if (!meta) return fallback;
  const o = overrides[calKey(meta.sourceId, meta.id)];
  return o?.label || meta.name || fallback;
}

export function overrideKey(sourceId: CalendarSourceId, calendarId: string): string {
  return calKey(sourceId, calendarId);
}
