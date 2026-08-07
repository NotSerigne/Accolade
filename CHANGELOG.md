# Changelog

All notable changes to Accolade are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.4] - 2025-08-07

### Fixed
- **Auto-start**: the main window no longer hides itself on a normal launch.
  `startMinimized` now only takes effect when the app is started by Windows
  auto-start (i.e. with the `--minimized` flag).
- **Goldberg notifications**: achievements were detected in the UI but no
  overlay notification was shown. The watcher now snapshots the previously
  unlocked set *before* merging live data, so newly unlocked achievements are
  correctly diffed against the pre-merge baseline.

## [1.0.3] - 2025-08-05

### Added
- Screenshots feature with configurable global shortcut.
- Auto-group games by emulator.
- PDF export of achievement profile.
- SteamGridDB icon support.
- Dynamic theme color extracted from game artwork.

### Fixed
- Various stability fixes and UI improvements.

## [1.0.0] - 2025-07-01

### Added
- Initial release.
- Achievement tracking for Goldberg, Empress, CODEX, OnlineFix, RUNE.
- Real-time overlay notifications.
- Steam metadata enrichment (names, icons, descriptions, rarity).
- Multi-language support: FR, EN, ES, DE, IT.
- Light / Dark / System theme with custom accent color.
- Auto-start with system.
- Minimize to tray.
