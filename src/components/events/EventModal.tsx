import { useEffect, useMemo, useState } from "react";
import type {
  CalendarMeta,
  CalendarSourceId,
  EventDraft,
  RecurringEditScope,
  UnifiedEvent,
} from "../../types";
import { DEFAULT_SOURCES } from "../../types";
import { useCalendarStore } from "../../store/calendarStore";
import {
  formPartsToIso,
  isoToFormParts,
  ymd,
} from "../../utils/dateUtils";
import { allDayEndInclusive, eventStart } from "../../utils/eventUtils";
import "./EventModal.css";

type Mode =
  | { kind: "create"; defaultDate: Date }
  | { kind: "edit"; event: UnifiedEvent };

/** Whether the source's API allows editing a single instance of a recurring event.
 *  Graph and GCal expose per-instance ids natively; CalDAV writes are always
 *  resource-level so editing a single instance is unsupported in Phase 4.x. */
function canEditSingleInstance(sourceId: CalendarSourceId): boolean {
  return sourceId !== "icloud";
}

export function EventModal({
  mode,
  onClose,
}: {
  mode: Mode;
  onClose: () => void;
}) {
  const calendars = useCalendarStore((s) => s.calendars);
  const createEvent = useCalendarStore((s) => s.createEvent);
  const updateEvent = useCalendarStore((s) => s.updateEvent);

  const initial = useMemo(() => buildInitialState(mode), [mode]);
  const [sourceId, setSourceId] = useState<CalendarSourceId>(initial.sourceId);
  const [calendarId, setCalendarId] = useState<string>(initial.calendarId);
  const [title, setTitle] = useState(initial.title);
  const [isAllDay, setIsAllDay] = useState(initial.isAllDay);
  const [startDate, setStartDate] = useState(initial.startDate);
  const [startTime, setStartTime] = useState(initial.startTime);
  const [endDate, setEndDate] = useState(initial.endDate);
  const [endTime, setEndTime] = useState(initial.endTime);
  const [location, setLocation] = useState(initial.location);
  const [body, setBody] = useState(initial.body);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Scope only matters in edit mode for recurring events. Default to "this" for Graph/GCal
  // (least-surprise: the user usually means the occurrence they clicked) and to "all" for
  // CalDAV (its API only operates on the master resource).
  const isRecurringEdit = mode.kind === "edit" && mode.event.isRecurring;
  const scopeAvailable: RecurringEditScope[] = isRecurringEdit
    ? canEditSingleInstance(initial.sourceId)
      ? ["this", "all"]
      : ["all"]
    : [];
  const [scope, setScope] = useState<RecurringEditScope>(
    isRecurringEdit && canEditSingleInstance(initial.sourceId) ? "this" : "all",
  );

  // Recurrence picker — only exposed in create mode. In edit mode the existing RRULE is
  // preserved by the backend (Graph/GCal omit `recurrence` on PATCH; CalDAV reads back the
  // existing RRULE before PUT). Phase 5+ will add an edit picker that handles arbitrary
  // RRULEs (including the round-trip back into a preset bucket).
  const [recurrencePreset, setRecurrencePreset] = useState<RecurrencePreset>("none");
  const [recurrenceUntil, setRecurrenceUntil] = useState<string>(""); // YYYY-MM-DD or empty

  // When the user changes the source, pick a writable calendar from that source so the
  // 2-stage selector stays in a valid state without forcing the user to re-tap it.
  const writableCalendars = (calendars[sourceId] ?? []).filter((c) => c.isWritable);
  useEffect(() => {
    if (writableCalendars.length === 0) return;
    if (!writableCalendars.some((c) => c.id === calendarId)) {
      const primary = writableCalendars.find((c) => c.isPrimary) ?? writableCalendars[0];
      setCalendarId(primary.id);
    }
  }, [sourceId, calendarId, writableCalendars]);

  /** When the user picks a start date today-or-later, also nudge the end date to match
   *  so the typical "single-day event" path doesn't require a second tap. Past dates
   *  leave the end date alone (assumed to be intentional bookkeeping). */
  const handleStartDateChange = (next: string) => {
    setStartDate(next);
    if (next && next >= ymdToday()) {
      setEndDate(next);
    }
  };

  /** Changing the start time pulls the end forward to start + 1h (rolling to the next
   *  day if start is late enough that the hour spills past midnight). */
  const handleStartTimeChange = (next: string) => {
    setStartTime(next);
    const shifted = shiftClock(startDate, next, 60);
    setEndDate(shifted.date);
    setEndTime(shifted.time);
  };

  const applyDuration = (minutes: number) => {
    const shifted = shiftClock(startDate, startTime, minutes);
    setEndDate(shifted.date);
    setEndTime(shifted.time);
  };

  const validationError = (() => {
    if (!title.trim()) return "タイトルを入力してください";
    if (!calendarId) return "登録先のカレンダーを選択してください";
    if (isAllDay) {
      if (!startDate || !endDate) return "開始日と終了日を入力してください";
      if (endDate < startDate) return "終了日は開始日以降にしてください";
    } else {
      if (!startDate || !startTime || !endDate || !endTime) {
        return "開始・終了の日時を入力してください";
      }
      const startIso = formPartsToIso(startDate, startTime);
      const endIso = formPartsToIso(endDate, endTime);
      if (endIso <= startIso) return "終了は開始より後にしてください";
    }
    return null;
  })();

  const handleSubmit = async () => {
    if (validationError) {
      setError(validationError);
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const draft = buildDraft({
        sourceId,
        calendarId,
        title: title.trim(),
        isAllDay,
        startDate,
        startTime,
        endDate,
        endTime,
        location,
        body,
        // Only emit a recurrence rule on create. On edit we preserve the original.
        recurrenceRule:
          mode.kind === "create"
            ? rrulePresetToString(recurrencePreset, startDate, isAllDay, recurrenceUntil)
            : undefined,
      });
      if (mode.kind === "create") {
        await createEvent(draft);
      } else {
        // For "all" scope on a recurring event, target the series master id (Graph's
        // seriesMasterId / GCal's recurringEventId). CalDAV's id strips its `::recurrence`
        // discriminator inside the backend, so passing event.id always operates on the
        // master — `all` is the only meaningful scope there.
        const target =
          scope === "all" && mode.event.recurringEventId
            ? mode.event.recurringEventId
            : mode.event.id;
        await updateEvent(target, draft, isRecurringEdit ? scope : undefined);
      }
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const isEdit = mode.kind === "edit";

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal event-modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <header className="modal-head">
          <h2>{isEdit ? "イベントを編集" : "新規イベント"}</h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="閉じる">
            ×
          </button>
        </header>

        <div className="event-form">
          <Field label="タイトル" required>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.currentTarget.value)}
              placeholder="例：デザインレビュー"
              autoFocus
            />
          </Field>

          <Field label="ソース">
            <select
              value={sourceId}
              onChange={(e) => setSourceId(e.currentTarget.value as CalendarSourceId)}
              disabled={isEdit}
              title="ソース"
            >
              {DEFAULT_SOURCES.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
                </option>
              ))}
            </select>
            {isEdit && (
              <span className="hint">
                ソース変更（移動）は未対応 · 削除＋新規作成で対応してください
              </span>
            )}
          </Field>

          <Field label="カレンダー">
            <select
              value={calendarId}
              onChange={(e) => setCalendarId(e.currentTarget.value)}
              disabled={isEdit}
              title="カレンダー"
            >
              <CalendarOptions calendars={writableCalendars} currentId={calendarId} />
            </select>
            {writableCalendars.length === 0 && (
              <span className="hint warn">
                このソースに書き込み可能なカレンダーがありません
              </span>
            )}
          </Field>

          <Field label="">
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={isAllDay}
                onChange={(e) => setIsAllDay(e.currentTarget.checked)}
              />
              <span>終日</span>
            </label>
          </Field>

          {isRecurringEdit && (
            <Field label="編集範囲">
              <div className="scope-row">
                <ScopeOption
                  label="この1件のみ"
                  value="this"
                  current={scope}
                  available={scopeAvailable}
                  onSelect={setScope}
                />
                <ScopeOption
                  label="この日以降すべて"
                  value="this_and_following"
                  current={scope}
                  available={scopeAvailable}
                  onSelect={setScope}
                  disabledHint="未対応（Phase 5+）"
                />
                <ScopeOption
                  label="すべて"
                  value="all"
                  current={scope}
                  available={scopeAvailable}
                  onSelect={setScope}
                />
              </div>
              {!canEditSingleInstance(initial.sourceId) && (
                <span className="hint">
                  iCloud では繰り返しイベントの個別編集は未対応です（マスタ全体に適用されます）
                </span>
              )}
            </Field>
          )}

          {mode.kind === "create" && (
            <Field label="繰り返し">
              <select
                value={recurrencePreset}
                onChange={(e) =>
                  setRecurrencePreset(e.currentTarget.value as RecurrencePreset)
                }
                title="繰り返し"
              >
                <option value="none">繰り返しなし</option>
                <option value="daily">毎日</option>
                <option value="weekly">毎週（{weekdayLabel(startDate)}）</option>
                <option value="weekdays">平日のみ（月〜金）</option>
                <option value="monthly">毎月（{dayOfMonthLabel(startDate)}）</option>
                <option value="yearly">毎年（{monthDayLabel(startDate)}）</option>
              </select>
              {recurrencePreset !== "none" && (
                <div className="datetime-row">
                  <span className="hint">終了日（任意）:</span>
                  <input
                    type="date"
                    value={recurrenceUntil}
                    onChange={(e) => setRecurrenceUntil(e.currentTarget.value)}
                    title="繰り返し終了日"
                  />
                  {recurrenceUntil && (
                    <button
                      type="button"
                      className="duration-btn"
                      onClick={() => setRecurrenceUntil("")}
                    >
                      クリア
                    </button>
                  )}
                </div>
              )}
            </Field>
          )}

          {mode.kind === "edit" && mode.event.isRecurring && (
            <Field label="繰り返し">
              <span className="hint">
                {mode.event.recurrenceRule
                  ? `現在の RRULE: ${mode.event.recurrenceRule}（変更不可・Phase 5+）`
                  : "繰り返しイベント（ルール変更は Phase 5+）"}
              </span>
            </Field>
          )}

          <Field label="開始">
            <div className="datetime-row">
              <input
                type="date"
                value={startDate}
                onChange={(e) => handleStartDateChange(e.currentTarget.value)}
                title="開始日"
              />
              {!isAllDay && (
                <input
                  type="time"
                  value={startTime}
                  onChange={(e) => handleStartTimeChange(e.currentTarget.value)}
                  title="開始時刻"
                />
              )}
            </div>
          </Field>

          <Field label="終了">
            <div className="datetime-row">
              <input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.currentTarget.value)}
                title="終了日"
              />
              {!isAllDay && (
                <input
                  type="time"
                  value={endTime}
                  onChange={(e) => setEndTime(e.currentTarget.value)}
                  title="終了時刻"
                />
              )}
            </div>
            {isAllDay && (
              <span className="hint">終了日は当日を含めて指定（その日の終わりまで）</span>
            )}
            {!isAllDay && (
              <div className="duration-row">
                <span className="duration-label">所要:</span>
                {DURATION_PRESETS.map((p) => (
                  <button
                    key={p.minutes}
                    type="button"
                    className="duration-btn"
                    onClick={() => applyDuration(p.minutes)}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            )}
          </Field>

          <Field label="場所">
            <input
              type="text"
              value={location}
              onChange={(e) => setLocation(e.currentTarget.value)}
              placeholder="任意"
            />
          </Field>

          <Field label="メモ">
            <textarea
              value={body}
              onChange={(e) => setBody(e.currentTarget.value)}
              rows={4}
              placeholder="任意"
            />
          </Field>

          {error && <div className="event-form-error">{error}</div>}

          <div className="event-form-actions">
            <button type="button" className="secondary" onClick={onClose} disabled={submitting}>
              キャンセル
            </button>
            <button
              type="button"
              onClick={handleSubmit}
              disabled={submitting || !!validationError}
            >
              {submitting ? "保存中…" : isEdit ? "更新" : "作成"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  required,
  children,
}: {
  label: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="event-field">
      <label className="event-field-label">
        {label}
        {required && <span className="required-mark">*</span>}
      </label>
      <div className="event-field-input">{children}</div>
    </div>
  );
}

function ScopeOption({
  label,
  value,
  current,
  available,
  onSelect,
  disabledHint,
}: {
  label: string;
  value: RecurringEditScope;
  current: RecurringEditScope;
  available: RecurringEditScope[];
  onSelect: (s: RecurringEditScope) => void;
  disabledHint?: string;
}) {
  const enabled = available.includes(value);
  return (
    <button
      type="button"
      className={`scope-btn ${current === value ? "active" : ""}`}
      disabled={!enabled}
      title={!enabled ? disabledHint ?? "未対応" : label}
      onClick={() => onSelect(value)}
    >
      {label}
    </button>
  );
}

function CalendarOptions({
  calendars,
  currentId,
}: {
  calendars: CalendarMeta[];
  currentId: string;
}) {
  if (calendars.length === 0) {
    return <option value="">（書き込み可能なカレンダーなし）</option>;
  }
  if (!calendars.some((c) => c.id === currentId)) {
    return (
      <>
        <option value="">選択…</option>
        {calendars.map((c) => (
          <option key={c.id} value={c.id}>
            {c.name}
            {c.isPrimary ? " (primary)" : ""}
          </option>
        ))}
      </>
    );
  }
  return (
    <>
      {calendars.map((c) => (
        <option key={c.id} value={c.id}>
          {c.name}
          {c.isPrimary ? " (primary)" : ""}
        </option>
      ))}
    </>
  );
}

function buildInitialState(mode: Mode) {
  if (mode.kind === "edit") {
    const e = mode.event;
    if (e.isAllDay) {
      const start = eventStart(e);
      // UnifiedEvent.end is exclusive for all-day; the form uses inclusive ends.
      const inclusiveEnd = allDayEndInclusive(e);
      return {
        sourceId: e.sourceId,
        calendarId: e.calendarId,
        title: e.title,
        isAllDay: true,
        startDate: ymd(start),
        startTime: "00:00",
        endDate: ymd(inclusiveEnd),
        endTime: "00:00",
        location: e.location ?? "",
        body: e.body ?? "",
      };
    }
    const startParts = isoToFormParts(e.start);
    const endParts = isoToFormParts(e.end);
    return {
      sourceId: e.sourceId,
      calendarId: e.calendarId,
      title: e.title,
      isAllDay: false,
      startDate: startParts.date,
      startTime: startParts.time,
      endDate: endParts.date,
      endTime: endParts.time,
      location: e.location ?? "",
      body: e.body ?? "",
    };
  }
  // Create mode: ceil "now" to the next 30-minute boundary for start, +1h for end.
  // 18:10 → 18:30 / 19:30. 18:30 → 18:30 / 19:30 (already on boundary). 23:50 → 00:00
  // tomorrow / 01:00 tomorrow (rolls past midnight).
  const startCeil = ceilToHalfHour(mode.defaultDate);
  const endShift = shiftClock(startCeil.date, startCeil.time, 60);
  return {
    sourceId: "ms365_work1" as CalendarSourceId,
    calendarId: "",
    title: "",
    isAllDay: false,
    startDate: startCeil.date,
    startTime: startCeil.time,
    endDate: endShift.date,
    endTime: endShift.time,
    location: "",
    body: "",
  };
}

/** Round a Date forward to the next 30-minute boundary, returning matching form parts.
 *  Already-on-boundary values are kept as-is (so 18:30 stays 18:30). Rolling past
 *  midnight is handled by re-deriving the date string from the rolled Date. */
function ceilToHalfHour(d: Date): { date: string; time: string } {
  const minutes = d.getHours() * 60 + d.getMinutes();
  const ceiled = Math.ceil(minutes / 30) * 30;
  const out = new Date(d);
  out.setSeconds(0, 0);
  if (ceiled >= 24 * 60) {
    out.setDate(out.getDate() + 1);
    out.setHours(0, 0, 0, 0);
  } else {
    out.setHours(Math.floor(ceiled / 60), ceiled % 60, 0, 0);
  }
  const hh = String(out.getHours()).padStart(2, "0");
  const mm = String(out.getMinutes()).padStart(2, "0");
  return { date: ymd(out), time: `${hh}:${mm}` };
}

type RecurrencePreset = "none" | "daily" | "weekly" | "weekdays" | "monthly" | "yearly";

const WEEKDAY_RRULE_TOKENS = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"] as const;
const WEEKDAY_LABELS_JA = ["日", "月", "火", "水", "木", "金", "土"] as const;

function parseFormDateLocal(date: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return null;
  const [y, m, d] = date.split("-").map(Number);
  return new Date(y, m - 1, d);
}

function weekdayLabel(date: string): string {
  const d = parseFormDateLocal(date);
  return d ? `${WEEKDAY_LABELS_JA[d.getDay()]}曜日` : "—";
}

function dayOfMonthLabel(date: string): string {
  const d = parseFormDateLocal(date);
  return d ? `${d.getDate()}日` : "—";
}

function monthDayLabel(date: string): string {
  const d = parseFormDateLocal(date);
  return d ? `${d.getMonth() + 1}月${d.getDate()}日` : "—";
}

/** Compose an RFC 5545 RRULE from a preset + form context. Returns `undefined` for the
 *  "none" preset so the draft omits the field cleanly. */
function rrulePresetToString(
  preset: RecurrencePreset,
  startDate: string,
  isAllDay: boolean,
  until: string,
): string | undefined {
  if (preset === "none") return undefined;
  const startLocal = parseFormDateLocal(startDate);
  if (!startLocal) return undefined;

  let base: string;
  switch (preset) {
    case "daily":
      base = "FREQ=DAILY";
      break;
    case "weekly":
      base = `FREQ=WEEKLY;BYDAY=${WEEKDAY_RRULE_TOKENS[startLocal.getDay()]}`;
      break;
    case "weekdays":
      base = "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR";
      break;
    case "monthly":
      base = "FREQ=MONTHLY";
      break;
    case "yearly":
      base = "FREQ=YEARLY";
      break;
  }

  if (until && /^\d{4}-\d{2}-\d{2}$/.test(until)) {
    const compact = until.replace(/-/g, "");
    // For all-day events, RFC 5545 allows date-only UNTIL. For timed events, providers
    // expect a UTC datetime — normalize to end-of-day Z so the event on the UNTIL date
    // is still included.
    base += isAllDay ? `;UNTIL=${compact}` : `;UNTIL=${compact}T235959Z`;
  }
  return base;
}

const DURATION_PRESETS: { label: string; minutes: number }[] = [
  { label: "10分", minutes: 10 },
  { label: "15分", minutes: 15 },
  { label: "30分", minutes: 30 },
  { label: "1時間", minutes: 60 },
  { label: "90分", minutes: 90 },
  { label: "2時間", minutes: 120 },
  { label: "3時間", minutes: 180 },
];

function ymdToday(): string {
  return ymd(new Date());
}

/** Add `n` calendar days to a YYYY-MM-DD string using local date arithmetic so JST
 *  day boundaries are respected (no timezone shift via UTC parsing). */
function addDaysToYmd(date: string, n: number): string {
  const [y, m, d] = date.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + n);
  const yy = dt.getFullYear();
  const mm = String(dt.getMonth() + 1).padStart(2, "0");
  const dd = String(dt.getDate()).padStart(2, "0");
  return `${yy}-${mm}-${dd}`;
}

/** Shift a (date, time) pair forward by `minutes`. Rolls into following calendar days as
 *  needed; 23:30 + 90min returns the next day at 01:00. */
function shiftClock(
  date: string,
  time: string,
  minutes: number,
): { date: string; time: string } {
  const [h, m] = time.split(":").map(Number);
  if (Number.isNaN(h) || Number.isNaN(m) || !date) {
    return { date, time };
  }
  let total = h * 60 + m + minutes;
  let dayOffset = 0;
  while (total >= 24 * 60) {
    total -= 24 * 60;
    dayOffset += 1;
  }
  while (total < 0) {
    total += 24 * 60;
    dayOffset -= 1;
  }
  const eh = Math.floor(total / 60);
  const em = total % 60;
  return {
    date: dayOffset === 0 ? date : addDaysToYmd(date, dayOffset),
    time: `${String(eh).padStart(2, "0")}:${String(em).padStart(2, "0")}`,
  };
}

function buildDraft(args: {
  sourceId: CalendarSourceId;
  calendarId: string;
  title: string;
  isAllDay: boolean;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
  location: string;
  body: string;
  recurrenceRule?: string;
}): EventDraft {
  const { sourceId, calendarId, title, isAllDay, startDate, endDate } = args;
  const start = isAllDay ? startDate : formPartsToIso(startDate, args.startTime);
  const end = isAllDay ? endDate : formPartsToIso(endDate, args.endTime);
  return {
    sourceId,
    calendarId,
    title,
    start,
    end,
    isAllDay,
    location: args.location.trim() || undefined,
    body: args.body.trim() || undefined,
    recurrenceRule: args.recurrenceRule,
  };
}
