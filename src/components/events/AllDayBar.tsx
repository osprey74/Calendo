import { useState } from "react";
import type { UnifiedEvent } from "../../types";
import { DEFAULT_SOURCES } from "../../types";
import { useCalendarStore } from "../../store/calendarStore";
import { EventDetailsModal } from "./EventDetailsModal";
import "./EventBlock.css";

export function AllDayBar({ events }: { events: UnifiedEvent[] }) {
  const calendars = useCalendarStore((s) => s.calendars);
  const [openId, setOpenId] = useState<string | null>(null);

  if (events.length === 0) {
    return <div className="allday-bar empty" />;
  }

  return (
    <>
      <div className="allday-bar">
        {events.map((e) => {
          const source = DEFAULT_SOURCES.find((s) => s.id === e.sourceId);
          const sourceColor = source?.color ?? "#888";
          const calendar = calendars[e.sourceId]?.find((c) => c.id === e.calendarId);
          const calColor = calendar?.color ?? sourceColor;
          const calendarName = calendar?.name ?? "(取得済み一覧外)";
          const tooltip = `${e.title || "(無題)"}\n${source?.label ?? e.sourceId} / ${calendarName}`;
          return (
            <button
              type="button"
              key={e.id}
              className="allday-pill"
              style={{ borderLeftColor: calColor }}
              title={tooltip}
              onClick={() => setOpenId(e.id)}
            >
              {e.title || "(無題)"}
            </button>
          );
        })}
      </div>
      {openId && (
        <EventDetailsModal
          event={events.find((e) => e.id === openId)!}
          onClose={() => setOpenId(null)}
        />
      )}
    </>
  );
}
