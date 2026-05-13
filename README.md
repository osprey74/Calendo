# Calendo

> [日本語版 README はこちら](README.ja.md)

A unified desktop calendar client that aggregates Microsoft 365, Google Workspace, and iCloud calendars into a single day/week view.

**Version**: v0.1.0 — see [HANDOFF.md](HANDOFF.md) for development status and roadmap.

## Supported Calendar Sources

| Source | Protocol | Authentication |
|---|---|---|
| Microsoft 365 | Microsoft Graph | OAuth2 + PKCE |
| Google Workspace | Google Calendar API v3 | OAuth2 + PKCE |
| iCloud | CalDAV | App-specific password |

## Features

- Day / Week views aggregating up to three calendar sources with timezone-aware (JST) layout
- Per-source and per-subcalendar visibility toggles, with hidden-calendar collapse and bulk on/off
- Per-calendar display overrides — custom color and display name without round-tripping to the provider
- Configurable visible time-of-day window (e.g. 7:00–21:00); out-of-window events are clipped automatically
- Adjustable hour-row height (40–240 px / hour, 8 presets) with click-to-reset to the default 60 px
- Current-time indicator — red horizontal line across the time grid, with a dot marking today's column in week view
- Event create / edit / delete with two-step source × calendar selection
- Recurring event support — preset RRULEs (daily / weekly / weekdays / monthly / yearly) at creation time, with edit / delete scopes ("this only" / "all")
- OS-native credential storage via the `keyring` crate (Keychain on macOS, Credential Manager on Windows)
- Automatic OAuth token refresh on 401 responses; re-auth prompted on persistent failure
- Structured error classification with localized messages (auth / network / permission / not-found / conflict / rate-limit / server)
- Per-source/per-calendar fetch warnings surfaced as info toasts (one source's auth expiry doesn't block the rest)
- Persisted UI settings (view mode, source/subcalendar visibility, hour height, visible time window, color & label overrides) via Tauri store plugin
- Toast notifications for write operations
- Windows (x86_64) & macOS (universal) desktop builds

## Tech Stack

- **App framework**: Tauri v2
- **Frontend**: React 19 + TypeScript + Vite, Zustand, CSS Modules
- **Backend**: Rust (`reqwest`, `quick-xml`, `chrono-tz`, `keyring`)
- **Build / Release**: GitHub Actions (Windows x86_64 + macOS universal, tag-triggered)

See [DESIGN.md](DESIGN.md) for the full specification.

## Getting Started

### Prerequisites

- Node.js 20+ (or Bun)
- Rust toolchain (stable)
- Tauri prerequisites — Windows: WebView2 + MSVC build tools / macOS: Xcode Command Line Tools

### OAuth Client Setup

Calendo requires your own OAuth clients (no shared credentials are bundled).

1. **Microsoft 365** — Register a public-client desktop app in Azure Portal with `Calendars.ReadWrite`, `offline_access`, `User.Read` scopes and `http://localhost` as a redirect URI.
2. **Google Workspace** — Create a Desktop-app OAuth client in Google Cloud Console with the Calendar API enabled.
3. **iCloud** — Generate an app-specific password at [appleid.apple.com](https://appleid.apple.com/) (entered at runtime in the app's settings).

Copy `.env.example` to `.env` and fill in the values:

```
MS_CLIENT_ID=...
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
```

These are injected at compile time via `src-tauri/build.rs`. `.env` is gitignored.

### Development

```bash
npm install
npm run tauri dev
```

### Release Build

```bash
npm run tauri build
```

CI builds are triggered by pushing a `v*.*.*` tag — GitHub Actions produces Windows and macOS artifacts and creates a draft release.

## License

MIT — see [LICENSE](LICENSE).
