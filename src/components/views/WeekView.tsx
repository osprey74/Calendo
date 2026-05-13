import { useMemo } from "react";
import type { CSSProperties } from "react";
import { filterVisible, useCalendarStore } from "../../store/calendarStore";
import { addDays, formatWeekdayShort, isSameDay, startOfWeekJst } from "../../utils/dateUtils";
import { assignLanes, partitionDay } from "../../utils/eventUtils";
import { AllDayBar } from "../events/AllDayBar";
import { EventBlock } from "../events/EventBlock";
import { HourGutter, HourLines } from "./DayView";
import { NowLine } from "./NowLine";
import "./TimeGrid.css";

export function WeekView() {
  const anchor = useCalendarStore((s) => s.anchor);
  const events = useCalendarStore((s) => s.events);
  const sourceEnabled = useCalendarStore((s) => s.sourceEnabled);
  const calendarEnabled = useCalendarStore((s) => s.calendarEnabled);
  const hourHeightPx = useCalendarStore((s) => s.hourHeightPx);
  const viewStartHour = useCalendarStore((s) => s.viewStartHour);
  const viewEndHour = useCalendarStore((s) => s.viewEndHour);

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
  const visibleHours = viewEndHour - viewStartHour;
  const scrollStyle = {
    ["--hour-px" as never]: `${hourHeightPx}px`,
    ["--visible-hours" as never]: visibleHours,
  } as CSSProperties;
  const todayColumnIndex = days.findIndex((d) => isSameDay(d, today));

  return (
    <div className="week-view">
      <div className="time-grid-scroll" style={scrollStyle}>
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
              // All-day bar uses the full-day partition so visibility-window changes
              // don't hide all-day events.
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
          <HourGutter startHour={viewStartHour} endHour={viewEndHour} />
          <div className="week-columns">
            <NowLine
              visibleStartHour={viewStartHour}
              visibleEndHour={viewEndHour}
              todayColumnIndex={todayColumnIndex}
              columnCount={7}
            />
            {days.map((d) => {
              const split = partitionDay(visible, d, viewStartHour, viewEndHour);
              const lanes = assignLanes(split.timed, d);
              return (
                <div
                  key={d.toISOString()}
                  className={`day-column ${isSameDay(d, today) ? "today" : ""}`}
                >
                  <HourLines startHour={viewStartHour} endHour={viewEndHour} />
                  {split.timed.map((e) => {
                    const meta = lanes.get(e.id) ?? { lane: 0, total: 1 };
                    return (
                      <EventBlock
                        key={e.id}
                        event={e}
                        day={d}
                        lane={meta.lane}
                        laneCount={meta.total}
                        visibleStartHour={viewStartHour}
                        visibleEndHour={viewEndHour}
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
