# DESIGN.md — Calendo

## 概要

複数カレンダーアカウント（Microsoft 365 × 1・Google Workspace × 1・iCloud × 1）を統合し、
日次・週次のスケジュールを一覧表示・登録・編集できるデスクトップアプリ。

---

## 技術スタック

| レイヤー | 採用技術 |
|---|---|
| アプリフレームワーク | Tauri v2 |
| フロントエンド | React + TypeScript + Vite |
| バックエンド（Rust） | Tauri コマンド / Axum は不使用（プロセス内処理） |
| HTTP クライアント | reqwest（Rust） |
| CalDAV パーサ | quick-xml（Rust） |
| 認証情報保管 | keyring（OS ネイティブ Keychain / Credential Manager） |
| 状態管理 | Zustand |
| スタイリング | CSS Modules + CSS Variables |
| アイコン | Material Symbols Rounded（kazahana 準拠） |
| テスト | Vitest（フロント）/ cargo test（Rust） |

---

## カレンダーソース定義

```typescript
type CalendarSourceId =
  | 'ms365_work1'   // Microsoft 365
  | 'google_gws'    // Google Workspace
  | 'icloud';       // iCloud CalDAV

type CalendarSource = {
  id: CalendarSourceId;
  label: string;       // 表示名（ユーザーが設定可能）
  color: string;       // イベント表示色（HEX）
  protocol: 'graph' | 'gcal' | 'caldav';
  enabled: boolean;    // 表示ON/OFF
};
```

### デフォルト色

| ID | ラベル | カラー |
|---|---|---|
| ms365_work1 | 仕事 | `#0582AF` |
| google_gws | Google カレンダー | `#2E7D32` |
| icloud | プライベート | `#888780` |

---

## 統一イベント型

```typescript
// サブカレンダーメタ情報
type CalendarMeta = {
  id: string;            // ソース側のカレンダーID
  sourceId: CalendarSourceId;
  name: string;          // カレンダー名（例：「田中さんの予定」「会議室 A」）
  isPrimary: boolean;
  color?: string;        // ソース側の設定色
  isWritable: boolean;   // 書き込み権限があるか
  enabled: boolean;      // 表示 ON/OFF（アプリ側設定）
};

```typescript
type UnifiedEvent = {
  id: string;                    // ソース側のID（Graph / GCal / CalDAV UID）
  sourceId: CalendarSourceId;
  calendarId: string;            // サブカレンダーID（プライマリ含む）
  title: string;
  start: string;                 // ISO 8601（JST）
  end: string;                   // ISO 8601（JST）
  isAllDay: boolean;
  location?: string;
  body?: string;                 // 説明・メモ
  isRecurring: boolean;
  recurringEventId?: string;     // 繰り返しイベントの親ID
  recurrenceRule?: string;       // RRULE 文字列（表示用）
};

// 繰り返しイベント編集スコープ
type RecurringEditScope =
  | 'this'            // この1件のみ
  | 'this_and_following' // この日以降すべて
  | 'all';            // すべての繰り返し

type EventDraft = Omit<
  UnifiedEvent,
  'id' | 'isRecurring' | 'recurringEventId' | 'recurrenceRule'
>;

type EventUpdateRequest = {
  draft: EventDraft;
  recurringScope?: RecurringEditScope; // 繰り返しイベントの場合のみ
};
```

---

## 認証方式

### Microsoft 365（ms365_work1）

- プロトコル：OAuth2 Authorization Code + PKCE
- ライブラリ：reqwest（Rust）で手実装 / Tauri plugin-oauth でコールバック受信
- エンドポイント：`https://login.microsoftonline.com/common/oauth2/v2.0/`
- スコープ：`Calendars.ReadWrite offline_access User.Read`
- トークン：アクセストークンはインメモリキャッシュ、リフレッシュトークンは keyring にチャンク分割保存
  - キー例：`calendo/ms365_work1.refresh.meta`, `calendo/ms365_work1.refresh.0` ...

### Google Workspace（google_gws）

- プロトコル：OAuth2 Authorization Code + PKCE
- エンドポイント：`https://accounts.google.com/o/oauth2/v2/auth`
- スコープ：`https://www.googleapis.com/auth/calendar`
- トークン：同上、keyring キー `calendo/google_gws/access_token`

### iCloud（icloud）

- プロトコル：CalDAV（HTTPS + Basic 認証）
- エンドポイント：`https://caldav.icloud.com`
- 認証情報：Apple ID + アプリ専用パスワード（keyring 保存）
- keyring キー：`calendo/icloud/app_password`

---

## Tauri コマンド一覧

### 認証系

```rust
// OAuth ブラウザ起動 → コールバック待機 → トークン保存
#[tauri::command]
async fn auth_start(source_id: String) -> Result<(), String>

// トークン有効期限確認・リフレッシュ
#[tauri::command]
async fn auth_refresh(source_id: String) -> Result<(), String>

// 認証情報削除（ログアウト）
#[tauri::command]
async fn auth_revoke(source_id: String) -> Result<(), String>

// iCloud 認証情報保存
#[tauri::command]
async fn auth_icloud_save(apple_id: String, app_password: String) -> Result<(), String>
```

### カレンダー操作系

```rust
// 利用可能なカレンダー（サブカレンダー含む）一覧取得
#[tauri::command]
async fn calendars_fetch(source_id: String) -> Result<Vec<CalendarMeta>, String>

// 指定期間のイベント取得（全ソース or 指定ソース・指定カレンダー）
#[tauri::command]
async fn events_fetch(
    source_ids: Vec<String>,
    calendar_ids: Option<Vec<String>>, // None = 全カレンダー
    date_from: String,   // YYYY-MM-DD
    date_to: String,
) -> Result<Vec<UnifiedEvent>, String>

// イベント新規作成
#[tauri::command]
async fn event_create(
    source_id: String,
    calendar_id: String,  // 登録先サブカレンダーID
    draft: EventDraft,
) -> Result<UnifiedEvent, String>

// イベント更新（繰り返しイベントのスコープ指定に対応）
#[tauri::command]
async fn event_update(
    source_id: String,
    event_id: String,
    request: EventUpdateRequest,  // draft + recurringScope
) -> Result<UnifiedEvent, String>

// イベント削除（繰り返しイベントのスコープ指定に対応）
#[tauri::command]
async fn event_delete(
    source_id: String,
    event_id: String,
    recurring_scope: Option<RecurringEditScope>,
) -> Result<(), String>
```

---

## Rust バックエンド構成

```
src-tauri/src/
  lib.rs                    # Tauri app builder・コマンド登録
  models.rs                 # UnifiedEvent・CalendarSource 型定義
  auth/
    mod.rs
    oauth.rs                # PKCE コード生成・トークン交換・リフレッシュ（M365・Google 共通）
    keyring.rs              # OS Keychain 読み書きラッパー
    icloud.rs               # iCloud アプリ専用パスワード保存
  calendars/
    mod.rs
    graph.rs                # Microsoft Graph API クライアント
    gcal.rs                 # Google Calendar API v3 クライアント
    caldav.rs               # CalDAV クライアント（iCloud）
  commands/
    mod.rs
    auth_commands.rs        # auth_* コマンド実装
    calendar_commands.rs    # events_* / event_* コマンド実装
```

---

## フロントエンド構成

```
src/
  components/
    layout/
      AppShell.tsx           # サイドバー + メインパネルのグリッド
      TopBar.tsx             # タイトル・ビュー切替・日付ナビ・新規ボタン
    sidebar/
      CalendarSidebar.tsx    # ソース一覧・ON/OFF トグル
      CalendarItem.tsx       # 各カレンダーの行
    views/
      DayView.tsx            # 日次ビュー
      WeekView.tsx           # 週次ビュー
    events/
      EventCard.tsx          # イベントカード（一覧表示用）
      EventModal.tsx         # 新規作成・編集モーダル
      AllDayBar.tsx          # 終日イベント表示バー
    settings/
      SettingsModal.tsx      # アカウント接続・カレンダー表示設定
  hooks/
    useEvents.ts             # events_fetch コマンド呼び出し・キャッシュ
    useCalendarSources.ts    # ソース設定・ON/OFF 状態管理
    useAuth.ts               # 認証状態確認・フロー開始
  store/
    calendarStore.ts         # Zustand store（イベントキャッシュ・フィルタ・選択日付）
    settingsStore.ts         # ソース設定・表示設定の永続化（Tauri store plugin）
  utils/
    dateUtils.ts             # 日付操作ユーティリティ（JST 変換・週計算）
    eventUtils.ts            # イベント正規化・ソート
  types/
    index.ts                 # フロントエンド共通型定義
  App.tsx
  main.tsx
```

---

## 画面構成

### メイン画面

```
┌─ TopBar ──────────────────────────────────────────────┐
│ Calendo  [日|週]  ‹ 2025年4月27日（日） ›  今日  [＋新規] │
└───────────────────────────────────────────────────────┘
┌─ Sidebar ───┬─ MainPanel ─────────────────────────────┐
│ Microsoft   │                                          │
│ 365         │  DayView or WeekView                     │
│  ● 仕事    │                                          │
│             │                                          │
│ Google      │                                          │
│  ● GWS     │                                          │
│             │                                          │
│ iCloud      │                                          │
│  ● プライベ │                                          │
│             │                                          │
│ [設定]      │                                          │
└─────────────┴──────────────────────────────────────────┘
```

### EventModal（新規作成・編集）

- タイトル（必須）
- 開始日時・終了日時
- 終日フラグ
- 場所（任意）
- メモ（任意）
- 登録先カレンダー選択（ラジオ or セレクト）
- 保存・キャンセルボタン

### SettingsModal

- アカウントごとの接続状態（接続済み / 未接続）
- 接続・再接続・切断ボタン
- iCloud: Apple ID・アプリ専用パスワード入力フォーム
- サブカレンダー一覧（接続後に取得）
  - 各カレンダーの表示 ON/OFF トグル
  - 表示名・色のカスタマイズ
  - 書き込み不可カレンダーは読み取り専用表示

---

## データフロー

```
フロントエンド（React）
  │
  │  invoke('events_fetch', { source_ids, date_from, date_to })
  ▼
Tauri コマンド（Rust）
  │
  ├─ graph.rs   → GET https://graph.microsoft.com/v1.0/me/calendarView
  ├─ gcal.rs    → GET https://www.googleapis.com/calendar/v3/calendars/primary/events
  └─ caldav.rs  → REPORT https://caldav.icloud.com/...
  │
  │  Vec<UnifiedEvent>（正規化済み・JST 変換済み）
  ▼
Zustand store（キャッシュ）
  │
  ▼
DayView / WeekView（レンダリング）
```

---

## エラーハンドリング方針

| ケース | 対応 |
|---|---|
| トークン期限切れ | 自動リフレッシュ → 失敗時は再認証を促す Toast 表示 |
| ネットワークエラー | Toast でエラー通知・キャッシュデータを維持表示 |
| CalDAV 認証失敗 | 設定画面にリダイレクト |
| イベント作成失敗 | モーダルを閉じずエラー表示・入力内容を保持 |

---

## 永続化

| データ | 保存場所 |
|---|---|
| OAuth トークン | OS Keychain（keyring crate） |
| iCloud パスワード | OS Keychain（keyring crate） |
| カレンダー表示設定（ソース・サブカレンダー ON/OFF・色・表示名） | Tauri store plugin（`~/.config/calendo/settings.json`） |
| イベントキャッシュ | インメモリのみ（再起動時に再取得） |

---

## 開発フェーズ

### Phase 1：認証・接続確立（MVP 前提）
- [ ] Tauri v2 プロジェクト初期化
- [ ] tauri-plugin-oauth 導入
- [ ] Microsoft Graph OAuth PKCE フロー実装
- [ ] Google Calendar OAuth PKCE フロー実装
- [ ] iCloud CalDAV 接続確認
- [ ] keyring による認証情報保存・読み出し

### Phase 2：イベント取得・表示
- [ ] Graph API カレンダー一覧取得（`/me/calendars`）
- [ ] Graph API イベント取得（各カレンダーの `calendarView`）
- [ ] Google Calendar API カレンダー一覧取得（`calendarList`）
- [ ] Google Calendar API イベント取得（各カレンダーの `events.list`）
- [ ] CalDAV カレンダー一覧取得（`PROPFIND`）
- [ ] CalDAV REPORT クエリ実装（各カレンダー）
- [ ] UnifiedEvent への正規化・JST 変換
- [ ] 繰り返しイベントの展開（RRULE → 個別インスタンス）
- [ ] DayView・WeekView 実装
- [ ] カレンダーソース・サブカレンダー ON/OFF フィルタ

### Phase 3：イベント作成・編集・削除
- [ ] EventModal UI 実装（登録先カレンダー選択：ソース→サブカレンダーの2段階）
- [ ] 繰り返しイベント編集ダイアログ（「この1件」「以降すべて」「すべて」）
- [ ] 各ソースへの POST / PATCH / DELETE 実装
- [ ] 繰り返し編集スコープ別の API 呼び出し分岐
  - Graph：`thisAndFollowing` / `singleInstance` / `master` への PATCH
  - GCal：`?recurringEventId=` + `sendUpdates=none`
  - CalDAV：VEVENT の RECURRENCE-ID 付き PUT / EXDATE 追記
- [ ] バリデーション

### Phase 4：設定・仕上げ
- [ ] SettingsModal 実装
- [ ] カレンダー表示名・色カスタマイズ
- [ ] エラーハンドリング・Toast 実装
- [ ] 自動トークンリフレッシュ

---

## 未決事項（TODO）

- [x] 通知・リマインダー機能の要否 — **不要**（2026-05-13 決定）
- [x] ウィンドウサイズ・最小サイズの決定 — **初期 1280×800 / 最小 960×600**（2026-05-13 決定、`src-tauri/tauri.conf.json` 参照）
- [x] macOS / Windows 両対応の動作確認環境 — **両 OS とも実機で確認可能**（2026-05-13）

---

*作成日：2025-04-27*
*Author：osprey74*
