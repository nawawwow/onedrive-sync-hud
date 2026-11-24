# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-24

### Added

- Native Windows UI Automation (`IUIAutomation`) taskbar tray tooltip reader (`Shell_TrayWnd` and overflow flyout) to capture real-time OneDrive status strings reliably under both standard user and elevated sessions.
- Dynamic compact pill chevron direction synced with screen half (`hud-anchor-changed`), pointing upward in the lower half and downward in the upper half.

### Changed

- Widened the expanded Fluent Glass detail card to `384px` and moved the status badge onto the subtitle row so full account names (such as `OneDrive - Personal`) never truncate.
- Moved recursive OneDrive folder watcher initialization onto a background thread and displayed the pill immediately on startup for near-instant launch visibility.

### Fixed

- Debounced drag-end edge snapping and position persistence using `WindowEvent::Moved` paired with `GetAsyncKeyState(VK_LBUTTON)`, preventing premature snapping when `startDragging()` resolves early on Windows.
- Preserved pill visibility after clicking **Reset Position** in the inline Settings drawer by applying a post-reset grace period before idle auto-hide resumes.
- Automatically pruned deleted files from *Recent File Activity* so every file row remains valid for 1-click Explorer reveal.
- Expanded asymmetric transparent window shadow halo padding (`20px` top, `24px` sides, `30px` bottom) and calibrated Fluent elevation shadows so drop shadows fade smoothly with zero rectangular clipping on all sides.

## [0.1.0] - 2026-11-23

### Added

- Floating Windows 11 Fluent Glass compact pill with click-to-expand adaptive detail panel (anchors upward near the bottom of the screen and downward near the top).
- Native Windows 11 OneDrive multi-source telemetry combining `IStorageProviderStatusUISourceFactory` COM/WinRT status, `SyncDiagnostics.log` counters, `OneDrive.exe` process I/O throughput, and real-time folder file watching via `ReadDirectoryChangesW` and Shell `PKEY_StorageProviderState`.
- Live 30-sample transfer speed sparkline, active/recent files list with 1-click **Reveal in Explorer**, OneDrive cloud storage quota bar, and quick actions (**Open Folder**, official OneDrive provider commands, and **Launch OneDrive**).
- Automatic multi-account discovery for OneDrive Personal and Work/School accounts with aggregate summary and per-account tabs.
- Smart visibility engine with configurable idle auto-hide, persistent error/paused alerts with dismiss control, and fullscreen application detection.
- Dynamic System Tray icon and quick menu, global `Ctrl+Alt+O` show/hide shortcut, multi-monitor drag with edge snapping, position lock, opacity control, automatic Windows Light/Dark Fluent Glass theming, and optional **Start with Windows** integration.

[Unreleased]: https://github.com/nawawwow/onedrive-sync-hud/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/nawawwow/onedrive-sync-hud/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nawawwow/onedrive-sync-hud/releases/tag/v0.1.0
