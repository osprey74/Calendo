import { useEffect, useMemo, useState } from "react";
import { useCalendarStore } from "../../store/calendarStore";
import { DEFAULT_SOURCES } from "../../types";
import { CalendarSidebar } from "../sidebar/CalendarSidebar";
import { DayView } from "../views/DayView";
import { WeekView } from "../views/WeekView";
import { TopBar } from "./TopBar";
import { SettingsModal } from "../settings/SettingsModal";
import { ToastHost } from "../toast/ToastHost";
import "./AppShell.css";

export function AppShell() {
  const view = useCalendarStore((s) => s.view);
  const error = useCalendarStore((s) => s.error);
  const hydrate = useCalendarStore((s) => s.hydrate);
  const loadEvents = useCalendarStore((s) => s.loadEvents);
  const calendars = useCalendarStore((s) => s.calendars);
  const events = useCalendarStore((s) => s.events);
  const loading = useCalendarStore((s) => s.loading);
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    // Hydrate persisted filters before the initial fetch so the events query already
    // reflects what the user saved last session (avoids one wasted full-fetch).
    void hydrate().then(() => loadEvents());
  }, [hydrate, loadEvents]);

  // First-launch detection: if every source's calendar list is either unfetched (null)
  // or empty AND we haven't pulled any events, the user almost certainly hasn't connected
  // yet. We surface a single hint pointing them at the settings gear.
  const noConnections = useMemo(() => {
    if (loading) return false;
    if (events.length > 0) return false;
    return DEFAULT_SOURCES.every((s) => {
      const list = calendars[s.id];
      return list === null || list.length === 0;
    });
  }, [loading, events.length, calendars]);

  return (
    <div className="app-shell">
      <TopBar onOpenSettings={() => setSettingsOpen(true)} />
      <div className="app-body">
        <CalendarSidebar />
        <main className="app-main">
          {error && <div className="app-error">{error}</div>}
          {noConnections && (
            <div className="onboarding-hint">
              <div className="onboarding-card">
                <h2>ようこそ！</h2>
                <p>
                  まずカレンダーアカウントを接続しましょう。右上の設定（⚙）から
                  Microsoft 365 / Google / iCloud のいずれかに接続できます。
                </p>
                <button
                  type="button"
                  className="onboarding-cta"
                  onClick={() => setSettingsOpen(true)}
                >
                  設定を開く
                </button>
              </div>
            </div>
          )}
          {view === "day" ? <DayView /> : <WeekView />}
        </main>
      </div>
      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}
      <ToastHost />
    </div>
  );
}
