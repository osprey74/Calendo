import { useState } from "react";
import { useCalendarStore } from "../../store/calendarStore";
import {
  formatDateHeading,
  formatWeekHeading,
  startOfWeekJst,
} from "../../utils/dateUtils";
import { EventModal } from "../events/EventModal";
import "./TopBar.css";

export function TopBar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const view = useCalendarStore((s) => s.view);
  const anchor = useCalendarStore((s) => s.anchor);
  const setView = useCalendarStore((s) => s.setView);
  const shiftAnchor = useCalendarStore((s) => s.shiftAnchor);
  const goToToday = useCalendarStore((s) => s.goToToday);
  const loading = useCalendarStore((s) => s.loading);

  const [createOpen, setCreateOpen] = useState(false);

  const heading =
    view === "day"
      ? formatDateHeading(anchor)
      : formatWeekHeading(startOfWeekJst(anchor));

  return (
    <>
      <header className="topbar">
        <div className="topbar-left">
          <h1 className="topbar-title">Calendo</h1>
          <div className="view-toggle" role="tablist" aria-label="ビュー切替">
            <button
              type="button"
              role="tab"
              aria-selected={view === "day"}
              className={view === "day" ? "active" : ""}
              onClick={() => setView("day")}
            >
              日
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={view === "week"}
              className={view === "week" ? "active" : ""}
              onClick={() => setView("week")}
            >
              週
            </button>
          </div>
        </div>

        <div className="topbar-center">
          <button
            type="button"
            className="nav-arrow"
            aria-label="前へ"
            onClick={() => shiftAnchor(-1)}
          >
            ‹
          </button>
          <span className="topbar-heading">{heading}</span>
          <button
            type="button"
            className="nav-arrow"
            aria-label="次へ"
            onClick={() => shiftAnchor(1)}
          >
            ›
          </button>
          <button type="button" className="today-btn" onClick={goToToday}>
            今日
          </button>
        </div>

        <div className="topbar-right">
          {loading && <span className="loading-tag">読み込み中…</span>}
          <button
            type="button"
            className="create-btn"
            onClick={() => setCreateOpen(true)}
            title="新しい予定を作成"
          >
            ＋ 新規
          </button>
          <button type="button" className="settings-btn" onClick={onOpenSettings} aria-label="設定">
            ⚙
          </button>
        </div>
      </header>

      {createOpen && (
        // Default the new-event form to today regardless of which week/day the user is
        // currently viewing — picking a day in the visible window has been a common
        // source of accidental wrong-day registrations.
        <EventModal
          mode={{ kind: "create", defaultDate: new Date() }}
          onClose={() => setCreateOpen(false)}
        />
      )}
    </>
  );
}
