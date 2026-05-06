# Calendo

> Unified desktop calendar client aggregating Microsoft 365, Google Workspace, and iCloud (Tauri v2 + React + TypeScript + Rust).

## Project Overview

複数カレンダーアカウント（Microsoft 365 × 1・Google Workspace × 1・iCloud × 1）を統合し、日次・週次のスケジュールを一覧表示・登録・編集できるデスクトップアプリ。

- **設計の正本**: [DESIGN.md](DESIGN.md)
- **タスク・進捗管理の正本**: [HANDOFF.md](HANDOFF.md)
- **対象 OS**: Windows / macOS

## Architecture

- **Frontend**: React 19 + TypeScript + Vite（[src/](src/)）
- **Backend**: Tauri v2 + Rust（[src-tauri/src/](src-tauri/src/)）
- **State**: Zustand（`calendarStore` / `settingsStore`）
- **Styling**: CSS Modules + CSS Variables
- **Icons**: Material Symbols Rounded（kazahana 準拠）
- **Auth**: OAuth2 + PKCE（MS365 / Google）/ アプリ専用パスワード（iCloud CalDAV）
- **Persistence**: OS Keychain（keyring crate）+ Tauri store plugin

### Source IDs

| ID | プロトコル | 認証 |
|---|---|---|
| `ms365_work1` | Microsoft Graph | OAuth2 + PKCE |
| `google_gws` | Google Calendar API v3 | OAuth2 + PKCE |
| `icloud` | CalDAV | アプリ専用パスワード |

## Task Management

- **task_file**: `HANDOFF.md`
- **done_marker**: `[x]`
- **progress_summary**: true (フェーズ毎チェックボックス・実装メモも更新)

## Documentation

- **docs_to_update**:
  - `README.md` (EN)
  - `README.ja.md` (JA)
  - `DESIGN.md`（仕様変更時）
  - `HANDOFF.md`（タスク・実装メモ更新）
- **doc_pairs**:
  - `README.md` ↔ `README.ja.md`

## Versioning

- **version_files**:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- **extra_version_files**: none
- **cargo_lockfile**: true

## CI/CD

- **cicd**: true
- **cicd_trigger**: tag push（`v*`）
- **cicd_platform**: GitHub Actions (Windows x86_64 + macOS universal)
- **cicd_note**: タグプッシュで自動ビルド＆ Release ドラフト作成
- **secrets**: `MS_CLIENT_ID` / `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`（コンパイル時 env 注入）

## Build & Run

```bash
# Dev（Tauri ウィンドウ起動）
npm run tauri dev

# Frontend のみ
npm run dev

# Release build
npm run tauri build

# Rust 単体チェック
cd src-tauri && cargo check
```

## Local .env

OAuth クライアント ID／シークレットは `.env`（gitignore 済）からコンパイル時注入：

```
MS_CLIENT_ID=...
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
```

`src-tauri/build.rs` が `.env` を読み取り `cargo:rustc-env=` で注入。

## SNS

- **sns_accounts**: none
