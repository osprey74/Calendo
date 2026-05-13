# HANDOFF.md — Calendo

**最終更新**: 2026-05-10
**バージョン**: v0.1.0（開発中）
**フェーズ**: **v0.1.0 公開完了**（2026-05-10）— https://github.com/osprey74/Calendo/releases/tag/v0.1.0 。次フェーズ以降の課題は本ドキュメント末尾「Phase 5 持ち越し」を参照

---

## プロジェクト概要

複数カレンダーアカウント（Microsoft 365 × 1・Google Workspace × 1・iCloud × 1）を統合し、
日次・週次のスケジュールを一覧表示・登録・編集できるデスクトップアプリ。

- **リポジトリ**: 未作成
- **開発者**: Sohshi / osprey74
- **設計書**: [DESIGN.md](DESIGN.md)（仕様の正本）
- **対象 OS**: Windows / macOS

---

## 技術スタック

| レイヤー | 採用技術 |
|---|---|
| アプリフレームワーク | Tauri v2 |
| フロントエンド | React + TypeScript + Vite |
| 状態管理 | Zustand |
| スタイリング | CSS Modules + CSS Variables |
| アイコン | Material Symbols Rounded（kazahana 準拠） |
| HTTP クライアント | reqwest（Rust） |
| CalDAV パーサ | quick-xml（Rust） |
| 認証情報保管 | keyring crate（OS Keychain / Credential Manager） |
| 永続設定 | Tauri store plugin（`~/.config/calendo/settings.json`） |
| テスト | Vitest（フロント）/ cargo test（Rust） |

---

## カレンダーソース構成

| ID | ラベル | プロトコル | デフォルト色 | 認証方式 |
|---|---|---|---|---|
| `ms365_work1` | 仕事 | Microsoft Graph | `#0582AF` | OAuth2 + PKCE |
| `google_gws` | Google カレンダー | Google Calendar API v3 | `#2E7D32` | OAuth2 + PKCE |
| `icloud` | プライベート | CalDAV | `#888780` | アプリ専用パスワード |

> 当初は Microsoft 365 を 2 アカウント統合する設計だったが、2 つ目のアカウントは Azure 管理者によるアプリ承認が必要なため、Phase 1 接続検証時点で 1 アカウント運用に変更（2026-04-27）。

---

## 🔴 着手前に必要な手動作業

### 1. OAuth クライアント登録 ✅ 取得済み

| サービス | 取得物 | 保管場所 |
|---------|--------|---------|
| Microsoft 365 | Application (client) ID | ローカル `.env`（未作成）+ GitHub Secrets `MS_CLIENT_ID` |
| Google Workspace | Client ID + Client Secret | ローカル `.env`（未作成）+ GitHub Secrets `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` |
| iCloud | Apple ID + アプリ専用パスワード | アプリ実行時に SettingsModal から入力 → keyring に保存 |

> ローカル開発用に `.env` を作成（`.env.example` をコピーして実値を記入）。`.env` は .gitignore 済。

### 2. GitHub Secrets 登録（CI/CD 用）

リリースビルド（タグプッシュ）時に Tauri アプリにクライアント ID／シークレットをコンパイル時注入するため、以下を **`osprey74/Calendo`** リポジトリの Secrets and variables → Actions に登録する:

| Secret 名 | 内容 |
|----------|------|
| `MS_CLIENT_ID` | Microsoft 365 Azure アプリの Application (client) ID |
| `GOOGLE_CLIENT_ID` | Google OAuth クライアント ID |
| `GOOGLE_CLIENT_SECRET` | Google OAuth クライアントシークレット |

登録コマンド例:

```bash
gh secret set MS_CLIENT_ID --repo osprey74/Calendo
gh secret set GOOGLE_CLIENT_ID --repo osprey74/Calendo
gh secret set GOOGLE_CLIENT_SECRET --repo osprey74/Calendo
```

### 3. 開発環境セットアップ（タスク化済 / Phase 1）

- Rust toolchain（stable）
- Bun または Node.js（Tauri 推奨は Node.js LTS / Vite との相性により）
- Tauri prerequisites（Windows: WebView2, MSVC build tools / macOS: Xcode CLT）

---

## 開発フェーズ

### Phase 0：事前準備

- [x] Microsoft Azure アプリ登録・Client ID 取得
- [x] Google Cloud OAuth クライアント ID 取得
- [x] iCloud アプリ専用パスワード発行
- [x] GitHub リポジトリ作成（`osprey74/Calendo` PUBLIC）
- [x] `.env.example` 設計（`MS_CLIENT_ID`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`）
- [x] `.gitignore` / `LICENSE`（MIT）/ `README.md` / `README.ja.md` 配置
- [x] CI/CD ワークフロー配置（`.github/workflows/release.yml` タグプッシュ自動ビルド + `ci.yml` PR テスト）
- [x] GitHub Secrets 登録（`MS_CLIENT_ID` / `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`）— 2026-04-27 登録済み
- [x] ローカル `.env` 作成（実値記入）— 2026-04-27 作成済み

### Phase 1：認証・接続確立（MVP 前提）

- [x] Tauri v2 プロジェクト初期化（`npm create tauri-app` / React + TypeScript + Vite テンプレート）
- [x] `tauri-plugin-oauth` 導入（コールバック受信用ローカルサーバ）
- [x] `tauri-plugin-store` 導入（設定永続化）
- [x] `keyring` crate 追加・OS Keychain 読み書きラッパー実装（[src-tauri/src/auth/keyring.rs](src-tauri/src/auth/keyring.rs)）
- [x] PKCE コード生成・トークン交換・リフレッシュ共通実装（[src-tauri/src/auth/oauth.rs](src-tauri/src/auth/oauth.rs)）
- [x] Microsoft Graph OAuth フロー実装（`auth_start("ms365_work1")`）
- [x] Google Calendar OAuth フロー実装（`auth_start("google_gws")`）
- [x] iCloud アプリ専用パスワード保存コマンド（`auth_icloud_save`）
- [x] iCloud CalDAV 接続確認（`PROPFIND /` で principal 到達確認、[src-tauri/src/auth/icloud.rs](src-tauri/src/auth/icloud.rs)）
- [x] 各ソースの動作確認用フロント実装（[src/App.tsx](src/App.tsx)：3 ソース連結カード + iCloud 入力フォーム）
- [x] `cargo check` / `npm run build` 通過
- [x] OAuth クライアント ID 注入用 [src-tauri/build.rs](src-tauri/build.rs)（`.env` 読み取り → `cargo:rustc-env=`）
- [x] **実機動作確認 (2026-04-27)**: 3 ソース（MS365 / Google / iCloud）すべて接続成功・カレンダー一覧取得確認

#### Phase 1 実装メモ

- OAuth Client ID／Secret は **コンパイル時 env 注入**（`option_env!`）。dev 時は `g:/dev/calendo/.env` に記入、CI は GitHub Secrets。
- `tauri-plugin-oauth` v2 の `start(handler)` は引数 1 つ（FnMut(String)）。redirect_uri は `http://localhost:<エフェメラルポート>` で生成。
- Azure / Google の OAuth 設定で **リダイレクト URI に `http://localhost`（パブリッククライアント）を登録**することを忘れないこと（Azure はパブリッククライアントフローを有効化、Google はデスクトップアプリ種別なら自動）。
- **MS Graph refresh token は Windows Credential Manager の 2560-byte 制限を超える** → `<source>.refresh.meta` + `<source>.refresh.{0..N}` のチャンク分割保存（[src-tauri/src/auth/keyring.rs](src-tauri/src/auth/keyring.rs)）。
- access_token は keyring に永続化せず、`OnceLock<Mutex<HashMap>>` のインメモリキャッシュのみ。再起動後の最初の API 呼び出しで refresh_token から再取得。
- iCloud 認証は `auth_icloud_save` で **保存前に PROPFIND で疎通確認**（401 なら拒否）。
- CalDAV のサブカレンダー列挙は `caldav.rs` で空リストを返す **スタブ状態**（principal 到達確認のみ）。完全な enumerate は Phase 2 で実装。
- 診断用 `auth_debug_clients` コマンドで注入されたクライアント ID をマスク表示（フロント上部の debug-bar）。

### Phase 2：イベント取得・表示

#### 2-A：カレンダー一覧取得

- [x] Graph API カレンダー一覧（`GET /me/calendars`）→ `CalendarMeta[]`（[src-tauri/src/calendars/graph.rs](src-tauri/src/calendars/graph.rs)）
- [x] Google Calendar `calendarList.list` → `CalendarMeta[]`（[src-tauri/src/calendars/gcal.rs](src-tauri/src/calendars/gcal.rs)）
- [x] CalDAV `PROPFIND` でカレンダーホーム → 各 `calendar` リソース列挙（[src-tauri/src/calendars/caldav.rs](src-tauri/src/calendars/caldav.rs)、principal → calendar-home-set → Depth:1 enumerate）

#### 2-B：イベント取得

- [x] Graph API `calendarView`（startDateTime / endDateTime）→ 繰り返し展開済みインスタンス取得（`Prefer: outlook.timezone="UTC"` で UTC 統一受信、ページネーション対応）
- [x] Google Calendar `events.list`（`singleEvents=true&orderBy=startTime`）→ 展開済みインスタンス（`nextPageToken` 対応、`status=cancelled` 除外）
- [x] CalDAV `REPORT` クエリ（`time-range` + `<C:expand>` でサーバ側展開）→ VEVENT 群
- [x] UnifiedEvent への正規化・JST 変換（[src-tauri/src/calendars/ical.rs](src-tauri/src/calendars/ical.rs)：TZID→JST 変換、UTC→JST 変換、終日イベント YYYY-MM-DD 形式）
- [x] 繰り返しイベント展開：Graph / GCal はサーバ側で展開済み、CalDAV は `<C:expand>` でサーバ側展開（自前 RRULE 展開は不要に）
- [x] `events_fetch` コマンド：複数ソース／複数カレンダーをまたいで集約（[src-tauri/src/commands/calendar_commands.rs](src-tauri/src/commands/calendar_commands.rs)）

#### 2-C：UI

- [x] AppShell（[src/components/layout/AppShell.tsx](src/components/layout/AppShell.tsx)：TopBar + Sidebar + Main の縦/横グリッド）
- [x] TopBar（[src/components/layout/TopBar.tsx](src/components/layout/TopBar.tsx)：日/週切替・前後ナビ・今日ボタン・設定モーダル起動）
- [x] CalendarSidebar（[src/components/sidebar/CalendarSidebar.tsx](src/components/sidebar/CalendarSidebar.tsx)：ソース ON/OFF・サブカレンダー ON/OFF・RO バッジ）
- [x] DayView（[src/components/views/DayView.tsx](src/components/views/DayView.tsx)：時間軸縦スクロール、レーン割当で重複表示）
- [x] WeekView（[src/components/views/WeekView.tsx](src/components/views/WeekView.tsx)：7 日横並び、日毎レーン割当、本日ハイライト）
- [x] AllDayBar（[src/components/events/AllDayBar.tsx](src/components/events/AllDayBar.tsx)：終日イベント帯）
- [x] EventBlock（[src/components/events/EventBlock.tsx](src/components/events/EventBlock.tsx)：時間ベースの絶対配置・レーン分割）
- [x] Zustand store（[src/store/calendarStore.ts](src/store/calendarStore.ts)：events/calendars/sourceEnabled/calendarEnabled/view/anchor）
- [x] フィルタ：ソース ON/OFF・サブカレンダー ON/OFF（クライアント側 visibleEvents セレクタ）
- [x] ConnectionPanel モーダル（[src/components/settings/ConnectionPanel.tsx](src/components/settings/ConnectionPanel.tsx)：接続状態表示・接続/切断・iCloud 入力フォーム）

#### Phase 2 実装メモ

- **CalDAV カレンダー列挙**: PROPFIND `current-user-principal` → PROPFIND `calendar-home-set` → PROPFIND Depth:1 で `<calendar/>` リソースタイプを持つコレクションを抽出。`displayname` / `cs:calendar-color` / `current-user-privilege-set` / `supported-calendar-component-set` を一括取得。
- **CalDAV イベント取得**: `<C:expand>` を `calendar-query` REPORT に含めることで、サーバ側で繰り返しを展開させる（クライアント側 RRULE 展開不要）。`time-range` フィルタは UTC 形式 `YYYYMMDDTHHMMSSZ`。
- **タイムゾーン**: 内部表現は JST（ISO 8601 + `+09:00`）に統一。Graph は `Prefer: outlook.timezone="UTC"` で UTC 受信→JST 変換。GCal は元々 RFC3339 オフセット付き→JST 変換。CalDAV は UTC（Z 終端）／TZID 付き／フローティングを `chrono-tz` で JST 変換（フローティングは JST と仮定）。
- **iCalendar パーサ**: [src-tauri/src/calendars/ical.rs](src-tauri/src/calendars/ical.rs) で line-folding（CRLF + 単一 SP/HTAB の RFC 5545 仕様）、TZID パラメータ、`VALUE=DATE`（終日）、エスケープ（`\,` `\;` `\n`）を処理。`<C:expand>` 前提のため RRULE 展開は未実装。
- **XML パーサ**: [src-tauri/src/calendars/xmlnode.rs](src-tauri/src/calendars/xmlnode.rs) で quick-xml の Event ストリーミングをツリー化、ローカル名一致で検索（namespace prefix を除去）。
- **イベント描画**: 1440 分高さの絶対配置グリッド。日内重複は greedy lane assignment で横並び。週ビューは 7 列で同様のレイアウトを各日に適用。
- **状態管理**: `useCalendarStore` 単一 Zustand ストア。`view`/`anchor` 変更時に `loadEvents()` を自動実行。`sourceEnabled`/`calendarEnabled` でクライアント側フィルタ（バックエンドリクエストにも反映）。永続化は Phase 4 で実装予定。
- **未実装/Phase 3 以降**: イベント作成・編集・削除（EventModal、繰り返し編集スコープ）、SettingsModal の表示設定（色・ラベルカスタマイズ）、Tauri store plugin による設定永続化、自動トークンリフレッシュ（401→refresh→retry）、Toast 通知、CalDAV PUT/DELETE。
- **依存追加**: `chrono-tz` v0.10（CalDAV TZID パラメータ→JST 変換用、Graph の名前付きタイムゾーンの予備実装にも使用）。

##### 2026-05-06 動作確認結果

- [x] MS365 イベントが日次/週次ビューに表示されること
- [x] Google Calendar イベントが日次/週次ビューに表示されること
- [x] iCloud カレンダー一覧（サブカレンダー含む）が Sidebar に表示されること
- [x] iCloud イベント（繰り返し含む）が日次/週次ビューに展開表示されること
- [x] サブカレンダー ON/OFF トグルがイベント表示に反映されること
- [x] 日/週切替・前後ナビゲーション・「今日」ボタンが期待通り動作すること

##### 2026-05-06 動作確認で発覚した問題と対応

実機テスト中に判明した問題を順次修正：

- **React 19 + Zustand v5 のセレクタ不安定によるレンダーループ** — `useCalendarStore(visibleEvents)` がフィルタ結果の新しい配列を毎回返すため `useSyncExternalStore` で snapshot 不安定→`Maximum update depth exceeded`→白画面。生 state を別個に取得→`useMemo` でフィルタする形に修正（[DayView.tsx](src/components/views/DayView.tsx) / [WeekView.tsx](src/components/views/WeekView.tsx)）
- **`events_fetch` のソース横断汚染** — フラットな `calendar_ids` を全ソースに iterate していたため、iCloud の URL を MS Graph に投げ込み reqwest URL ビルダー破綻（`builder error`）。API を per-source map (`HashMap<String, Vec<String>>`) に変更（[calendar_commands.rs](src-tauri/src/commands/calendar_commands.rs) / [calendarStore.ts](src/store/calendarStore.ts)）
- **MS Graph `/me/calendars` のページネーション未対応** — デフォルト 10 件で取りこぼし。`$top=100` + `@odata.nextLink` フォロー対応（[graph.rs](src-tauri/src/calendars/graph.rs)）
- **カレンダー ID の URL エンコード欠落** — メールアドレスを ID として返すカレンダーで `@` が生のまま URL に入り 400。共通の `percent_encode_segment` を [util.rs](src-tauri/src/calendars/util.rs) に追加し Graph / GCal で利用
- **per-calendar 4xx で全体停止** — 1 件のカレンダー失敗で全ソースのイベント取得が止まっていた。`is_recoverable_per_calendar()` で 4xx / CalDAV エラーを許容しスキップ継続
- **終日イベントの翌日表示** — RFC 5545 / Graph / GCal すべて DTEND 排他的なのに `>=` で比較していたため 5/5 のイベントが 5/6 にも重複表示。終日のみ `>` 厳密比較に変更（[eventUtils.ts](src/utils/eventUtils.ts)）
- **週ビューのヘッダー列ズレ** — 縦スクロールバー幅分だけ時間グリッドが狭くなり、外側のヘッダーと食い違っていた。ヘッダー＋終日帯を `time-grid-scroll` 内に移動し `position: sticky; top: 0` で同一コンテナ管理に統一（[TimeGrid.css](src/components/views/TimeGrid.css)）

##### 2026-05-06 追加した UX 改善

- **イベントクリックで詳細モーダル** — どのカレンダーの予定か一目で分かるよう `EventDetailsModal` を追加。ソース・カレンダー名（+ primary / RO バッジ）・calendar_id・繰り返し情報・event_id まで表示（[EventDetailsModal.tsx](src/components/events/EventDetailsModal.tsx)）
- **イベントブロックにカレンダー名タグ** — 各イベントカードの 3 行目に所属カレンダー名を常時表示。Tooltip にも「ソース / カレンダー名」を含める
- **サイドバーのスクロール可視化と sticky ヘッダー** — WebView2 で薄いスクロールバーが視認できない問題に対応し常時 8px 表示。ソースヘッダーを sticky 化
- **全 ON / 全 OFF クイック切替 + カレンダー件数バッジ** — ソースごとに一括切替ボタンと件数表示を追加（[CalendarSidebar.tsx](src/components/sidebar/CalendarSidebar.tsx) / [calendarStore.ts](src/store/calendarStore.ts) `setAllCalendarsEnabled`）
- **非表示カレンダーの折りたたみ** — ON/OFF を切った瞬間に「非表示中 N 件」セクションへ移動。クリックで展開し再 ON 可能。取り消し線 + 透過で見た目区別

### Phase 3：イベント作成・編集・削除

#### Phase 3.0：基本 CRUD（実装済み）

- [x] EventModal UI（タイトル・日時・終日・場所・メモ・登録先カレンダー2段階選択）
- [x] バリデーション（タイトル必須・終了 > 開始・書き込み可能なカレンダー必須）
- [x] Graph 書き込み実装（`POST /me/calendars/{id}/events` / `PATCH /me/events/{id}` / `DELETE /me/events/{id}`、`Prefer: outlook.timezone="UTC"` 付き）
- [x] Google Calendar 書き込み実装（`POST/PATCH/DELETE /calendars/{calendarId}/events[/{eventId}]`、`sendUpdates=none` 付き）
- [x] CalDAV 書き込み実装（VEVENT 生成・`PUT` で create（`If-None-Match: *`）、existing UID 保持で update、`DELETE` で削除）
- [x] イベント詳細モーダルから「編集」「削除」ボタンで起動。新規は TopBar の `+ 新規` ボタンから
- [x] 全イベント書き込みパスで作成/更新後に `loadEvents()` を呼んで現在表示窓を再フェッチ

#### Phase 3.0 実装メモ

- **DTEND 変換**: フォーム入力は inclusive（ユーザーが指定した最終日）。バックエンドが各 provider の API に投げる時に exclusive（次日）へ変換（`Graph` / `GCal` `date` フィールド / CalDAV `DTEND;VALUE=DATE`）
- **タイムゾーン**: 終日以外は JST RFC3339（`+09:00`）を生成。Graph では `timeZone: "Asia/Tokyo"` パラメータ + 19文字の naive datetime に変換。GCal はそのまま渡す。CalDAV は UTC（`Z` 付き）に変換して `VTIMEZONE` ブロックを省略
- **CalDAV id の意味変更**: 以前は iCalendar UID ベースだったが、Phase 3 では **`.ics` リソース URL** をカノニカル ID に。展開済み繰り返しインスタンスは `<resource_url>::<recurrence_id>` で discriminate（READ 時に注意）。書き込み系は常に URL 部分だけを使う
- **CalDAV 更新の UID 保持**: PUT する前に GET で既存 .ics を読み、`UID:` 行を抽出して同じ UID で書き戻す（UID 変更すると iCloud がイベント重複を作る恐れがあるため）。GET 失敗時は新 UID 発行→新規作成扱いにフォールバック
- **書き込み可否ガード**: 詳細モーダルの編集/削除ボタンは `calendar.isWritable` が真かつ「iCloud かつ繰り返し」でない場合のみ有効。書き込み不可の理由をボタン横にヒント表示
- **EventModal 編集モード**: ソース／カレンダーは固定（移動は未対応 — 削除＋再作成で対応）

#### Phase 4 に先送り

- [ ] 繰り返しイベント編集ダイアログ（「この1件のみ」「以降すべて」「すべて」）
- [ ] Graph 繰り返しスコープ別の API 呼び出し分岐（`thisAndFollowing` / `singleInstance` / `master`）
- [ ] Google Calendar 繰り返し系（`recurringEventId` 連動）
- [ ] CalDAV 繰り返し: `RECURRENCE-ID` 付き VEVENT 部分上書き、`EXDATE` 追記による単一インスタンス削除
- [ ] イベントのソース／カレンダー間移動（現状: 削除＋再作成）

### Phase 4：設定・仕上げ

#### Phase 4.0（実装済み）

- [x] アプリアイコン配置（[src-tauri/icons/](src-tauri/icons/) — `npx tauri icon` で 1024x1024 から全プラットフォーム向けサイズを生成）
- [x] Toast 実装（成功・エラー通知 — [ToastHost.tsx](src/components/toast/ToastHost.tsx) + [toastStore.ts](src/store/toastStore.ts)）
  - create / update / delete の結果を `toast.success` / `toast.error` で表示
  - エラーは 6 秒、成功・情報は 4 秒で自動消去
- [x] Tauri store plugin で設定永続化（[src/lib/persistence.ts](src/lib/persistence.ts)）
  - 永続化対象: `view`（日/週）、`sourceEnabled`（ソース ON/OFF）、`calendarEnabled`（サブカレンダー ON/OFF）
  - 起動時に `hydrate()` でロード→`loadEvents()` の順で適用
  - 保存ファイル: `<app config>/settings.json`（Windows: `%APPDATA%\com.osprey74.calendo\`、macOS: `~/Library/Application Support/com.osprey74.calendo/`）
- [x] ConnectionPanel: 既存実装で接続管理は既に網羅されていたため Phase 4.0 では再利用（接続状態・接続/切断ボタン・iCloud 入力フォーム・OAuth 診断バー）
- [x] 初回起動時オンボーディング（[AppShell.tsx](src/components/layout/AppShell.tsx)）
  - すべてのソースのカレンダーが空 / 未取得かつイベント 0 件のとき、メイン領域中央に「ようこそ」カードを表示
  - 「設定を開く」ボタンで ConnectionPanel を直接起動

#### Phase 4.0 実装メモ

- **永続化のタイミング**: `setView` / `toggleSource` / `toggleCalendar` / `setAllCalendarsEnabled` のミューテーションごとに `void saveXxx(...)` で fire-and-forget 保存。ストアは即座に新しい状態を返し、IO は背景で進行
- **`hydrate()` 設計**: ストア初期状態（全ソース ON / view: "week"）が defaults。永続化された値があればその上にマージ。`AppShell` の最初の `useEffect` で `hydrate()` を `await` してから `loadEvents()` を呼び、ハイドレーション前のフィルタが反映されない一発目フェッチを回避
- **Toast の使い方**: `toast.success(msg)` / `toast.error(msg)` / `toast.info(msg)` の関数 API。`useToastStore.getState().show(...)` のショートハンド。React 外（store action 等）からも呼べる
- **Onboarding 検出**: `loading=false && events.length===0 && すべてのソース calendars が null か []`。一旦接続して再起動した状態（calendars が取得済み・events 0 件）では出ない仕様
- **ConnectionPanel の SettingsModal 化**: HANDOFF 当初想定では SettingsModal にカレンダー表示設定（色・ラベル上書き）を統合する案だったが、Phase 4.0 では既存 ConnectionPanel をそのまま流用。色・ラベルのカスタマイズは Phase 5+ に先送り

#### Phase 4.1（実装済み）

- [x] 自動トークンリフレッシュ（401 → refresh → retry）— [oauth.rs](src-tauri/src/auth/oauth.rs) `send_with_refresh()` 共通化
  - Graph / GCal の全認証付きリクエスト（fetch_calendars / fetch_events / create_event / update_event / delete_event）が 401 受信時に自動リフレッシュ＋リトライ
  - サーバ側クロックスキュー・トークン revoke 直後・refresh 直後の旧トークン使用ケース等に対応
- [x] 繰り返しイベント編集スコープ（「この1件のみ」「すべて」）— [EventModal.tsx](src/components/events/EventModal.tsx) / [EventDetailsModal.tsx](src/components/events/EventDetailsModal.tsx)
  - **Graph / GCal**: per-instance id（イベントクリックで取れた id）と series master id（`recurringEventId`）をフロントで切り替えて backend に渡すことで API 標準のスコープ動作を実現
  - **CalDAV**: 「すべて」スコープのみサポート（書き込みがリソース単位のため。「この1件のみ」は無効化＋ヒント表示）
  - 編集: EventModal 内に「編集範囲」ラジオを表示、デフォルトは "this"（ユーザーがクリックしたインスタンスを意図する想定）
  - 削除: EventDetailsModal で削除ボタン → インライン確認ダイアログに切替、繰り返しイベントなら「この1件のみ削除 / すべて削除 / キャンセル」のボタン

#### Phase 4.1 実装メモ

- **scope → targetId 解決はフロント側**: backend は `event_id` パラメータで指定されたものを操作するだけ。frontend が `scope=="all"` の場合 `event.recurringEventId` をターゲットに切り替える。`recurringScope` フィールドは現状 backend では情報用（CalDAV "this" や thisAndFollowing 実装時に使う想定）
- **401 リトライの closure パターン**: `Fn(&str) -> RequestBuilder` で再構築。RequestBuilder が Clone でないため二度ビルド。`.json(&payload)` のシリアライズが各呼び出しで走るが小ペイロードなので実害なし
- **CalDAV recurring の "すべて" 動作**: 既存の `caldav_resource_url(event_id)` が `<url>::<recurrence-id>` の `::` 以降を strip してマスタ resource URL を返すため、そのまま PUT/DELETE すれば全インスタンス対象になる

#### Phase 4.2（実装済み）

- [x] 繰り返し予定の作成 UI — [EventModal.tsx](src/components/events/EventModal.tsx) にプリセット dropdown 追加
  - **プリセット**: なし / 毎日 / 毎週（開始日の曜日） / 平日のみ（月〜金） / 毎月（開始日の日） / 毎年（開始日の月日）
  - **任意の終了日**（UNTIL）も同フィールド内で指定可能
  - フロントが RFC 5545 RRULE 文字列を生成 → 各 provider 向けに backend が変換
- [x] **Graph**: RRULE → JSON `recurrence` オブジェクト変換（[graph.rs](src-tauri/src/calendars/graph.rs) `build_graph_recurrence`）
  - daily / weekly+BYDAY / absoluteMonthly / absoluteYearly に対応
  - `range` は `noEnd` または `endDate`（UNTIL 指定時）
- [x] **GCal**: `recurrence: ["RRULE:..."]` 配列で送信（[gcal.rs](src-tauri/src/calendars/gcal.rs)）
- [x] **CalDAV**: VEVENT に `RRULE:` 行を出力（[caldav.rs](src-tauri/src/calendars/caldav.rs) `build_vcalendar`）
- [x] CalDAV update 時の RRULE 保持 — `extract_rrule()` で既存 .ics から RRULE を読み出し、draft が空なら引き継ぐ。ないと既存の繰り返しが update のたびに消える

#### Phase 4.2 実装メモ

- **編集時の RRULE は不変**: create フォームのみピッカーを表示。edit モードでは既存ルールを表示するだけ（変更不可）。Graph/GCal は PATCH に `recurrence` を含めなければ既存ルールを保持。CalDAV は明示的に extract→引き継ぎ
- **UNTIL の正規化**: 終日イベントは `YYYYMMDD`、時刻ありは `YYYYMMDDT235959Z`（UTC 終日）でフォームから生成。RFC 5545 上、時刻ありイベントの UNTIL は UTC date-time が要求されるため
- **プリセットの「日本式」表現**: ユーザーが見るのは「毎週 月曜日」「毎月 13日」のような日本語ラベル。内部の RRULE は `FREQ=WEEKLY;BYDAY=MO` のような英語標準仕様
- **edit 時の表示**: 現在の RRULE を文字列で表示するだけのリードオンリー UI。Phase 5+ で RRULE → プリセット逆引き＋カスタムエディタを実装予定

#### Phase 5 持ち越し

- [ ] 編集モードでの RRULE 変更 UI（プリセット逆引き＋カスタム RRULE エディタ）
- [ ] 「この日以降すべて」スコープ — Graph/GCal で単一 API 呼び出し不可。マスタ recurrence range 終端付け替え＋新シリーズ作成の複合操作が必要
- [ ] CalDAV 繰り返しの「この1件のみ」: `RECURRENCE-ID` 付き VEVENT 部分上書き、`EXDATE` 追記による単一インスタンス削除
- [x] エラーハンドリング全網羅（DESIGN.md「エラーハンドリング方針」表）— 2026-05-13 実装
- [x] SettingsModal でサブカレンダーの色・ラベル上書き（カスタマイズ） — 2026-05-13 実装
- [ ] イベントのソース／カレンダー間移動（現状: 削除＋再作成）

#### Phase 5.1 実装メモ — 色・ラベル上書き（2026-05-13）

- **永続化**: `calendarOverrides: Record<\`${sourceId}|${calendarId}\`, { color?, label? }>` を Tauri store の `calendarOverrides` キーに保存（[persistence.ts](src/lib/persistence.ts) `saveCalendarOverrides`）。空オブジェクトはストアから削除し、`hasOverride` 検出を正確に保つ
- **適用**: 描画時のセレクタで合成。`effectiveCalendarColor()` / `effectiveCalendarName()` を [calendarStore.ts](src/store/calendarStore.ts) に追加し、CalendarSidebar / EventBlock / EventDetailsModal で provider 値の代わりに使用。CalendarMeta 自体は不変（provider 値を保持）
- **UI**: 既存 ConnectionPanel をタブ化。新規 [SettingsModal.tsx](src/components/settings/SettingsModal.tsx) が「アカウント接続」「表示設定」の 2 タブを切り替え、ConnectionPanelContent / DisplayPanelContent を埋め込む。ラベル入力は local state にバッファし blur / Enter で commit（毎打鍵で永続化しないため打鍵レスポンスが軽快）
- **リセット**: 行ごとの「リセット」ボタンで両フィールドを削除し provider 値に戻す。color 単独・label 単独の片方リセットは Phase 5 では未対応（UX 上の複雑度を回避）

#### Phase 5.3 実装メモ — 時間軸ズーム（2026-05-13）

- **ズームレベル**: 1時間あたりの px を 8 段階のプリセット `[40, 60, 80, 100, 120, 160, 200, 240]`（60 が既定 = 旧固定値）。120 を境にステップが粗くなり、最大 240px まで到達。任意値で保存されても起動時に最近接プリセットへスナップ
- **状態**: [calendarStore.ts](src/store/calendarStore.ts) に `hourHeightPx` を追加、`setHourHeightPx(px)` / `stepHourHeight(±1)` を公開。Tauri store の `hourHeightPx` キーに永続化（[persistence.ts](src/lib/persistence.ts) `saveHourHeightPx`）
- **適用**: [TimeGrid.css](src/components/views/TimeGrid.css) の `.time-grid` を `calc(var(--hour-px, 60px) * 24)` 化。DayView / WeekView の `.time-grid-scroll` に `style={{ "--hour-px": "{n}px" }}` を inline 注入（動的 CSS 変数のため inline 必須）。HourGutter / HourLines / EventBlock は既に絶対位置・パーセンテージ計算なので追加変更不要
- **UI**: [TopBar.tsx](src/components/layout/TopBar.tsx) 右側に `−` / `60px` / `＋` のズームコントロールを追加。両端で disabled。`role="group"` ＋ `aria-label="時間軸の高さ"`
- **既定値リセット**: 中央の px 表示がボタンになっており、クリックで `DEFAULT_HOUR_HEIGHT_PX`（60）に戻る。既定値のときは disabled。`title` に「クリックで既定（60px）に戻す」を表示

#### Phase 5.4 実装メモ — 表示時間範囲（2026-05-13）

- **状態**: `viewStartHour` (0-23, 既定 0) / `viewEndHour` (1-24, 既定 24) を [calendarStore.ts](src/store/calendarStore.ts) に追加。`setViewHours(start, end)` で同時更新（inverted/empty な値は silently 棄却）。Tauri store の `viewStartHour` / `viewEndHour` に永続化（[persistence.ts](src/lib/persistence.ts) `saveViewHours`）
- **レイアウト計算**: [eventUtils.ts](src/utils/eventUtils.ts) を内部関数 `eventDayMinutes` と公開関数 `dayBlockLayout(e, day, visStart, visEnd)` に分離。dayBlockLayout は ウィンドウに対する％を返し、ウィンドウ外イベントは端でクリップ。`partitionDay(events, day, visStart, visEnd)` がウィンドウに重ならない timed イベントを事前除外して `assignLanes` に渡すため、非表示イベントが可視イベントのレーン幅を奪う問題を回避
- **描画**: [HourGutter / HourLines](src/components/views/DayView.tsx) に `startHour` / `endHour` props を追加し、可視時間帯ぶんだけ繰り返し。DayView / WeekView から store の値を渡す。EventBlock も同 props を受け取り、layout クリップを実施
- **CSS**: `--visible-hours` を `.time-grid-scroll` に inline 注入（DayView / WeekView）。`.time-grid` 高さは [TimeGrid.css](src/components/views/TimeGrid.css) で `calc(var(--hour-px) * 24)` のまま（HourGutter / HourLines が flex:1 で要素数に応じて分割するため、要素数を可視時間数にすれば 1 時間あたりの実 px は `var(--hour-px) * 24 / visibleHours` になる）。※ 将来 1 時間 = 厳密に hourHeightPx にしたい場合は `--visible-hours` を用いて高さ計算式を変える余地あり
- **UI**: [DisplayPanel.tsx](src/components/settings/DisplayPanel.tsx) に `TimeRangeSection` を追加。開始 (0:00-23:00) / 終了 (1:00-24:00) のドロップダウン＋リセットボタン。終了値が開始以下になる選択時は自動的に start+1 / end-1 に補正（inverted を UI で発生させない）。設定モーダル「表示設定」タブの先頭に配置
- **終日イベント**: 表示範囲を狭めても影響を受けない（all-day bar は別レイアウト）。timed イベントのみがクリップ対象

#### Phase 5.5 実装メモ — 現在時刻インジケーター（2026-05-13）

- **コンポーネント**: 新規 [NowLine.tsx](src/components/views/NowLine.tsx)。`visibleStartHour` / `visibleEndHour` / `todayColumnIndex` / `columnCount` を受け取り、現在時刻が可視ウィンドウ内かつ今日が表示範囲内のときに限り赤い水平線と小さな丸印を描画。`setInterval(30s)` で再描画
- **配置**: DayView では `.day-column` の中（columnCount=1）、WeekView では `.week-columns` の中（columnCount=7）に直接マウント。どちらも `position: relative` なので `left: 0; right: 0` で親いっぱいに広がる。WeekView では HourGutter は別兄弟なので、線はガター以外の 7 日カラム横断
- **位置計算**: `nowMin = h*60 + m + s/60`（秒も使い視覚ジッタを抑制）。`topPct = ((nowMin - visStart) / visSpan) * 100`。ドットは `todayColumnIndex / columnCount` × 100% に絶対位置（`transform: translate(-50%, -50%)` で中央寄せ）
- **CSS**: [TimeGrid.css](src/components/views/TimeGrid.css) に `.now-line`（赤 2px ライン、z-index:2）と `.now-dot`（赤 10px 丸、z-index:3、`box-shadow: 0 0 0 1px surface` でイベントブロック上でも視認性確保）
- **非表示条件**: ① 今日が表示中の週／日に含まれない（`todayColumnIndex < 0`） ② 現在時刻が `[visibleStartHour, visibleEndHour)` の外

#### Phase 5.2 実装メモ — エラーハンドリング全網羅（2026-05-13）

- **Backend AppError 構造化**: [error.rs](src-tauri/src/error.rs) `Serialize` を `{kind, message, status?, sourceId?}` 形式に変更。`kind()` / `status()` / `source_id()` のヘルパーで各バリアントを分類。安定した kind 文字列（`auth_required` / `network` / `permission` / `not_found` / `conflict` / `rate_limit` / `server` / ...）をフロントの switch 文と契約として固定
- **HTTP 分類**: 新規バリアント `HttpStatus { status, message }` を導入し、CalDAV の各書き込み・読み込みパス（[caldav.rs](src-tauri/src/calendars/caldav.rs)）で個別 401 を `AuthRequired("icloud")` に切り出し、それ以外の 4xx/5xx を `HttpStatus` に分類。Graph/GCal は既存の `error_for_status()` 経由で `AppError::Http(reqwest)` のまま流れるが、`kind()` が reqwest::Error の status を見て同じ kind 文字列を返す
- **トークンリフレッシュの再失敗**: [oauth.rs](src-tauri/src/auth/oauth.rs) `send_with_refresh` が refresh 後の 2 度目も 401 だった場合に `AppError::AuthRequired(source_id)` を返す（refresh は succeed したが新トークンも reject = 再認証が必要）。サーバ側 token revoke 後・アプリ側で `token_for_user` を強制無効化したケース等に対応
- **events_fetch warnings**: 戻り型を `EventsFetchResult { events, warnings: Vec<FetchWarning> }` に変更（[calendar_commands.rs](src-tauri/src/commands/calendar_commands.rs) / [models.rs](src-tauri/src/models.rs)）。per-source / per-calendar のスキップ事由（auth_required / 4xx / caldav パース失敗）を `{sourceId, calendarId?, kind, message}` として集約し、フロントが Toast にまとめて表示。`is_disconnected_source` は `AuthRequired` も含めるので個別ソースの認証切れがあっても他ソースのフェッチを妨げない
- **Frontend classifier**: 新規 [lib/errors.ts](src/lib/errors.ts) の `classifyError(unknown): ClassifiedError` が Tauri の reject ペイロード（structured / 文字列 / Error）を吸収して `{kind, status?, sourceId?, userMessage}` を返す。`userMessage` は日本語ローカライズ済みでそのまま Toast / モーダル表示可能。`isAuthRequired()` も公開
- **catch サイトの全置換**: `String(e)` パターンを全 6 箇所で classifyError 経由に置換：[calendarStore.ts](src/store/calendarStore.ts)（loadCalendars / loadEvents / createEvent / updateEvent / deleteEvent）、[ConnectionPanel.tsx](src/components/settings/ConnectionPanel.tsx)（status / connect / disconnect）、[EventModal.tsx](src/components/events/EventModal.tsx)、[EventDetailsModal.tsx](src/components/events/EventDetailsModal.tsx)
- **UX 動作**:
  - **トークン期限切れ**: `send_with_refresh` 自動リトライ→失敗時に `auth_required` Toast「{ソース}：認証期限が切れました。設定から再ログインしてください。」
  - **ネットワークエラー**: 既存の events 配列を保持したまま（cache clobber しない）、Toast に「ネットワークエラーが発生しました。接続を確認して再試行してください。」
  - **CalDAV 認証失敗**: 401 → `AuthRequired("icloud")` → Toast。設定モーダルから再入力可能
  - **イベント作成失敗**: モーダルを閉じず入力保持。`error` フィールドに日本語メッセージ
  - **削除時の 404**: 他のクライアントで既に削除済みのケースは「対象は既に削除されていました」を info Toast で表示し成功扱い（ユーザの意図は満たされている）
  - **fetch 警告集約**: 同一 source+kind の warnings は 1 Toast に dedup。`not_authenticated`（未接続ソース）は静かに無視（毎回 Toast が出るのを回避）

### Phase 5：リリース準備

- [x] バージョン v0.1.0 確定 → `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` すべて `0.1.0`
- [x] `Cargo.lock` 再生成（`cargo generate-lockfile`、2026-05-10）
- [x] README.md / README.ja.md を v0.1.0 リリース向けに書き直し（機能リスト・OAuth セットアップ・開発／ビルド手順）
- [x] CI/CD（GitHub Actions: Windows x86_64 + macOS universal、タグプッシュ自動ビルド）— `.github/workflows/release.yml` 配置済み
- [x] リリースノート（EN/JA）— [RELEASE_NOTES_v0.1.0.md](RELEASE_NOTES_v0.1.0.md)
- [x] コミット＆プッシュ（メインブランチ）— `e26a4b7`（Phase 5 prep）+ `e08957f`（Tauri NPM bump）
- [x] `v0.1.0` タグ作成・プッシュ → CI/CD で Windows / macOS バイナリ自動ビルド＆ Release ドラフト生成（Run `25623864547`、8m25s）
- [x] GitHub Release ページ用リリースノート（EN/JA）を生成 → ドラフトに添付して公開（2026-05-10 08:39 UTC）

#### Phase 5 実装メモ

- **初回ビルド失敗と再起動**: 1 回目のタグプッシュで Tauri バージョン不整合（NPM `@tauri-apps/api@2.10.1` vs Rust `tauri@2.11.1`）によりビルド失敗。`@tauri-apps/api`/`cli`/`plugin-store` を `^2.11` に明示ピン → `package-lock.json` 再生成 → `v0.1.0` タグを削除・再プッシュで解決
- **配布アセット**: Windows NSIS (`x64-setup.exe`) / Windows MSI (`x64_en-US.msi`) / macOS DMG (`universal.dmg`) / macOS app tarball (`universal.app.tar.gz`) の 4 種
- **GitHub Release ページのリリースノート**: `RELEASE_NOTES_v0.1.0.md`（リポジトリ内のフル版）と Release ページ本文（コンパクト版＋ダウンロードテーブル）を分離。Release ページ本文は EN→JA 順、ダウンロード一覧と既知の制限を強調

---

## ディレクトリ構成（予定）

```
calendo/
├── CLAUDE.md                  ← 未作成（Phase 1 で起票）
├── DESIGN.md                  ← 仕様の正本
├── HANDOFF.md                 ← このファイル
├── README.md / README.ja.md   ← Phase 5 で作成
├── package.json
├── src/                       ← フロント
│   ├── components/{layout,sidebar,views,events,settings}/
│   ├── hooks/
│   ├── store/
│   ├── utils/
│   ├── types/
│   ├── App.tsx
│   └── main.tsx
└── src-tauri/                 ← Rust バックエンド
    ├── src/
    │   ├── lib.rs
    │   ├── models.rs
    │   ├── auth/{mod,oauth,keyring,icloud}.rs
    │   ├── calendars/{mod,graph,gcal,caldav}.rs
    │   └── commands/{mod,auth_commands,calendar_commands}.rs
    ├── Cargo.toml
    └── tauri.conf.json
```

---

## Tauri コマンド一覧（実装予定）

### 認証系
- `auth_start(source_id)` — OAuth ブラウザ起動 → コールバック → トークン保存
- `auth_refresh(source_id)` — トークン期限確認 + リフレッシュ
- `auth_revoke(source_id)` — 認証情報削除（ログアウト）
- `auth_icloud_save(apple_id, app_password)` — iCloud 認証情報保存

### カレンダー操作系
- `calendars_fetch(source_id)` → `Vec<CalendarMeta>`
- `events_fetch(source_ids, calendar_ids?, date_from, date_to)` → `Vec<UnifiedEvent>`
- `event_create(source_id, calendar_id, draft)` → `UnifiedEvent`
- `event_update(source_id, event_id, request)` → `UnifiedEvent`（`recurringScope` 対応）
- `event_delete(source_id, event_id, recurring_scope?)` → `()`

---

## 永続化マップ

| データ | 保存場所 | キー例 |
|---|---|---|
| OAuth アクセス・リフレッシュトークン | OS Keychain | `calendo/ms365_work1/access_token` 等 |
| iCloud アプリ専用パスワード | OS Keychain | `calendo/icloud/app_password` |
| カレンダー表示設定（ソース・サブカレンダー ON/OFF・色・ラベル） | Tauri store plugin | `~/.config/calendo/settings.json` |
| イベントキャッシュ | インメモリ（Zustand） | 再起動時に再取得 |

---

## 注意点・既知の検討事項

### 繰り返しイベントの差分実装

| プロトコル | 展開担当 | 編集スコープ表現 |
|---|---|---|
| Microsoft Graph | サーバ（`calendarView`） | `thisAndFollowing` / `singleInstance` / `master` |
| Google Calendar | サーバ（`singleEvents=true`） | `recurringEventId` + 編集対象インスタンスの ID |
| CalDAV | クライアント（自前 RRULE 展開） | `RECURRENCE-ID` 付き VEVENT / `EXDATE` 追記 |

→ CalDAV のみ RRULE パーサが必要。`rrule` crate（Rust）採用候補。

### タイムゾーン

- 内部表現はすべて JST（ISO 8601 + `+09:00`）に統一
- Graph / GCal は ISO 8601（UTC または TZID）で受信 → JST 変換
- CalDAV は VEVENT の `DTSTART;TZID=...` を明示的に解釈

### OAuth コールバック

- `tauri-plugin-oauth` がエフェメラルポートで HTTP サーバ起動 → リダイレクト URI
- Azure / Google 側のリダイレクト URI 登録は `http://localhost` のワイルドカード（ポート可変）として登録

---

## 未決事項（DESIGN.md より転記）

- [x] 通知・リマインダー機能の要否 — **不要**（2026-05-13 決定）
- [x] ウィンドウサイズ・最小サイズの決定 — **1280×800**（2026-05-13 決定）
- [x] macOS / Windows 両対応の動作確認環境 — **両 OS とも実機で確認可能**（2026-05-13）
- [x] CLAUDE.md の起票 — Phase 1 時点で配置済み（[CLAUDE.md](CLAUDE.md)）
- [x] アイコン素材の準備 — Phase 4.0 で配置済み（[src-tauri/icons/](src-tauri/icons/)）

---

*作成日：2026-04-27*
*Author：osprey74*
