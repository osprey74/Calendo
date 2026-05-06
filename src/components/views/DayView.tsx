import { useMemo } from "react";
import { filterVisible, useCalendarStore } from "../../store/calendarStore";
import { assignLanes, partitionDay } from "../../utils/eventUtils";
import { AllDayBar } from "../events/AllDayBar";
import { EventBlock } from "../events/EventBlock";
import "./TimeGrid.css";

export function DayView() {
  const anchor = useCalendarStore((s) => s.anchor);
  const events = useCalendarStore((s) => s.events);
  const sourceEnabled = useCalendarStore((s) => s.sourceEnabled);
  const calendarEnabled = useCalendarStore((s) => s.calendarEnabled);

  const visible = useMemo(
    () => filterVisible(events, sourceEnabled, calendarEnabled),
    [events, sourceEnabled, calendarEnabled],
  );

  const { allDay, timed, lanes } = useMemo(() => {
    const split = partitionDay(visible, anchor);
    return {
      allDay: split.allDay,
      timed: split.timed,
      lanes: assignLanes(split.timed, anchor),
    };
  }, [visible, anchor]);

  return (
    <div className="day-view">
      <div className="time-grid-scroll">
        <div className="week-sticky-top">
          <AllDayBar events={allDay} />
        </div>
        <div className="time-grid">
          <HourGutter />
          <div className="day-column">
            <HourLines />
            {timed.map((e) => {
              const meta = lanes.get(e.id) ?? { lane: 0, total: 1 };
              return (
                <EventBlock
                  key={e.id}
                  event={e}
                  day={anchor}
                  lane={meta.lane}
                  laneCount={meta.total}
                />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

export function HourGutter() {
  const hours = Array.from({ length: 24 }, (_, i) => i);
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

export function HourLines() {
  const hours = Array.from({ length: 24 }, (_, i) => i);
  return (
    <div className="hour-lines">
      {hours.map((h) => (
        <div key={h} className="hour-line" />
      ))}
    </div>
  );
}
