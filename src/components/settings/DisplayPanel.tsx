import { useMemo, useState } from "react";
import { DEFAULT_SOURCES } from "../../types";
import type { CalendarMeta, CalendarOverride } from "../../types";
import {
  effectiveCalendarColor,
  effectiveCalendarName,
  overrideKey,
  useCalendarStore,
} from "../../store/calendarStore";
import "./DisplayPanel.css";

/** Hour options for the time-range dropdowns. Start is 0-23, end is 1-24 (24 = midnight
 *  next day). The labels use a leading zero so widths stay stable. */
const START_HOUR_OPTIONS = Array.from({ length: 24 }, (_, i) => i);
const END_HOUR_OPTIONS = Array.from({ length: 24 }, (_, i) => i + 1);

function formatHour(h: number): string {
  return `${String(h).padStart(2, "0")}:00`;
}

/** Inner content of the "表示設定" tab. Lets the user override per-calendar color and
 *  display label without round-tripping to the provider. Cleared overrides fall back
 *  to the provider-supplied values. */
export function DisplayPanelContent() {
  const calendars = useCalendarStore((s) => s.calendars);
  const overrides = useCalendarStore((s) => s.calendarOverrides);
  const setCalendarOverride = useCalendarStore((s) => s.setCalendarOverride);
  const clearCalendarOverride = useCalendarStore((s) => s.clearCalendarOverride);

  const hasAnyCalendar = useMemo(
    () => DEFAULT_SOURCES.some((s) => (calendars[s.id]?.length ?? 0) > 0),
    [calendars],
  );

  return (
    <div className="display-cards">
      <TimeRangeSection />
      {!hasAnyCalendar && (
        <div className="display-empty">
          まずアカウントを接続して、カレンダー一覧を取得してください。
        </div>
      )}
      {hasAnyCalendar && DEFAULT_SOURCES.map((s) => {
        const list = calendars[s.id] ?? [];
        if (list.length === 0) return null;
        return (
          <section key={s.id} className="display-source" style={{ borderLeftColor: s.color }}>
            <h3 className="display-source-title">{s.label}</h3>
            <ul className="display-list">
              {list.map((c) => (
                <CalendarRow
                  key={c.id}
                  sourceFallbackColor={s.color}
                  meta={c}
                  override={overrides[overrideKey(s.id, c.id)]}
                  onPatch={(patch) => setCalendarOverride(s.id, c.id, patch)}
                  onReset={() => clearCalendarOverride(s.id, c.id)}
                />
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}

type RowProps = {
  sourceFallbackColor: string;
  meta: CalendarMeta;
  override: CalendarOverride | undefined;
  onPatch: (patch: CalendarOverride) => void;
  onReset: () => void;
};

function CalendarRow({ sourceFallbackColor, meta, override, onPatch, onReset }: RowProps) {
  const singletonMap = override
    ? { [overrideKey(meta.sourceId, meta.id)]: override }
    : {};
  const effectiveColor = effectiveCalendarColor(meta, singletonMap, sourceFallbackColor);
  const effectiveName = effectiveCalendarName(meta, singletonMap, meta.name);

  // Local label buffer keeps typing snappy; commit on blur / Enter only.
  const [labelDraft, setLabelDraft] = useState(override?.label ?? "");

  const commitLabel = () => {
    const trimmed = labelDraft.trim();
    if (trimmed === (override?.label ?? "")) return;
    onPatch({ label: trimmed || undefined });
  };

  const commitColor = (next: string) => {
    if (next.toLowerCase() === (override?.color ?? "").toLowerCase()) return;
    onPatch({ color: next });
  };

  const hasAnyOverride = Boolean(override?.color || override?.label);

  return (
    <li className="display-row">
      <span
        className="display-swatch"
        style={{ background: effectiveColor }}
        aria-hidden
        title={`現在の色: ${effectiveColor}`}
      />
      <div className="display-row-main">
        <div className="display-row-name">
          <span className="display-name-current">{effectiveName}</span>
          {meta.isPrimary && <span className="primary-tag">primary</span>}
          {!meta.isWritable && <span className="ro-badge">RO</span>}
        </div>
        <div className="display-row-controls">
          <label className="display-field">
            <span className="display-field-label">表示名</span>
            <input
              type="text"
              value={labelDraft}
              placeholder={meta.name}
              onChange={(e) => setLabelDraft(e.currentTarget.value)}
              onBlur={commitLabel}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.currentTarget.blur();
                }
              }}
            />
          </label>
          <label className="display-field display-field-color">
            <span className="display-field-label">色</span>
            <input
              type="color"
              value={normalizeHex(override?.color ?? meta.color ?? sourceFallbackColor)}
              onChange={(e) => commitColor(e.currentTarget.value)}
              title="色を変更"
            />
          </label>
          <button
            type="button"
            className="display-reset"
            disabled={!hasAnyOverride}
            onClick={() => {
              setLabelDraft("");
              onReset();
            }}
            title="提供元の色・名前に戻す"
          >
            リセット
          </button>
        </div>
      </div>
    </li>
  );
}

// `<input type="color">` requires lower-case `#rrggbb`. Provider colors sometimes come
// through as `#RRGGBB` or without `#`; we round-trip via a safe default for non-hex.
function normalizeHex(input: string): string {
  const m = input.match(/^#?([0-9a-fA-F]{6})$/);
  if (!m) return "#888888";
  return `#${m[1].toLowerCase()}`;
}

/** Top-of-tab section letting the user constrain the time-of-day window for day/week
 *  views. Picking a start ≥ current end auto-bumps the end to start+1 (and vice versa)
 *  so the dropdowns can never produce an inverted range. */
function TimeRangeSection() {
  const viewStartHour = useCalendarStore((s) => s.viewStartHour);
  const viewEndHour = useCalendarStore((s) => s.viewEndHour);
  const setViewHours = useCalendarStore((s) => s.setViewHours);

  const handleStartChange = (next: number) => {
    const safeEnd = next >= viewEndHour ? Math.min(24, next + 1) : viewEndHour;
    setViewHours(next, safeEnd);
  };
  const handleEndChange = (next: number) => {
    const safeStart = next <= viewStartHour ? Math.max(0, next - 1) : viewStartHour;
    setViewHours(safeStart, next);
  };

  const isFullDay = viewStartHour === 0 && viewEndHour === 24;

  return (
    <section className="display-source time-range-section">
      <h3 className="display-source-title">表示時間範囲</h3>
      <div className="time-range-row">
        <label className="time-range-field">
          <span className="time-range-label">開始</span>
          <select
            value={viewStartHour}
            onChange={(e) => handleStartChange(Number(e.currentTarget.value))}
            title="表示開始時刻"
          >
            {START_HOUR_OPTIONS.map((h) => (
              <option key={h} value={h}>
                {formatHour(h)}
              </option>
            ))}
          </select>
        </label>
        <span className="time-range-sep">〜</span>
        <label className="time-range-field">
          <span className="time-range-label">終了</span>
          <select
            value={viewEndHour}
            onChange={(e) => handleEndChange(Number(e.currentTarget.value))}
            title="表示終了時刻"
          >
            {END_HOUR_OPTIONS.map((h) => (
              <option key={h} value={h}>
                {h === 24 ? "24:00 (翌 00:00)" : formatHour(h)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="display-reset"
          disabled={isFullDay}
          onClick={() => setViewHours(0, 24)}
          title="0:00〜24:00 に戻す"
        >
          リセット
        </button>
      </div>
      <p className="time-range-hint">
        日／週ビューで表示する時間帯を制限します。範囲外の予定は非表示になります（終日予定を除く）。
      </p>
    </section>
  );
}
