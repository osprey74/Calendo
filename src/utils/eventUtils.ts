import type { UnifiedEvent } from "../types";
import {
  addDays,
  endOfDayJst,
  isSameDay,
  minutesSinceMidnight,
  parseEventDate,
  startOfDayJst,
} from "./dateUtils";

export function eventStart(e: UnifiedEvent): Date {
  return parseEventDate(e.start);
}

export function eventEnd(e: UnifiedEvent): Date {
  return parseEventDate(e.end);
}

/** For all-day events, the end date is exclusive across MS Graph / GCal / iCal (RFC 5545).
 *  Returns the last day the event actually occupies (inclusive). */
export function allDayEndInclusive(e: UnifiedEvent): Date {
  return addDays(eventEnd(e), -1);
}

/** Returns true when the event overlaps the given JST calendar day. */
export function eventOccursOn(e: UnifiedEvent, day: Date): boolean {
  const start = eventStart(e);
  const end = eventEnd(e);
  const dayStart = startOfDayJst(day);
  const dayEnd = endOfDayJst(day);
  if (e.isAllDay) {
    // RFC 5545 / Graph / GCal all use an exclusive DTEND for all-day events: a single-day
    // event on 5/5 has end = 5/6 00:00. Compare with strict `>` so the trailing midnight
    // doesn't bleed onto the next calendar day.
    return start <= dayEnd && end > dayStart;
  }
  return start <= dayEnd && end >= dayStart;
}

/** Vertical layout for a timed event in the day grid. */
export type DayBlockLayout = {
  topPct: number;
  heightPct: number;
  startMin: number;
  endMin: number;
};

export function dayBlockLayout(e: UnifiedEvent, day: Date): DayBlockLayout {
  const start = eventStart(e);
  const end = eventEnd(e);
  const startMin = minutesSinceMidnight(start, day);
  const endMin = Math.max(startMin + 15, minutesSinceMidnight(end, day));
  return {
    topPct: (startMin / 1440) * 100,
    heightPct: ((endMin - startMin) / 1440) * 100,
    startMin,
    endMin,
  };
}

/** Sorts events for stable in-day rendering: timed first by start, then all-day by title. */
export function sortForDay(events: UnifiedEvent[]): UnifiedEvent[] {
  return [...events].sort((a, b) => {
    if (a.isAllDay !== b.isAllDay) return a.isAllDay ? -1 : 1;
    const cmp = a.start.localeCompare(b.start);
    if (cmp !== 0) return cmp;
    return a.title.localeCompare(b.title);
  });
}

export function partitionDay(events: UnifiedEvent[], day: Date) {
  const onDay = events.filter((e) => eventOccursOn(e, day));
  const allDay = onDay.filter((e) => e.isAllDay);
  const timed = onDay.filter((e) => !e.isAllDay);
  return {
    allDay: sortForDay(allDay),
    timed: sortForDay(timed),
  };
}

/** Greedy lane assignment for timed events on a single day. */
export function assignLanes(events: UnifiedEvent[], day: Date): Map<string, { lane: number; total: number }> {
  const layouts = events
    .map((e) => ({ e, layout: dayBlockLayout(e, day) }))
    .sort((a, b) => a.layout.startMin - b.layout.startMin);

  const laneEnds: number[] = [];
  const out = new Map<string, { lane: number; total: number }>();

  for (const { e, layout } of layouts) {
    let lane = laneEnds.findIndex((end) => end <= layout.startMin);
    if (lane === -1) {
      lane = laneEnds.length;
      laneEnds.push(layout.endMin);
    } else {
      laneEnds[lane] = layout.endMin;
    }
    out.set(e.id, { lane, total: 0 });
  }

  const total = laneEnds.length;
  for (const v of out.values()) {
    v.total = total;
  }
  return out;
}

export function isSameDayAllDay(e: UnifiedEvent, day: Date): boolean {
  return e.isAllDay && isSameDay(parseEventDate(e.start), day);
}
