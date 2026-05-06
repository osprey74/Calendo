import { useEffect, useState } from "react";
import {
  isCalendarEnabledIn,
  useCalendarStore,
} from "../../store/calendarStore";
import { DEFAULT_SOURCES } from "../../types";
import type { CalendarSourceId } from "../../types";
import "./CalendarSidebar.css";

export function CalendarSidebar() {
  const calendars = useCalendarStore((s) => s.calendars);
  const sourceEnabled = useCalendarStore((s) => s.sourceEnabled);
  const calendarEnabled = useCalendarStore((s) => s.calendarEnabled);
  const toggleSource = useCalendarStore((s) => s.toggleSource);
  const toggleCalendar = useCalendarStore((s) => s.toggleCalendar);
  const setAllCalendarsEnabled = useCalendarStore((s) => s.setAllCalendarsEnabled);
  const loadCalendars = useCalendarStore((s) => s.loadCalendars);
  const loadEvents = useCalendarStore((s) => s.loadEvents);

  /** Per-source toggle for showing the collapsed "hidden calendars" group. */
  const [hiddenExpanded, setHiddenExpanded] = useState<Partial<Record<CalendarSourceId, boolean>>>({});

  useEffect(() => {
    DEFAULT_SOURCES.forEach((s) => {
      if (calendars[s.id] === null) {
        loadCalendars(s.id).catch(() => {});
      }
    });
  }, [calendars, loadCalendars]);

  return (
    <aside className="sidebar">
      {DEFAULT_SOURCES.map((s) => {
        const list = calendars[s.id];
        const enabled = sourceEnabled[s.id];

        const visibleCalendars = list?.filter((c) => isCalendarEnabledIn(calendarEnabled, s.id, c.id)) ?? [];
        const hiddenCalendars = list?.filter((c) => !isCalendarEnabledIn(calendarEnabled, s.id, c.id)) ?? [];
        const allOn = !!list && list.length > 0 && hiddenCalendars.length === 0;
        const allOff = !!list && list.length > 0 && visibleCalendars.length === 0;
        const isHiddenOpen = hiddenExpanded[s.id] ?? false;

        return (
          <section key={s.id} className="source-block">
            <header className="source-head" style={{ borderLeftColor: s.color }}>
              <label className="source-toggle">
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={() => {
                    toggleSource(s.id);
                    queueMicrotask(() => loadEvents().catch(() => {}));
                  }}
                />
                <span
                  className="source-swatch"
                  style={{ background: s.color }}
                  aria-hidden
                />
                <span className="source-label">{s.label}</span>
                {list && list.length > 0 && (
                  <span className="source-count">{list.length}</span>
                )}
              </label>
              {list && list.length > 1 && (
                <div className="bulk-actions">
                  <button
                    type="button"
                    className="bulk-btn"
                    disabled={allOn}
                    onClick={() => {
                      setAllCalendarsEnabled(s.id, true);
                      queueMicrotask(() => loadEvents().catch(() => {}));
                    }}
                    title="このソースの全カレンダーを表示"
                  >
                    全 ON
                  </button>
                  <button
                    type="button"
                    className="bulk-btn"
                    disabled={allOff}
                    onClick={() => {
                      setAllCalendarsEnabled(s.id, false);
                      queueMicrotask(() => loadEvents().catch(() => {}));
                    }}
                    title="このソースの全カレンダーを非表示"
                  >
                    全 OFF
                  </button>
                </div>
              )}
            </header>

            {list === null ? (
              <div className="cal-empty muted">未取得</div>
            ) : list.length === 0 ? (
              <div className="cal-empty muted">カレンダー無し</div>
            ) : (
              <>
                <ul className="cal-list">
                  {visibleCalendars.map((c) => (
                    <CalendarRow
                      key={c.id}
                      sourceColor={s.color}
                      name={c.name}
                      color={c.color}
                      isWritable={c.isWritable}
                      isPrimary={c.isPrimary}
                      enabled
                      onToggle={() => {
                        toggleCalendar(s.id, c.id);
                        queueMicrotask(() => loadEvents().catch(() => {}));
                      }}
                    />
                  ))}
                  {visibleCalendars.length === 0 && (
                    <li className="cal-empty muted">表示中のカレンダーなし</li>
                  )}
                </ul>

                {hiddenCalendars.length > 0 && (
                  <div className="hidden-group">
                    <button
                      type="button"
                      className="hidden-toggle"
                      aria-expanded={isHiddenOpen ? "true" : "false"}
                      onClick={() =>
                        setHiddenExpanded((prev) => ({ ...prev, [s.id]: !isHiddenOpen }))
                      }
                    >
                      <span className="hidden-caret">{isHiddenOpen ? "▾" : "▸"}</span>
                      <span>非表示中</span>
                      <span className="hidden-count">{hiddenCalendars.length}</span>
                    </button>
                    {isHiddenOpen && (
                      <ul className="cal-list hidden-list">
                        {hiddenCalendars.map((c) => (
                          <CalendarRow
                            key={c.id}
                            sourceColor={s.color}
                            name={c.name}
                            color={c.color}
                            isWritable={c.isWritable}
                            isPrimary={c.isPrimary}
                            enabled={false}
                            onToggle={() => {
                              toggleCalendar(s.id, c.id);
                              queueMicrotask(() => loadEvents().catch(() => {}));
                            }}
                          />
                        ))}
                      </ul>
                    )}
                  </div>
                )}
              </>
            )}
          </section>
        );
      })}
    </aside>
  );
}

type RowProps = {
  sourceColor: string;
  name: string;
  color?: string;
  isWritable: boolean;
  isPrimary: boolean;
  enabled: boolean;
  onToggle: () => void;
};

function CalendarRow({ sourceColor, name, color, isWritable, isPrimary, enabled, onToggle }: RowProps) {
  return (
    <li className={`cal-row ${enabled ? "" : "disabled"}`}>
      <label className="cal-toggle">
        <input type="checkbox" checked={enabled} onChange={onToggle} />
        <span className="cal-swatch" style={{ background: color || sourceColor }} aria-hidden />
        <span className="cal-name">
          {name}
          {isPrimary && <span className="primary-tag"> primary</span>}
        </span>
      </label>
      {!isWritable && <span className="ro-badge" title="読み取り専用">RO</span>}
    </li>
  );
}
