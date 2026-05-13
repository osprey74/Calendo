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

/** Raw start/end minutes for an event clipped to a single calendar day. Used by both
 *  `dayBlockLayout` (for percentage rendering) and `assignLanes` (for collision
 *  detection); the latter doesn't care about the visible window. */
function eventDayMinutes(e: UnifiedEvent, day: Date): { startMin: number; endMin: number } {
  const start = eventStart(e);
  const end = eventEnd(e);
  const startMin = minutesSinceMidnight(start, day);
  const endMin = Math.max(startMin + 15, minutesSinceMidnight(end, day));
  return { startMin, endMin };
}

/** Returns the layout percentages for an event relative to the visible time window
 *  `[visibleStartHour, visibleEndHour)`. Events that fall outside the window are
 *  clipped to its edges so partially-out-of-range events still render with the visible
 *  portion. The original startMin/endMin are returned unchanged for lane assignment
 *  and tooltips. */
export function dayBlockLayout(
  e: UnifiedEvent,
  day: Date,
  visibleStartHour: number = 0,
  visibleEndHour: number = 24,
): DayBlockLayout {
  const { startMin, endMin } = eventDayMinutes(e, day);
  const visStart = visibleStartHour * 60;
  const visEnd = visibleEndHour * 60;
  const visSpan = Math.max(1, visEnd - visStart);
  const clippedStart = Math.max(startMin, visStart);
  const clippedEnd = Math.min(endMin, visEnd);
  return {
    topPct: ((clippedStart - visStart) / visSpan) * 100,
    heightPct: (Math.max(0, clippedEnd - clippedStart) / visSpan) * 100,
    startMin,
    endMin,
  };
}

/** True when any portion of the event overlaps the visible window for `day`. */
export function eventOverlapsWindow(
  e: UnifiedEvent,
  day: Date,
  visibleStartHour: number,
  visibleEndHour: number,
): boolean {
  const { startMin, endMin } = eventDayMinutes(e, day);
  return endMin > visibleStartHour * 60 && startMin < visibleEndHour * 60;
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

/** Splits a day's events into all-day and timed buckets. When a visible window is given,
 *  timed events that don't overlap the window are dropped — this keeps the lane-assigned
 *  layout accurate (otherwise hidden events would still steal lane width from visible
 *  ones that overlap them). All-day events are unaffected since they live in their own
 *  bar above the time grid. */
export function partitionDay(
  events: UnifiedEvent[],
  day: Date,
  visibleStartHour: number = 0,
  visibleEndHour: number = 24,
) {
  const onDay = events.filter((e) => eventOccursOn(e, day));
  const allDay = onDay.filter((e) => e.isAllDay);
  const timedAll = onDay.filter((e) => !e.isAllDay);
  const timed =
    visibleStartHour === 0 && visibleEndHour === 24
      ? timedAll
      : timedAll.filter((e) => eventOverlapsWindow(e, day, visibleStartHour, visibleEndHour));
  return {
    allDay: sortForDay(allDay),
    timed: sortForDay(timed),
  };
}

/** Greedy lane assignment for timed events on a single day. Operates on raw day minutes
 *  so callers don't need to thread the visible window through — the window only affects
 *  which events are passed in via `partitionDay`. */
export function assignLanes(events: UnifiedEvent[], day: Date): Map<string, { lane: number; total: number }> {
  const layouts = events
    .map((e) => ({ e, mins: eventDayMinutes(e, day) }))
    .sort((a, b) => a.mins.startMin - b.mins.startMin);

  const laneEnds: number[] = [];
  const out = new Map<string, { lane: number; total: number }>();

  for (const { e, mins } of layouts) {
    let lane = laneEnds.findIndex((end) => end <= mins.startMin);
    if (lane === -1) {
      lane = laneEnds.length;
      laneEnds.push(mins.endMin);
    } else {
      laneEnds[lane] = mins.endMin;
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
