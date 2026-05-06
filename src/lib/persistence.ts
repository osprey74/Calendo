/**
 * Settings persistence wrapper around `@tauri-apps/plugin-store`.
 *
 * The store backs a JSON file under the user's app config (`%APPDATA%\com.osprey74.calendo`
 * on Windows / `~/Library/Application Support/com.osprey74.calendo` on macOS). Reads happen
 * once at startup via `loadSettings()`; writes are fire-and-forget per-action.
 */

import { Store, type Store as StoreInstance } from "@tauri-apps/plugin-store";
import type { CalendarSourceId, CalendarView } from "../types";

const STORE_PATH = "settings.json";

let storePromise: Promise<StoreInstance> | null = null;
function getStore(): Promise<StoreInstance> {
  if (!storePromise) {
    storePromise = Store.load(STORE_PATH);
  }
  return storePromise;
}

export type PersistedSettings = {
  view?: CalendarView;
  sourceEnabled?: Partial<Record<CalendarSourceId, boolean>>;
  calendarEnabled?: Record<string, boolean>;
};

export async function loadSettings(): Promise<PersistedSettings> {
  try {
    const s = await getStore();
    return {
      view: (await s.get<CalendarView>("view")) ?? undefined,
      sourceEnabled:
        (await s.get<Partial<Record<CalendarSourceId, boolean>>>("sourceEnabled")) ?? undefined,
      calendarEnabled: (await s.get<Record<string, boolean>>("calendarEnabled")) ?? undefined,
    };
  } catch {
    // First launch or corrupted store — fall through to defaults.
    return {};
  }
}

export async function saveView(view: CalendarView): Promise<void> {
  const s = await getStore();
  await s.set("view", view);
  await s.save();
}

export async function saveSourceEnabled(
  map: Partial<Record<CalendarSourceId, boolean>>,
): Promise<void> {
  const s = await getStore();
  await s.set("sourceEnabled", map);
  await s.save();
}

export async function saveCalendarEnabled(map: Record<string, boolean>): Promise<void> {
  const s = await getStore();
  await s.set("calendarEnabled", map);
  await s.save();
}
