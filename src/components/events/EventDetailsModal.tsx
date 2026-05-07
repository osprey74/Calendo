import { useState } from "react";
import type { CalendarSourceId, RecurringEditScope, UnifiedEvent } from "../../types";
import { DEFAULT_SOURCES } from "../../types";
import { useCalendarStore } from "../../store/calendarStore";
import { allDayEndInclusive, eventStart, eventEnd } from "../../utils/eventUtils";
import { formatDateHeading, formatTimeShort, isSameDay } from "../../utils/dateUtils";
import { EventModal } from "./EventModal";
import "./EventDetailsModal.css";

function canEditSingleInstance(sourceId: CalendarSourceId): boolean {
  return sourceId !== "icloud";
}

export function EventDetailsModal({
  event,
  onClose,
}: {
  event: UnifiedEvent;
  onClose: () => void;
}) {
  const calendars = useCalendarStore((s) => s.calendars);
  const deleteEvent = useCalendarStore((s) => s.deleteEvent);
  const source = DEFAULT_SOURCES.find((s) => s.id === event.sourceId);
  const calendar = calendars[event.sourceId]?.find((c) => c.id === event.calendarId);
  const calendarColor = calendar?.color ?? source?.color ?? "#888";

  const [editing, setEditing] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const start = eventStart(event);
  const end = eventEnd(event);
  const startDate = formatDateHeading(start);
  const inclusiveEnd = event.isAllDay ? allDayEndInclusive(event) : end;
  const endDate = formatDateHeading(inclusiveEnd);
  const allDaySingleDay = event.isAllDay && isSameDay(start, inclusiveEnd);

  const isWritable = calendar?.isWritable ?? false;
  const writeBlockReason = !calendar
    ? "カレンダー一覧未取得"
    : !calendar.isWritable
      ? "読み取り専用カレンダー"
      : null;

  // For recurring events on Graph/GCal, "this" means the per-instance id; "all" requires
  // resolving to the master id. CalDAV only supports "all" since writes are resource-level.
  const performDelete = async (scope: RecurringEditScope | undefined) => {
    setDeleting(true);
    setActionError(null);
    try {
      const targetId =
        scope === "all" && event.recurringEventId ? event.recurringEventId : event.id;
      await deleteEvent(event.sourceId, event.calendarId, targetId, scope);
      onClose();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setDeleting(false);
    }
  };

  if (editing) {
    return <EventModal mode={{ kind: "edit", event }} onClose={onClose} />;
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal event-details"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <header className="modal-head">
          <h2>{event.title || "(無題)"}</h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="閉じる">
            ×
          </button>
        </header>

        <div className="event-details-body">
          <Row label="ソース">
            <span className="source-pill" style={{ borderLeftColor: source?.color }}>
              {source?.label ?? event.sourceId}
            </span>
            <code className="source-id">{event.sourceId}</code>
          </Row>

          <Row label="カレンダー">
            <span className="cal-pill" style={{ borderLeftColor: calendarColor }}>
              {calendar?.name ?? "(取得済み一覧外)"}
              {calendar?.isPrimary && <span className="primary-tag"> primary</span>}
            </span>
            {!calendar?.isWritable && calendar !== undefined && (
              <span className="ro-badge" title="読み取り専用">RO</span>
            )}
            <code className="cal-id">{event.calendarId}</code>
          </Row>

          <Row label="日時">
            {event.isAllDay ? (
              <span>
                {startDate}
                {!allDaySingleDay && <> – {endDate}</>}
                <span className="muted"> · 終日</span>
              </span>
            ) : (
              <span>
                {startDate} {formatTimeShort(start)} – {formatTimeShort(end)}
              </span>
            )}
          </Row>

          {event.location && <Row label="場所">{event.location}</Row>}

          {event.body && (
            <Row label="メモ">
              <div className="event-body-text">{event.body}</div>
            </Row>
          )}

          {event.isRecurring && (
            <Row label="繰り返し">
              <span className="muted">
                {event.recurrenceRule
                  ? `RRULE: ${event.recurrenceRule}`
                  : "繰り返しイベントのインスタンス"}
              </span>
            </Row>
          )}

          <Row label="イベント ID">
            <code className="event-id">{event.id}</code>
          </Row>

          {actionError && <div className="event-form-error">{actionError}</div>}

          {confirmingDelete ? (
            <DeleteConfirm
              event={event}
              busy={deleting}
              onCancel={() => setConfirmingDelete(false)}
              onConfirm={performDelete}
            />
          ) : (
            <div className="event-details-actions">
              {writeBlockReason && (
                <span className="hint warn">{writeBlockReason}</span>
              )}
              <button
                type="button"
                className="danger"
                disabled={!isWritable || deleting}
                onClick={() => setConfirmingDelete(true)}
              >
                削除
              </button>
              <button
                type="button"
                disabled={!isWritable}
                onClick={() => setEditing(true)}
              >
                編集
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function DeleteConfirm({
  event,
  busy,
  onCancel,
  onConfirm,
}: {
  event: UnifiedEvent;
  busy: boolean;
  onCancel: () => void;
  onConfirm: (scope: RecurringEditScope | undefined) => void;
}) {
  if (!event.isRecurring) {
    return (
      <div className="event-details-confirm">
        <p>「{event.title || "(無題)"}」を削除しますか？</p>
        <div className="event-details-actions">
          <button type="button" className="secondary" onClick={onCancel} disabled={busy}>
            キャンセル
          </button>
          <button
            type="button"
            className="danger"
            onClick={() => onConfirm(undefined)}
            disabled={busy}
          >
            {busy ? "削除中…" : "削除"}
          </button>
        </div>
      </div>
    );
  }

  const allowSingle = canEditSingleInstance(event.sourceId);
  return (
    <div className="event-details-confirm">
      <p>
        繰り返しイベント「{event.title || "(無題)"}」を削除します。
        {allowSingle ? "削除範囲を選択してください。" : "iCloud は系列全体の削除のみ対応しています。"}
      </p>
      <div className="event-details-actions wrap">
        <button type="button" className="secondary" onClick={onCancel} disabled={busy}>
          キャンセル
        </button>
        {allowSingle && (
          <button
            type="button"
            className="danger"
            onClick={() => onConfirm("this")}
            disabled={busy}
          >
            この1件のみ削除
          </button>
        )}
        <button
          type="button"
          className="danger"
          onClick={() => onConfirm("all")}
          disabled={busy}
          title="繰り返し系列全体を削除"
        >
          すべて削除
        </button>
      </div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="event-row">
      <div className="event-row-label">{label}</div>
      <div className="event-row-value">{children}</div>
    </div>
  );
}
