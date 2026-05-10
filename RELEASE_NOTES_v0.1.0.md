# Calendo v0.1.0

**Release date**: 2026-05-10
**Initial release** — first tagged build of Calendo, a unified desktop calendar client for Microsoft 365, Google Workspace, and iCloud.

---

## English

### Highlights

This is the first public release of Calendo. The app aggregates a Microsoft 365 work account, a Google Workspace account, and an iCloud account into a single day/week view, all in JST.

### Features

- **Three-source calendar aggregation** — Microsoft 365 (Microsoft Graph), Google Workspace (Calendar API v3), iCloud (CalDAV)
- **Day / Week views** — timezone-aware (JST) layout with absolute-position grid, lane assignment for overlapping events, and a sticky time grid header
- **Sidebar filtering** — toggle visibility per source and per subcalendar, with bulk on/off, count badges, hidden-calendar collapse, and read-only badges
- **Event create / edit / delete** — two-step source × calendar selection, validation (title required, end > start, writable calendar required), Toast feedback on success / failure
- **Recurring events** — RRULE preset picker at creation time (none / daily / weekly / weekdays / monthly / yearly), optional UNTIL date, and edit/delete scope ("this only" / "all"); CalDAV write supports "all" only
- **OAuth & credentials** — OAuth2 + PKCE for Microsoft / Google with automatic 401 → refresh → retry; iCloud uses an app-specific password verified by PROPFIND before save; all credentials stored in OS Keychain via the `keyring` crate (with chunked storage for MS Graph refresh tokens > 2560 bytes on Windows)
- **Persisted UI state** — view mode and source/subcalendar visibility persist across launches via the Tauri store plugin
- **Onboarding** — welcome card on first launch when no calendars are connected, with a direct link to the connection settings
- **Cross-platform builds** — Windows x86_64 and macOS universal artifacts via GitHub Actions on tag push

### Known Limitations

These are deferred to a future release:

- Edit-mode RRULE changes (preset reverse-lookup + custom editor)
- "This and following" recurrence scope (Graph / GCal: requires composite operations; CalDAV: not yet implemented)
- CalDAV "this only" recurrence handling (`RECURRENCE-ID` partial overlay, `EXDATE` for single-instance delete)
- Per-subcalendar color / label customization
- Moving events between sources / calendars (workaround: delete + recreate)
- Comprehensive error-handling matrix (current behavior: surface error string in Toast)

### Setup

See [README.md](README.md) for OAuth client registration, `.env` setup, and build / dev instructions.

### Platforms

- Windows 10 / 11 (x86_64)
- macOS 12+ (universal — Apple Silicon + Intel)

---

## 日本語

### ハイライト

Calendo の初回公開リリースです。Microsoft 365 仕事アカウント、Google Workspace アカウント、iCloud アカウントを 1 つの日次／週次ビュー（JST 統一）に集約します。

### 機能

- **3 ソース統合** — Microsoft 365（Microsoft Graph）、Google Workspace（Calendar API v3）、iCloud（CalDAV）
- **日次／週次ビュー** — JST タイムゾーン統一の絶対配置グリッド、重複イベントのレーン割当、sticky 時間軸ヘッダ
- **サイドバーフィルタ** — ソース・サブカレンダー単位の表示 ON/OFF、ソース毎の一括切替、件数バッジ、非表示カレンダーの折りたたみ、読み取り専用バッジ
- **イベント新規・編集・削除** — ソース × カレンダーの 2 段階選択、バリデーション（タイトル必須・終了 > 開始・書き込み可能カレンダー必須）、結果を Toast で通知
- **繰り返しイベント** — 作成時に RRULE プリセット（なし／毎日／毎週／平日のみ／毎月／毎年）と任意の UNTIL を指定可能。編集／削除スコープは「この1件のみ」「すべて」をサポート（CalDAV は「すべて」のみ）
- **OAuth と資格情報** — Microsoft / Google は OAuth2 + PKCE、401 受信時に自動リフレッシュ＋リトライ。iCloud はアプリ専用パスワードを保存前に PROPFIND で疎通確認。資格情報はすべて OS Keychain に保管（`keyring` crate 経由。Windows の 2560 byte 制限超え MS Graph refresh token はチャンク分割保存）
- **UI 設定の永続化** — 表示モードとソース／サブカレンダー ON/OFF を Tauri store plugin で永続化
- **オンボーディング** — 未接続状態での初回起動時に「ようこそ」カードを表示し、接続設定への導線を提供
- **クロスプラットフォームビルド** — GitHub Actions のタグプッシュ起動で Windows x86_64・macOS universal の成果物を生成

### 既知の制限事項

以下は次回以降のリリースに先送り：

- 編集モードでの RRULE 変更（プリセット逆引き＋カスタムエディタ）
- 「この日以降すべて」スコープ（Graph / GCal は複合操作が必要、CalDAV は未実装）
- CalDAV 繰り返しの「この1件のみ」対応（`RECURRENCE-ID` 部分上書き、`EXDATE` 追記による単一削除）
- サブカレンダーの色・ラベルカスタマイズ
- イベントのソース／カレンダー間移動（現状は削除＋再作成で対応）
- エラーハンドリングの全網羅（現状は Toast にエラー文字列を表示するのみ）

### セットアップ

OAuth クライアントの登録、`.env` の作成、開発／ビルド手順は [README.ja.md](README.ja.md) を参照してください。

### 対応プラットフォーム

- Windows 10 / 11（x86_64）
- macOS 12+（universal — Apple Silicon + Intel）
