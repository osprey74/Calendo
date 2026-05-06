/**
 * Date helpers in the app's canonical timezone (JST).
 *
 * Internally we lean on the JS `Date` object set against +09:00 strings — for the unified
 * event window we never cross timezone boundaries, so manipulating local components is safe.
 */

const JST_OFFSET_MIN = 9 * 60;

export function todayJst(): Date {
  return new Date();
}

/** Returns a Date positioned at 00:00 JST for the given calendar day in JST. */
export function startOfDayJst(d: Date): Date {
  const out = new Date(d);
  out.setHours(0, 0, 0, 0);
  return out;
}

export function endOfDayJst(d: Date): Date {
  const out = new Date(d);
  out.setHours(23, 59, 59, 999);
  return out;
}

export function addDays(d: Date, n: number): Date {
  const out = new Date(d);
  out.setDate(out.getDate() + n);
  return out;
}

/** Sunday-anchored start of the week containing `d` (00:00 JST). */
export function startOfWeekJst(d: Date): Date {
  const out = startOfDayJst(d);
  out.setDate(out.getDate() - out.getDay());
  return out;
}

export function endOfWeekJst(d: Date): Date {
  const out = startOfWeekJst(d);
  out.setDate(out.getDate() + 6);
  out.setHours(23, 59, 59, 999);
  return out;
}

/** YYYY-MM-DD for the given Date in local (JST) calendar terms. */
export function ymd(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/** True when two Dates fall on the same JST calendar day. */
export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** Parses an ISO string (with offset) or YYYY-MM-DD into a Date positioned at the same moment. */
export function parseEventDate(iso: string): Date {
  if (/^\d{4}-\d{2}-\d{2}$/.test(iso)) {
    // All-day: treat as midnight JST.
    const [y, m, d] = iso.split("-").map(Number);
    const utc = Date.UTC(y, m - 1, d, 0 - Math.trunc(JST_OFFSET_MIN / 60), -JST_OFFSET_MIN % 60, 0);
    return new Date(utc);
  }
  return new Date(iso);
}

/** Returns minutes since 00:00 JST on `dayAnchor`'s date, clamped to [0, 1440]. */
export function minutesSinceMidnight(date: Date, dayAnchor: Date): number {
  const start = startOfDayJst(dayAnchor);
  const diffMs = date.getTime() - start.getTime();
  const minutes = Math.round(diffMs / 60000);
  return Math.max(0, Math.min(1440, minutes));
}

const WEEKDAYS_JA = ["日", "月", "火", "水", "木", "金", "土"];

export function formatDateHeading(d: Date): string {
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日（${WEEKDAYS_JA[d.getDay()]}）`;
}

export function formatWeekHeading(weekStart: Date): string {
  const end = addDays(weekStart, 6);
  if (weekStart.getMonth() === end.getMonth()) {
    return `${weekStart.getFullYear()}年${weekStart.getMonth() + 1}月 ${weekStart.getDate()}日 – ${end.getDate()}日`;
  }
  return `${weekStart.getFullYear()}年${weekStart.getMonth() + 1}月${weekStart.getDate()}日 – ${end.getMonth() + 1}月${end.getDate()}日`;
}

export function formatTimeShort(d: Date): string {
  const h = String(d.getHours()).padStart(2, "0");
  const m = String(d.getMinutes()).padStart(2, "0");
  return `${h}:${m}`;
}

export function formatWeekdayShort(d: Date): string {
  return WEEKDAYS_JA[d.getDay()];
}
