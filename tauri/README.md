# Timesheet

A cross-platform timesheet tracking application built with Tauri, Svelte, and Rust.

## Features

- Time tracking with system tray integration
- Session management with JIRA integration
- Week view calendar
- OAuth authentication for JIRA
- System monitors (sleep/wake detection on macOS)

## Tech Stack

- **Backend**: Rust with Tauri 2.0
- **Frontend**: Svelte + TypeScript
- **Database**: SQLite (rusqlite)
- **Integration**: Local worklog and jira crates

## Project Structure

### Rust Backend (`src-tauri/src/`)
- `main.rs` - Tauri entry point and commands
- `models.rs` - Data structures
- `monitors.rs` - System event monitoring
- `timesheet_integration.rs` - Worklog/JIRA integration

### Frontend (`src/`)
- `App.svelte` - Main app component
- `views/` - SessionsView, WeekView, SettingsView
- `lib/` - API client, types, theme

## Development

### Prerequisites
- Rust (latest stable)
- Node.js and npm

### Run
```bash
npm install
npm run tauri:dev
```

### Build Release
```bash
# Build .app bundle (for local use)
npm run tauri:build -- --bundles app

# Build .dmg installer (for distribution)
npm run tauri:build -- --bundles dmg
```

Output locations:
- **App**: `/Users/oyvindh/timesheet/target/release/bundle/macos/Timesheet.app`
- **DMG**: `/Users/oyvindh/timesheet/target/release/bundle/dmg/Timesheet_0.1.0_*.dmg`
- **Binary**: `/Users/oyvindh/timesheet/target/release/timesheet-tauri`

### Universal Binary (Intel + Apple Silicon)
```bash
npm run tauri:build -- --bundles dmg --target universal-apple-darwin
```

## OAuth Setup

See [OAUTH_SETUP.md](OAUTH_SETUP.md) for JIRA OAuth configuration.
