/**
 * Frontend-side classification of errors raised by Tauri commands.
 *
 * The Rust side serializes `AppError` as `{kind, message, status?, sourceId?}` (see
 * `src-tauri/src/error.rs`). Tauri's `invoke` rejects with that payload directly when
 * the command returns `Err`. This module turns the raw payload into a typed
 * `ClassifiedError` and a localized Japanese user-facing message.
 *
 * Catch-site usage:
 *   try { ... } catch (e) { const c = classifyError(e); toast.error(c.userMessage); }
 */

import type { CalendarSourceId, SourceDescriptor } from "../types";
import { DEFAULT_SOURCES } from "../types";

/** Stable kind strings emitted by `AppError::kind()` on the Rust side. The
 *  exhaustive enum lets switch statements compile-time-check coverage. */
export type ErrorKind =
  | "auth_required"
  | "not_authenticated"
  | "permission"
  | "not_found"
  | "conflict"
  | "rate_limit"
  | "server"
  | "network"
  | "oauth"
  | "oauth_timeout"
  | "missing_credential"
  | "unknown_source"
  | "caldav"
  | "keyring"
  | "io"
  | "http"
  | "other";

export type ClassifiedError = {
  kind: ErrorKind;
  /** Original message from the backend (often English / technical). Useful for logs. */
  rawMessage: string;
  /** HTTP status when applicable. */
  status?: number;
  /** Source id when the error is tied to a specific account. */
  sourceId?: CalendarSourceId;
  /** Localized Japanese message safe to show in a toast or modal. */
  userMessage: string;
};

type RawAppError = {
  kind?: string;
  message?: string;
  status?: number | null;
  sourceId?: string | null;
};

function isCalendarSourceId(v: unknown): v is CalendarSourceId {
  return (
    v === "ms365_work1" || v === "google_gws" || v === "icloud"
  );
}

function sourceLabel(id: CalendarSourceId | undefined): string {
  if (!id) return "";
  const d: SourceDescriptor | undefined = DEFAULT_SOURCES.find((s) => s.id === id);
  return d?.label ?? id;
}

/** Convert anything `invoke()` rejected with into a structured error. Tauri delivers
 *  AppError as the parsed JSON object directly, but defensive coding handles both
 *  string-shaped legacy errors and JS-thrown Error instances. */
export function classifyError(raw: unknown): ClassifiedError {
  const parsed = parseRaw(raw);
  const kind = normalizeKind(parsed.kind);
  const sourceId = isCalendarSourceId(parsed.sourceId) ? parsed.sourceId : undefined;
  const status = typeof parsed.status === "number" ? parsed.status : undefined;
  return {
    kind,
    rawMessage: parsed.message ?? "",
    status,
    sourceId,
    userMessage: buildUserMessage(kind, sourceId, status, parsed.message ?? ""),
  };
}

function parseRaw(raw: unknown): RawAppError {
  if (raw == null) return {};
  if (typeof raw === "string") {
    // Legacy / non-structured error (e.g., thrown from front-end). Best-effort parse.
    try {
      const j = JSON.parse(raw);
      if (j && typeof j === "object") return j as RawAppError;
    } catch {
      // not JSON
    }
    return { message: raw };
  }
  if (raw instanceof Error) {
    return { message: raw.message };
  }
  if (typeof raw === "object") {
    return raw as RawAppError;
  }
  return { message: String(raw) };
}

function normalizeKind(k: string | undefined): ErrorKind {
  switch (k) {
    case "auth_required":
    case "not_authenticated":
    case "permission":
    case "not_found":
    case "conflict":
    case "rate_limit":
    case "server":
    case "network":
    case "oauth":
    case "oauth_timeout":
    case "missing_credential":
    case "unknown_source":
    case "caldav":
    case "keyring":
    case "io":
    case "http":
      return k;
    default:
      return "other";
  }
}

function buildUserMessage(
  kind: ErrorKind,
  sourceId: CalendarSourceId | undefined,
  status: number | undefined,
  rawMessage: string,
): string {
  const acct = sourceId ? `${sourceLabel(sourceId)}：` : "";
  switch (kind) {
    case "auth_required":
      return `${acct}認証期限が切れました。設定から再ログインしてください。`;
    case "not_authenticated":
      return `${acct}まだ接続されていません。設定から接続してください。`;
    case "permission":
      return `${acct}この操作には権限がありません（${status ?? 403}）。`;
    case "not_found":
      return `${acct}対象が見つかりませんでした（${status ?? 404}）。他のクライアントで削除された可能性があります。`;
    case "conflict":
      return `${acct}データ競合が発生しました（${status ?? 412}）。最新を取得してからやり直してください。`;
    case "rate_limit":
      return `${acct}APIのレート制限に達しました（${status ?? 429}）。しばらく待ってから再試行してください。`;
    case "server":
      return `${acct}サーバーエラーが発生しました（${status ?? "5xx"}）。時間をおいて再試行してください。`;
    case "network":
      return "ネットワークエラーが発生しました。接続を確認して再試行してください。";
    case "oauth":
      return `${acct}OAuth 認証に失敗しました：${rawMessage}`;
    case "oauth_timeout":
      return "OAuth のコールバック待機がタイムアウトしました。再度お試しください。";
    case "missing_credential":
      return `OAuth クライアント情報が未設定です：${rawMessage}`;
    case "unknown_source":
      return `不明なカレンダーソースです：${rawMessage}`;
    case "caldav":
      return `${acct}CalDAV エラー：${rawMessage}`;
    case "keyring":
      return `OS のキーリングアクセスに失敗しました：${rawMessage}`;
    case "io":
      return `入出力エラー：${rawMessage}`;
    case "http":
      return `${acct}通信エラー：${rawMessage}`;
    case "other":
    default:
      return rawMessage || "エラーが発生しました。";
  }
}

/** True when the error indicates the user needs to re-authenticate this source. */
export function isAuthRequired(c: ClassifiedError): boolean {
  return c.kind === "auth_required" || c.kind === "not_authenticated";
}
