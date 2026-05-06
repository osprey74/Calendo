import { useEffect, useState } from "react";
import { useCalendarStore } from "../../store/calendarStore";
import { CalendarSidebar } from "../sidebar/CalendarSidebar";
import { DayView } from "../views/DayView";
import { WeekView } from "../views/WeekView";
import { TopBar } from "./TopBar";
import { ConnectionPanel } from "../settings/ConnectionPanel";
import "./AppShell.css";

export function AppShell() {
  const view = useCalendarStore((s) => s.view);
  const error = useCalendarStore((s) => s.error);
  const loadEvents = useCalendarStore((s) => s.loadEvents);
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    void loadEvents();
  }, [loadEvents]);

  return (
    <div className="app-shell">
      <TopBar onOpenSettings={() => setSettingsOpen(true)} />
      <div className="app-body">
        <CalendarSidebar />
        <main className="app-main">
          {error && <div className="app-error">{error}</div>}
          {view === "day" ? <DayView /> : <WeekView />}
        </main>
      </div>
      {settingsOpen && <ConnectionPanel onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
