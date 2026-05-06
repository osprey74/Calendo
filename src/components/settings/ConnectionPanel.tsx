import { useEffect, useState } from "react";
import { DEFAULT_SOURCES } from "../../types";
import type { AuthStatus, CalendarSourceId, ClientDebugInfo, SourceDescriptor } from "../../types";
import {
  authDebugClients,
  authIcloudSave,
  authRevoke,
  authStart,
  authStatus,
} from "../../lib/tauri";
import { useCalendarStore } from "../../store/calendarStore";
import "./ConnectionPanel.css";

export function ConnectionPanel({ onClose }: { onClose: () => void }) {
  const loadCalendars = useCalendarStore((s) => s.loadCalendars);
  const loadEvents = useCalendarStore((s) => s.loadEvents);
  const clearSourceCalendars = useCalendarStore((s) => s.clearSourceCalendars);

  const [statuses, setStatuses] = useState<Record<CalendarSourceId, AuthStatus | null>>({
    ms365_work1: null,
    google_gws: null,
    icloud: null,
  });
  const [busy, setBusy] = useState<CalendarSourceId | null>(null);
  const [errors, setErrors] = useState<Record<CalendarSourceId, string | null>>({
    ms365_work1: null,
    google_gws: null,
    icloud: null,
  });
  const [appleId, setAppleId] = useState("");
  const [appPassword, setAppPassword] = useState("");
  const [debug, setDebug] = useState<ClientDebugInfo | null>(null);

  useEffect(() => {
    DEFAULT_SOURCES.forEach(async (s) => {
      try {
        const st = await authStatus(s.id);
        setStatuses((prev) => ({ ...prev, [s.id]: st }));
      } catch (e) {
        setErrors((prev) => ({ ...prev, [s.id]: String(e) }));
      }
    });
    authDebugClients().then(setDebug).catch(() => {});
  }, []);

  const setError = (id: CalendarSourceId, msg: string | null) =>
    setErrors((prev) => ({ ...prev, [id]: msg }));

  const handleConnect = async (s: SourceDescriptor) => {
    setBusy(s.id);
    setError(s.id, null);
    try {
      if (s.id === "icloud") {
        if (!appleId || !appPassword) {
          throw new Error("Apple ID とアプリ専用パスワードを入力してください");
        }
        const st = await authIcloudSave(appleId, appPassword);
        setStatuses((prev) => ({ ...prev, [s.id]: st }));
        setAppPassword("");
      } else {
        const st = await authStart(s.id);
        setStatuses((prev) => ({ ...prev, [s.id]: st }));
      }
      await loadCalendars(s.id);
      await loadEvents();
    } catch (e) {
      setError(s.id, String(e));
    } finally {
      setBusy(null);
    }
  };

  const handleDisconnect = async (s: SourceDescriptor) => {
    setBusy(s.id);
    setError(s.id, null);
    try {
      await authRevoke(s.id);
      setStatuses((prev) => ({
        ...prev,
        [s.id]: { sourceId: s.id, connected: false },
      }));
      // Drop the cached calendar list so events_fetch doesn't try to use stale ids,
      // and so the onboarding hint can detect "no connections" once all sources are
      // disconnected.
      clearSourceCalendars(s.id);
      await loadEvents();
    } catch (e) {
      setError(s.id, String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal connection-panel"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <header className="modal-head">
          <h2>アカウント接続</h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="閉じる">
            ×
          </button>
        </header>

        {debug && (
          <div className="debug-bar">
            <span>
              MS_CLIENT_ID = <code>{debug.msClientId ?? "(未設定)"}</code>
            </span>
            <span>
              GOOGLE_CLIENT_ID = <code>{debug.googleClientId ?? "(未設定)"}</code>
            </span>
            <span>
              GOOGLE_CLIENT_SECRET ={" "}
              <code>{debug.googleClientSecretConfigured ? "(設定済み)" : "(未設定)"}</code>
            </span>
          </div>
        )}

        <div className="conn-cards">
          {DEFAULT_SOURCES.map((s) => {
            const st = statuses[s.id];
            const err = errors[s.id];
            const isBusy = busy === s.id;
            const connected = st?.connected ?? false;
            return (
              <section key={s.id} className="conn-card" style={{ borderLeftColor: s.color }}>
                <div className="conn-head">
                  <h3>{s.label}</h3>
                  <span className={`status ${connected ? "ok" : "off"}`}>
                    {connected ? "接続済み" : "未接続"}
                  </span>
                </div>
                <div className="conn-meta">
                  <code>{s.id}</code> · <span>{s.protocol}</span>
                  {st?.expiresAt && (
                    <span className="expires">
                      {" "}
                      · expires {new Date(st.expiresAt * 1000).toLocaleString()}
                    </span>
                  )}
                </div>

                {s.id === "icloud" && !connected && (
                  <div className="icloud-form">
                    <input
                      type="email"
                      placeholder="Apple ID"
                      value={appleId}
                      onChange={(e) => setAppleId(e.currentTarget.value)}
                      autoComplete="off"
                    />
                    <input
                      type="password"
                      placeholder="アプリ専用パスワード"
                      value={appPassword}
                      onChange={(e) => setAppPassword(e.currentTarget.value)}
                      autoComplete="off"
                    />
                  </div>
                )}

                <div className="conn-actions">
                  {connected ? (
                    <button
                      type="button"
                      onClick={() => handleDisconnect(s)}
                      disabled={isBusy}
                      className="secondary"
                    >
                      切断
                    </button>
                  ) : (
                    <button type="button" onClick={() => handleConnect(s)} disabled={isBusy}>
                      {isBusy ? "処理中…" : "接続"}
                    </button>
                  )}
                </div>

                {err && <div className="conn-error">{err}</div>}
              </section>
            );
          })}
        </div>
      </div>
    </div>
  );
}
