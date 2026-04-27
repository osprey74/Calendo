# Calendo

> [日本語版 README はこちら](README.ja.md)

A unified desktop calendar client that aggregates Microsoft 365, Google Workspace, and iCloud calendars into a single day/week view.

**Status**: Pre-alpha (under initial development — see [HANDOFF.md](HANDOFF.md))

## Supported Calendar Sources

| Source | Protocol | Authentication |
|---|---|---|
| Microsoft 365 (×2 accounts) | Microsoft Graph | OAuth2 + PKCE |
| Google Workspace | Google Calendar API v3 | OAuth2 + PKCE |
| iCloud | CalDAV | App-specific password |

## Features (planned)

- Day / Week views aggregating up to four calendar sources
- Per-calendar visibility toggles, custom color & label
- Event create / edit / delete (with recurring-event scope: this only / this and following / all)
- OS-native credential storage (Keychain / Credential Manager via `keyring` crate)
- Windows & macOS desktop builds

## Tech Stack

- **App framework**: Tauri v2
- **Frontend**: React + TypeScript + Vite
- **State**: Zustand
- **Backend**: Rust (`reqwest`, `quick-xml`, `keyring`)
- **Build / Release**: GitHub Actions (Windows x86_64 + macOS universal)

See [DESIGN.md](DESIGN.md) for the full specification.

## Development

> Project scaffolding is in progress. Setup instructions will be filled in once Phase 1 of [HANDOFF.md](HANDOFF.md) is complete.

```bash
# (Coming soon)
npm install
npm run tauri dev
```

## License

MIT — see [LICENSE](LICENSE).
