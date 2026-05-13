import { useState } from "react";
import type { UnifiedEvent } from "../../types";
import { DEFAULT_SOURCES } from "../../types";
import {
  effectiveCalendarColor,
  effectiveCalendarName,
  useCalendarStore,
} from "../../store/calendarStore";
import { dayBlockLayout } from "../../utils/eventUtils";
import { eventEnd, eventStart } from "../../utils/eventUtils";
import { formatTimeShort } from "../../utils/dateUtils";
import { EventDetailsModal } from "./EventDetailsModal";
import "./EventBlock.css";

type Props = {
  event: UnifiedEvent;
  day: Date;
  lane: number;
  laneCount: number;
  visibleStartHour?: number;
  visibleEndHour?: number;
};

export function EventBlock({
  event,
  day,
  lane,
  laneCount,
  visibleStartHour = 0,
  visibleEndHour = 24,
}: Props) {
  const calendars = useCalendarStore((s) => s.calendars);
  const calendarOverrides = useCalendarStore((s) => s.calendarOverrides);
  const [open, setOpen] = useState(false);
  const layout = dayBlockLayout(event, day, visibleStartHour, visibleEndHour);
  const widthPct = 100 / Math.max(1, laneCount);
  const leftPct = widthPct * lane;

  const source = DEFAULT_SOURCES.find((s) => s.id === event.sourceId);
  const sourceColor = source?.color ?? "#888";
  const calendar = calendars[event.sourceId]?.find((c) => c.id === event.calendarId);
  const calendarColor = effectiveCalendarColor(calendar, calendarOverrides, sourceColor);
  const calendarName = effectiveCalendarName(calendar, calendarOverrides, "(取得済み一覧外)");

  const start = eventStart(event);
  const end = eventEnd(event);
  const tooltip = [
    event.title || "(無題)",
    `${formatTimeShort(start)} – ${formatTimeShort(end)}`,
    `${source?.label ?? event.sourceId} / ${calendarName}`,
    event.location ? `📍 ${event.location}` : null,
  ]
    .filter(Boolean)
    .join("\n");

  return (
    <>
      <button
        type="button"
        className="event-block"
        style={{
          top: `${layout.topPct}%`,
          height: `${layout.heightPct}%`,
          left: `${leftPct}%`,
          width: `calc(${widthPct}% - 2px)`,
          background: hexToBg(calendarColor),
          borderLeftColor: calendarColor,
        }}
        title={tooltip}
        onClick={() => setOpen(true)}
      >
        <div className="event-time">
          {formatTimeShort(start)} – {formatTimeShort(end)}
        </div>
        <div className="event-title">{event.title || "(無題)"}</div>
        <div className="event-cal-tag">{calendarName}</div>
      </button>
      {open && <EventDetailsModal event={event} onClose={() => setOpen(false)} />}
    </>
  );
}

/** Lighter background derived from event color for legibility on light/dark backgrounds. */
function hexToBg(hex: string): string {
  const m = hex.match(/^#?([0-9a-f]{6})$/i);
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  const r = (n >> 16) & 0xff;
  const g = (n >> 8) & 0xff;
  const b = n & 0xff;
  return `rgba(${r}, ${g}, ${b}, 0.18)`;
}
