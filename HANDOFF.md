# HANDOFF.md — Calendo

**最終更新**: 2026-05-06
**バージョン**: v0.1.0（開発中）
**フェーズ**: Phase 4.0（仕上げ：永続化・トースト・オンボーディング・アイコン）— 実装完了、実機動作確認待ち。繰り返しスコープ別編集と 401 自動リトライは Phase 4.x に持ち越し

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
- [ ] GitHub Secrets 登録（`MS_CLIENT_ID` / `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`）
- [ ] ローカル `.env` 作成（実値記入）

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

#### Phase 4.x 持ち越し

- [ ] 繰り返しイベント編集ダイアログ（「この1件のみ」「以降すべて」「すべて」）
- [ ] Graph 繰り返しスコープ別の API 呼び出し分岐（`thisAndFollowing` / `singleInstance` / `master`）
- [ ] Google Calendar 繰り返し系（`recurringEventId` 連動）
- [ ] CalDAV 繰り返し: `RECURRENCE-ID` 付き VEVENT 部分上書き、`EXDATE` 追記による単一インスタンス削除
- [ ] 自動トークンリフレッシュ（401 検出 → リフレッシュ → リトライ）— 現状 `ensure_fresh` の事前チェックで 95% カバー、サーバ側のクロックスキューや revoke は再認証が必要
- [ ] エラーハンドリング全網羅（DESIGN.md「エラーハンドリング方針」表）— 現状は Toast にエラー文字列を投げるのみ
- [ ] SettingsModal でサブカレンダーの色・ラベル上書き（カスタマイズ）
- [ ] イベントのソース／カレンダー間移動（現状: 削除＋再作成）

### Phase 5：リリース準備

- [ ] バージョン v0.1.0 確定 → `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` 更新
- [ ] `Cargo.lock` 再生成（`cargo generate-lockfile`）
- [ ] README.md / README.ja.md 作成
- [ ] CI/CD（GitHub Actions: Windows x86_64 + macOS universal、タグプッシュ自動ビルド）
- [ ] リリースノート（EN/JA）

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

- [ ] 通知・リマインダー機能の要否
- [ ] ウィンドウサイズ・最小サイズの決定
- [ ] macOS / Windows 両対応の動作確認環境
- [ ] CLAUDE.md の起票（Phase 1 開始時）
- [ ] アイコン素材の準備

---

*作成日：2026-04-27*
*Author：osprey74*
