# HANDOFF.md — Calendo

**最終更新**: 2026-04-27
**バージョン**: v0.0.0（プロジェクト未初期化）
**フェーズ**: Phase 0（事前準備）着手前

---

## プロジェクト概要

複数カレンダーアカウント（Microsoft 365 × 2・Google Workspace × 1・iCloud × 1）を統合し、
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
| `ms365_work1` | 仕事 A | Microsoft Graph | `#0582AF` | OAuth2 + PKCE |
| `ms365_work2` | 仕事 B | Microsoft Graph | `#E65C00` | OAuth2 + PKCE |
| `google_gws` | Google カレンダー | Google Calendar API v3 | `#2E7D32` | OAuth2 + PKCE |
| `icloud` | プライベート | CalDAV | `#888780` | アプリ専用パスワード |

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

- [ ] Tauri v2 プロジェクト初期化（`bun create tauri-app` または `npm create tauri-app`）
- [ ] React + TypeScript + Vite テンプレート選択
- [ ] `tauri-plugin-oauth` 導入（コールバック受信用ローカルサーバ）
- [ ] `tauri-plugin-store` 導入（設定永続化）
- [ ] `keyring` crate 追加・OS Keychain 読み書きラッパー実装（[src-tauri/src/auth/keyring.rs](src-tauri/src/auth/keyring.rs)）
- [ ] PKCE コード生成・トークン交換・リフレッシュ共通実装（[src-tauri/src/auth/oauth.rs](src-tauri/src/auth/oauth.rs)）
- [ ] Microsoft Graph OAuth フロー実装（`auth_start("ms365_work1" | "ms365_work2")`）
- [ ] Google Calendar OAuth フロー実装（`auth_start("google_gws")`）
- [ ] iCloud アプリ専用パスワード保存コマンド（`auth_icloud_save`）
- [ ] iCloud CalDAV 接続確認（`PROPFIND /` でプリンシパル URL 取得）
- [ ] 各ソースの動作確認用 CLI 的フロント（最小限）

### Phase 2：イベント取得・表示

#### 2-A：カレンダー一覧取得

- [ ] Graph API カレンダー一覧（`GET /me/calendars`）→ `CalendarMeta[]`
- [ ] Google Calendar `calendarList.list` → `CalendarMeta[]`
- [ ] CalDAV `PROPFIND` でカレンダーホーム → 各 `calendar` リソース列挙

#### 2-B：イベント取得

- [ ] Graph API `calendarView`（startDateTime / endDateTime）→ 繰り返し展開済みインスタンス取得
- [ ] Google Calendar `events.list`（`singleEvents=true&orderBy=startTime`）→ 展開済みインスタンス
- [ ] CalDAV `REPORT` クエリ（`time-range`）→ VEVENT 群、RRULE は手動展開
- [ ] UnifiedEvent への正規化・JST 変換（[src/utils/eventUtils.ts](src/utils/eventUtils.ts), [src-tauri/src/models.rs](src-tauri/src/models.rs)）
- [ ] 繰り返しイベントの展開（CalDAV のみ自前 RRULE 展開、Graph / GCal はサーバ側で展開済み）

#### 2-C：UI

- [ ] AppShell（サイドバー + メインパネル）
- [ ] TopBar（日/週切替・日付ナビ・新規ボタン）
- [ ] CalendarSidebar（ソース・サブカレンダー ON/OFF トグル）
- [ ] DayView（時間軸縦スクロール）
- [ ] WeekView（7 日横並び）
- [ ] AllDayBar（終日イベント帯）
- [ ] EventCard（一覧表示用）
- [ ] Zustand store（`calendarStore` / `settingsStore`）
- [ ] フィルタ：ソース ON/OFF・サブカレンダー ON/OFF

### Phase 3：イベント作成・編集・削除

- [ ] EventModal UI（タイトル・日時・終日・場所・メモ・登録先カレンダー2段階選択）
- [ ] バリデーション（タイトル必須・終了 ≥ 開始）
- [ ] 繰り返しイベント編集ダイアログ（「この1件のみ」「以降すべて」「すべて」）
- [ ] Graph 書き込み実装
  - 新規: `POST /me/calendars/{id}/events`
  - 更新: `PATCH /me/events/{id}`（`thisAndFollowing` / `singleInstance` / `master`）
  - 削除: `DELETE /me/events/{id}`
- [ ] Google Calendar 書き込み実装
  - 新規: `POST /calendars/{calendarId}/events`
  - 更新: `PATCH /calendars/{calendarId}/events/{eventId}`（`recurringEventId` 連動）
  - 削除: `DELETE` + `sendUpdates=none`
- [ ] CalDAV 書き込み実装
  - 新規: VEVENT を含む `.ics` を `PUT`
  - 更新: `RECURRENCE-ID` 付き VEVENT で部分上書き or `EXDATE` 追記
  - 削除: `DELETE` or `EXDATE`
- [ ] 繰り返し編集スコープ別の API 呼び出し分岐ロジック

### Phase 4：設定・仕上げ

- [ ] SettingsModal 実装
  - アカウント接続状態（接続済 / 未接続）
  - 接続・再接続・切断ボタン
  - iCloud Apple ID + アプリ専用パスワード入力
  - サブカレンダー一覧・表示 ON/OFF・色・ラベルカスタマイズ
  - 書き込み不可カレンダーは読み取り専用バッジ
- [ ] Toast 実装（成功・エラー通知）
- [ ] 自動トークンリフレッシュ（401 検出 → リフレッシュ → リトライ）
- [ ] エラーハンドリング全網羅（DESIGN.md「エラーハンドリング方針」表）
- [ ] アプリアイコン作成・配置
- [ ] 初回起動時オンボーディング（最小限：「設定からアカウントを接続してください」）

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
