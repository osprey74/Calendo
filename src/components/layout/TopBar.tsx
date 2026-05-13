import { useState } from "react";
import {
  DEFAULT_HOUR_HEIGHT_PX,
  HOUR_HEIGHT_LEVELS,
  useCalendarStore,
} from "../../store/calendarStore";
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
  const hourHeightPx = useCalendarStore((s) => s.hourHeightPx);
  const stepHourHeight = useCalendarStore((s) => s.stepHourHeight);
  const setHourHeightPx = useCalendarStore((s) => s.setHourHeightPx);

  const minHourPx = HOUR_HEIGHT_LEVELS[0];
  const maxHourPx = HOUR_HEIGHT_LEVELS[HOUR_HEIGHT_LEVELS.length - 1];
  const isDefaultZoom = hourHeightPx === DEFAULT_HOUR_HEIGHT_PX;

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
          <div className="zoom-control" role="group" aria-label="時間軸の高さ">
            <button
              type="button"
              className="zoom-btn"
              onClick={() => stepHourHeight(-1)}
              disabled={hourHeightPx <= minHourPx}
              title="時間軸を縮める"
              aria-label="時間軸を縮める"
            >
              −
            </button>
            <button
              type="button"
              className="zoom-value"
              onClick={() => setHourHeightPx(DEFAULT_HOUR_HEIGHT_PX)}
              disabled={isDefaultZoom}
              title={
                isDefaultZoom
                  ? `1時間あたり ${hourHeightPx}px（既定）`
                  : `1時間あたり ${hourHeightPx}px ・ クリックで既定（${DEFAULT_HOUR_HEIGHT_PX}px）に戻す`
              }
              aria-label={`1時間あたり ${hourHeightPx}px。クリックで既定値に戻す`}
            >
              {hourHeightPx}px
            </button>
            <button
              type="button"
              className="zoom-btn"
              onClick={() => stepHourHeight(1)}
              disabled={hourHeightPx >= maxHourPx}
              title="時間軸を伸ばす"
              aria-label="時間軸を伸ばす"
            >
              ＋
            </button>
          </div>
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
