import { useMemo } from "react";
import { filterVisible, useCalendarStore } from "../../store/calendarStore";
import { addDays, formatWeekdayShort, isSameDay, startOfWeekJst } from "../../utils/dateUtils";
import { assignLanes, partitionDay } from "../../utils/eventUtils";
import { AllDayBar } from "../events/AllDayBar";
import { EventBlock } from "../events/EventBlock";
import { HourGutter, HourLines } from "./DayView";
import "./TimeGrid.css";

export function WeekView() {
  const anchor = useCalendarStore((s) => s.anchor);
  const events = useCalendarStore((s) => s.events);
  const sourceEnabled = useCalendarStore((s) => s.sourceEnabled);
  const calendarEnabled = useCalendarStore((s) => s.calendarEnabled);

  const visible = useMemo(
    () => filterVisible(events, sourceEnabled, calendarEnabled),
    [events, sourceEnabled, calendarEnabled],
  );

  const weekStart = useMemo(() => startOfWeekJst(anchor), [anchor]);
  const days = useMemo(
    () => Array.from({ length: 7 }, (_, i) => addDays(weekStart, i)),
    [weekStart],
  );

  const today = new Date();

  return (
    <div className="week-view">
      <div className="time-grid-scroll">
        <div className="week-sticky-top">
          <div className="week-header">
            <div className="week-header-gutter" />
            {days.map((d) => (
              <div
                key={d.toISOString()}
                className={`week-day-head ${isSameDay(d, today) ? "today" : ""}`}
              >
                <div className="weekday">{formatWeekdayShort(d)}</div>
                <div className="daynum">{d.getDate()}</div>
              </div>
            ))}
          </div>

          <div className="week-allday-row">
            <div className="week-allday-gutter">終日</div>
            {days.map((d) => {
              const split = partitionDay(visible, d);
              return (
                <div key={d.toISOString()} className="week-allday-cell">
                  <AllDayBar events={split.allDay} />
                </div>
              );
            })}
          </div>
        </div>

        <div className="time-grid">
          <HourGutter />
          <div className="week-columns">
            {days.map((d) => {
              const split = partitionDay(visible, d);
              const lanes = assignLanes(split.timed, d);
              return (
                <div
                  key={d.toISOString()}
                  className={`day-column ${isSameDay(d, today) ? "today" : ""}`}
                >
                  <HourLines />
                  {split.timed.map((e) => {
                    const meta = lanes.get(e.id) ?? { lane: 0, total: 1 };
                    return (
                      <EventBlock
                        key={e.id}
                        event={e}
                        day={d}
                        lane={meta.lane}
                        laneCount={meta.total}
                      />
                    );
                  })}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
