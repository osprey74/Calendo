# Calendo

> [English README is here](README.md)

Microsoft 365・Google Workspace・iCloud のカレンダーを統合し、日次・週次ビューで一元的に閲覧／編集できるデスクトップカレンダークライアント。

**ステータス**: Pre-alpha（初期開発中 — [HANDOFF.md](HANDOFF.md) を参照）

## 対応カレンダーソース

| ソース | プロトコル | 認証方式 |
|---|---|---|
| Microsoft 365（2 アカウント） | Microsoft Graph | OAuth2 + PKCE |
| Google Workspace | Google Calendar API v3 | OAuth2 + PKCE |
| iCloud | CalDAV | アプリ専用パスワード |

## 機能（予定）

- 最大 4 ソースを統合した日次・週次ビュー
- カレンダー毎の表示 ON/OFF・色・ラベルカスタマイズ
- イベント新規作成・編集・削除（繰り返しスコープ：この1件のみ／以降すべて／すべて）
- OS ネイティブの資格情報保管（`keyring` crate 経由で Keychain / Credential Manager）
- Windows・macOS デスクトップビルド

## 技術スタック

- **アプリフレームワーク**: Tauri v2
- **フロントエンド**: React + TypeScript + Vite
- **状態管理**: Zustand
- **バックエンド**: Rust（`reqwest`, `quick-xml`, `keyring`）
- **ビルド／リリース**: GitHub Actions（Windows x86_64 + macOS universal）

詳細仕様は [DESIGN.md](DESIGN.md) を参照してください。

## 開発

> プロジェクトの骨組みを構築中です。セットアップ手順は [HANDOFF.md](HANDOFF.md) の Phase 1 完了後に追記します。

```bash
# (準備中)
npm install
npm run tauri dev
```

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
