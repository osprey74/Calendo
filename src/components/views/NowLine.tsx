import { useEffect, useState } from "react";
import type { CSSProperties } from "react";

type Props = {
  visibleStartHour: number;
  visibleEndHour: number;
  /** Today's column index among the rendered columns, or `-1` if today is outside the
   *  displayed range (e.g., week view showing next week). In DayView this is `0` when
   *  the anchor day is today, else `-1`. In WeekView it's `0-6` or `-1`. */
  todayColumnIndex: number;
  /** Total column count in the parent grid (1 for DayView, 7 for WeekView). Used to
   *  position the dot at the left edge of today's column. */
  columnCount: number;
};

/** Horizontal "now" indicator drawn across the time grid. Renders a thin red line that
 *  spans the full parent width plus a small dot anchored to the left edge of today's
 *  column. Hidden when today is outside the displayed range or when the current time
 *  is outside the visible-hour window.
 *
 *  Parent must be `position: relative`. The line uses `left: 0; right: 0` so it spans
 *  whatever container it's mounted in — `.day-column` in DayView, `.week-columns` in
 *  WeekView. */
export function NowLine({
  visibleStartHour,
  visibleEndHour,
  todayColumnIndex,
  columnCount,
}: Props) {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    // 30 s is well below the pixel-resolution threshold (a minute = ~1 px at 60 px/hr)
    // but cheap enough to keep the indicator from drifting visibly. We don't bother
    // aligning to the next minute boundary; visual jitter at the sub-minute scale is
    // imperceptible.
    const id = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(id);
  }, []);

  if (todayColumnIndex < 0) return null;

  const nowMin = now.getHours() * 60 + now.getMinutes() + now.getSeconds() / 60;
  const visStart = visibleStartHour * 60;
  const visEnd = visibleEndHour * 60;
  if (nowMin < visStart || nowMin >= visEnd) return null;

  const topPct = ((nowMin - visStart) / (visEnd - visStart)) * 100;
  const dotLeftPct = (todayColumnIndex / Math.max(1, columnCount)) * 100;

  const lineStyle = { top: `${topPct}%` } as CSSProperties;
  const dotStyle = { top: `${topPct}%`, left: `${dotLeftPct}%` } as CSSProperties;

  return (
    <>
      <div className="now-line" style={lineStyle} aria-hidden />
      <div className="now-dot" style={dotStyle} aria-hidden />
    </>
  );
}
