import { create } from "zustand";
import type {
  CalendarMeta,
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
  saveSourceEnabled,
  saveView,
} from "../lib/persistence";
import { toast } from "./toastStore";

type RecordBy<T extends string, V> = Record<T, V>;

type FetchKey = string;

type State = {
  view: CalendarView;
  /** Anchor date (current day for DayView, any day in the week for WeekView). */
  anchor: Date;

  /** Calendars known per source (null = not yet fetched). */
  calendars: RecordBy<CalendarSourceId, CalendarMeta[] | null>;
  /** Source-level enable toggle (UI filter). */
  sourceEnabled: RecordBy<CalendarSourceId, boolean>;
  /** Sub-calendar enable toggle, keyed by `${source}|${calendarId}`. */
  calendarEnabled: Record<string, boolean>;

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

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  view: "week",
  anchor: new Date(),

  calendars: {
    ms365_work1: null,
    google_gws: null,
    icloud: null,
  },
  sourceEnabled: defaultSourceEnabled(),
  calendarEnabled: {},

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

  loadCalendars: async (sourceId) => {
    try {
      const list = await calendarsFetch(sourceId);
      set((state) => ({
        calendars: { ...state.calendars, [sourceId]: list },
      }));
    } catch (e) {
      // Per-source errors are expected pre-auth; surface but don't clobber the global state.
      set({ error: `${sourceId}: ${String(e)}` });
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
      const events = await eventsFetch(enabledSources, calendarFilter, dateFrom, dateTo);
      set((state) => ({
        events,
        loadedRange: { from: dateFrom, to: dateTo },
        loading: false,
        revision: state.revision + 1,
      }));
    } catch (e) {
      set({ loading: false, error: String(e) });
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
    set((state) => ({
      view: persisted.view ?? state.view,
      sourceEnabled: persisted.sourceEnabled
        ? { ...state.sourceEnabled, ...persisted.sourceEnabled }
        : state.sourceEnabled,
      calendarEnabled: persisted.calendarEnabled ?? state.calendarEnabled,
    }));
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
      toast.error(`作成に失敗しました: ${e}`);
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
      toast.error(`更新に失敗しました: ${e}`);
      throw e;
    }
  },

  deleteEvent: async (sourceId, calendarId, eventId, scope) => {
    try {
      await eventDelete(sourceId, calendarId, eventId, scope);
      await get().loadEvents();
      toast.success("削除しました");
    } catch (e) {
      toast.error(`削除に失敗しました: ${e}`);
      throw e;
    }
  },
}));

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
