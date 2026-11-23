# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-23

### Added

- Floating Windows 11 Fluent Glass compact pill with click-to-expand adaptive detail panel (anchors upward near the bottom of the screen and downward near the top).
- Native Windows 11 OneDrive multi-source telemetry combining `IStorageProviderStatusUISourceFactory` COM/WinRT status, `SyncDiagnostics.log` counters, `OneDrive.exe` process I/O throughput, and real-time folder file watching via `ReadDirectoryChangesW` and Shell `PKEY_StorageProviderState`.
- Live 30-sample transfer speed sparkline, active/recent files list with 1-click **Reveal in Explorer**, OneDrive cloud storage quota bar, and quick actions (**Open Folder**, official OneDrive provider commands, and **Launch OneDrive**).
- Automatic multi-account discovery for OneDrive Personal and Work/School accounts with aggregate summary and per-account tabs.
- Smart visibility engine with configurable idle auto-hide, persistent error/paused alerts with dismiss control, and fullscreen application detection.
- Dynamic System Tray icon and quick menu, global `Ctrl+Alt+O` show/hide shortcut, multi-monitor drag with edge snapping, position lock, opacity control, automatic Windows Light/Dark Fluent Glass theming, and optional **Start with Windows** integration.

[Unreleased]: https://github.com/nawawwow/onedrive-sync-hud/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nawawwow/onedrive-sync-hud/releases/tag/v0.1.0
