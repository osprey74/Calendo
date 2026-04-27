import { useEffect, useState } from "react";
import { DEFAULT_SOURCES } from "./types";
import type { AuthStatus, CalendarMeta, CalendarSourceId, SourceDescriptor } from "./types";
import {
  authIcloudSave,
  authRevoke,
  authStart,
  authStatus,
  calendarsFetch,
} from "./lib/tauri";
import "./App.css";

function App() {
  const [statuses, setStatuses] = useState<Record<CalendarSourceId, AuthStatus | null>>({
    ms365_work1: null,
    ms365_work2: null,
    google_gws: null,
    icloud: null,
  });
  const [busy, setBusy] = useState<CalendarSourceId | null>(null);
  const [calendars, setCalendars] = useState<Record<CalendarSourceId, CalendarMeta[] | null>>({
    ms365_work1: null,
    ms365_work2: null,
    google_gws: null,
    icloud: null,
  });
  const [errors, setErrors] = useState<Record<CalendarSourceId, string | null>>({
    ms365_work1: null,
    ms365_work2: null,
    google_gws: null,
    icloud: null,
  });
  const [appleId, setAppleId] = useState("");
  const [appPassword, setAppPassword] = useState("");

  useEffect(() => {
    DEFAULT_SOURCES.forEach(async (s) => {
      try {
        const st = await authStatus(s.id);
        setStatuses((prev) => ({ ...prev, [s.id]: st }));
      } catch (e) {
        setErrors((prev) => ({ ...prev, [s.id]: String(e) }));
      }
    });
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
      setCalendars((prev) => ({ ...prev, [s.id]: null }));
    } catch (e) {
      setError(s.id, String(e));
    } finally {
      setBusy(null);
    }
  };

  const handleListCalendars = async (s: SourceDescriptor) => {
    setBusy(s.id);
    setError(s.id, null);
    try {
      const list = await calendarsFetch(s.id);
      setCalendars((prev) => ({ ...prev, [s.id]: list }));
    } catch (e) {
      setError(s.id, String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <main className="container">
      <header className="topbar">
        <h1>Calendo</h1>
        <span className="phase-badge">Phase 1 — Connection Test</span>
      </header>

      <div className="cards">
        {DEFAULT_SOURCES.map((s) => {
          const st = statuses[s.id];
          const cals = calendars[s.id];
          const err = errors[s.id];
          const isBusy = busy === s.id;
          const connected = st?.connected ?? false;

          return (
            <section key={s.id} className="card" style={{ borderLeftColor: s.color }}>
              <div className="card-head">
                <h2>{s.label}</h2>
                <span className={`status ${connected ? "ok" : "off"}`}>
                  {connected ? "接続済み" : "未接続"}
                </span>
              </div>
              <div className="card-meta">
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

              <div className="actions">
                {connected ? (
                  <>
                    <button
                      type="button"
                      onClick={() => handleListCalendars(s)}
                      disabled={isBusy}
                    >
                      カレンダー一覧取得
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDisconnect(s)}
                      disabled={isBusy}
                      className="secondary"
                    >
                      切断
                    </button>
                  </>
                ) : (
                  <button type="button" onClick={() => handleConnect(s)} disabled={isBusy}>
                    {isBusy ? "処理中…" : "接続"}
                  </button>
                )}
              </div>

              {err && <div className="error">{err}</div>}

              {cals && cals.length > 0 && (
                <ul className="cal-list">
                  {cals.map((c) => (
                    <li key={c.id}>
                      <span
                        className="cal-dot"
                        style={{ background: c.color || s.color }}
                      />
                      <span className="cal-name">
                        {c.name}
                        {c.isPrimary && <span className="primary-tag"> primary</span>}
                      </span>
                      <span className={`cal-perm ${c.isWritable ? "rw" : "ro"}`}>
                        {c.isWritable ? "RW" : "RO"}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
              {cals && cals.length === 0 && (
                <div className="cal-empty">
                  カレンダー 0 件（CalDAV はPhase 2で実装予定）
                </div>
              )}
            </section>
          );
        })}
      </div>
    </main>
  );
}

export default App;
