# Calendo

> [English README is here](README.md)

Microsoft 365・Google Workspace・iCloud のカレンダーを統合し、日次・週次ビューで一元的に閲覧／編集できるデスクトップカレンダークライアント。

**バージョン**: v0.1.0 — 開発状況とロードマップは [HANDOFF.md](HANDOFF.md) を参照してください。

## 対応カレンダーソース

| ソース | プロトコル | 認証方式 |
|---|---|---|
| Microsoft 365 | Microsoft Graph | OAuth2 + PKCE |
| Google Workspace | Google Calendar API v3 | OAuth2 + PKCE |
| iCloud | CalDAV | アプリ専用パスワード |

## 機能

- 最大 3 ソースを統合した日次・週次ビュー（JST 統一表示）
- ソース毎・サブカレンダー毎の表示 ON/OFF、非表示折りたたみ、ソース単位の一括 ON/OFF
- サブカレンダー毎の表示名・色の上書きカスタマイズ（プロバイダ側を変更せずローカルに反映）
- 表示時間範囲の指定（例: 7:00〜21:00）。範囲外のイベントは自動的にクリップ表示
- 1 時間あたりの縦幅を調整可能（40〜240 px、8 段階）。中央表示クリックで既定値 60 px に即復帰
- 現在時刻インジケーター — タイムライン上に赤い水平線、週ビューでは 7 日を横断・今日列にドット
- イベントの新規作成・編集・削除（ソース × カレンダーの 2 段階選択）
- 繰り返しイベント — 作成時に RRULE プリセット（毎日／毎週／平日のみ／毎月／毎年）を指定可能。編集／削除時のスコープは「この1件のみ」「すべて」をサポート
- OS ネイティブの資格情報保管（macOS Keychain / Windows Credential Manager 経由、`keyring` crate）
- 401 レスポンス時の OAuth トークン自動リフレッシュ、永続的な失敗時は再ログイン誘導
- エラー種別ごとに分類した日本語メッセージ（認証期限切れ／ネットワーク／権限／未検出／競合／レート制限／サーバ）
- ソース毎／カレンダー毎のフェッチ警告を集約して通知（あるソースの認証期限切れが他ソースをブロックしない）
- UI 設定（表示モード／ソース・サブカレンダー ON/OFF／時間軸ズーム／表示時間範囲／カレンダーの色・表示名上書き）の永続化（Tauri store plugin）
- 書き込み操作の Toast 通知
- Windows（x86_64）・macOS（universal）デスクトップビルド

## 技術スタック

- **アプリフレームワーク**: Tauri v2
- **フロントエンド**: React 19 + TypeScript + Vite、Zustand、CSS Modules
- **バックエンド**: Rust（`reqwest`、`quick-xml`、`chrono-tz`、`keyring`）
- **ビルド／リリース**: GitHub Actions（Windows x86_64 + macOS universal、タグ起動）

詳細仕様は [DESIGN.md](DESIGN.md) を参照してください。

## セットアップ

### 前提環境

- Node.js 20+（または Bun）
- Rust toolchain（stable）
- Tauri 必須環境 — Windows: WebView2 + MSVC build tools / macOS: Xcode Command Line Tools

### OAuth クライアントの取得

Calendo は共有クレデンシャルを同梱していません。各サービスで自身の OAuth クライアントを用意してください。

1. **Microsoft 365** — Azure Portal でパブリッククライアント・デスクトップアプリを登録。スコープ: `Calendars.ReadWrite`、`offline_access`、`User.Read`。リダイレクト URI に `http://localhost` を追加。
2. **Google Workspace** — Google Cloud Console で「デスクトップ アプリ」種別の OAuth クライアント ID を発行し、Calendar API を有効化。
3. **iCloud** — [appleid.apple.com](https://appleid.apple.com/) でアプリ専用パスワードを発行（アプリ起動後の設定画面で入力）。

`.env.example` を `.env` にコピーし、値を記入：

```
MS_CLIENT_ID=...
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
```

`src-tauri/build.rs` がコンパイル時に `.env` を読み取って注入します。`.env` は .gitignore 済。

### 開発実行

```bash
npm install
npm run tauri dev
```

### リリースビルド

```bash
npm run tauri build
```

CI ビルドは `v*.*.*` タグのプッシュで起動します。GitHub Actions が Windows・macOS の成果物を作成し、リリースドラフトを自動生成します。

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
