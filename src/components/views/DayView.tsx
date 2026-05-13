import { useMemo } from "react";
import type { CSSProperties } from "react";
import { filterVisible, useCalendarStore } from "../../store/calendarStore";
import { isSameDay } from "../../utils/dateUtils";
import { assignLanes, partitionDay } from "../../utils/eventUtils";
import { AllDayBar } from "../events/AllDayBar";
import { EventBlock } from "../events/EventBlock";
import { NowLine } from "./NowLine";
import "./TimeGrid.css";

export function DayView() {
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

  const { allDay, timed, lanes } = useMemo(() => {
    const split = partitionDay(visible, anchor, viewStartHour, viewEndHour);
    return {
      allDay: split.allDay,
      timed: split.timed,
      lanes: assignLanes(split.timed, anchor),
    };
  }, [visible, anchor, viewStartHour, viewEndHour]);

  const visibleHours = viewEndHour - viewStartHour;
  const scrollStyle = {
    ["--hour-px" as never]: `${hourHeightPx}px`,
    ["--visible-hours" as never]: visibleHours,
  } as CSSProperties;
  const anchorIsToday = isSameDay(anchor, new Date());

  return (
    <div className="day-view">
      <div className="time-grid-scroll" style={scrollStyle}>
        <div className="week-sticky-top">
          <AllDayBar events={allDay} />
        </div>
        <div className="time-grid">
          <HourGutter startHour={viewStartHour} endHour={viewEndHour} />
          <div className="day-column">
            <HourLines startHour={viewStartHour} endHour={viewEndHour} />
            <NowLine
              visibleStartHour={viewStartHour}
              visibleEndHour={viewEndHour}
              todayColumnIndex={anchorIsToday ? 0 : -1}
              columnCount={1}
            />
            {timed.map((e) => {
              const meta = lanes.get(e.id) ?? { lane: 0, total: 1 };
              return (
                <EventBlock
                  key={e.id}
                  event={e}
                  day={anchor}
                  lane={meta.lane}
                  laneCount={meta.total}
                  visibleStartHour={viewStartHour}
                  visibleEndHour={viewEndHour}
                />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

export function HourGutter({
  startHour = 0,
  endHour = 24,
}: {
  startHour?: number;
  endHour?: number;
}) {
  const hours = Array.from({ length: endHour - startHour }, (_, i) => startHour + i);
  return (
    <div className="hour-gutter">
      {hours.map((h) => (
        <div key={h} className="hour-label">
          {String(h).padStart(2, "0")}:00
        </div>
      ))}
    </div>
  );
}

export function HourLines({
  startHour = 0,
  endHour = 24,
}: {
  startHour?: number;
  endHour?: number;
}) {
  const hours = Array.from({ length: endHour - startHour }, (_, i) => startHour + i);
  return (
    <div className="hour-lines">
      {hours.map((h) => (
        <div key={h} className="hour-line" />
      ))}
    </div>
  );
}
