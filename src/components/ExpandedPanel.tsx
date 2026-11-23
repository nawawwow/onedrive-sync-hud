import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { AccountTelemetry, AppSettings, HudSnapshot, SettingsPatch } from "../types";
import {
  formatBytes,
  formatEta,
  formatRelativeTime,
  formatSpeed,
  isAttentionState,
  stateToneClass,
} from "../utils/format";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  ChevronIcon,
  CloseIcon,
  CloudStatusIcon,
  FileStateDot,
  FolderIcon,
  LockIcon,
  PinIcon,
  SettingsIcon,
} from "./Icons";
import { SpeedSparkline } from "./SpeedSparkline";

interface ExpandedPanelProps {
  snapshot: HudSnapshot;
  settings: AppSettings;
  expandedUpward: boolean;
  onCollapse: () => void;
  onStartDrag: () => void;
  onUpdateSettings: (patch: SettingsPatch) => void;
  onDismiss: () => void;
  onResetPosition: () => void;
  onOpenFolder: (folderPath: string) => void;
  onRevealFile: (filePath: string) => void;
  onLaunchOneDrive: () => void;
  onOpenOneDriveFlyout: () => void;
  onInvokeProviderCommand: (accountId: string, commandId: string) => Promise<void>;
  onQuit: () => void;
  onMeasureHeight: (logicalHeight: number) => void;
}

export function ExpandedPanel({
  snapshot,
  settings,
  expandedUpward,
  onCollapse,
  onStartDrag,
  onUpdateSettings,
  onDismiss,
  onResetPosition,
  onOpenFolder,
  onRevealFile,
  onLaunchOneDrive,
  onOpenOneDriveFlyout,
  onInvokeProviderCommand,
  onQuit,
  onMeasureHeight,
}: ExpandedPanelProps) {
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [showSettings, setShowSettings] = useState(false);
  const [busyActionId, setBusyActionId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const dragOrigin = useRef<{ x: number; y: number; dragged: boolean } | null>(null);

  const activeAccount: AccountTelemetry | null =
    snapshot.accounts.find((a) => a.id === selectedAccountId) ?? snapshot.accounts[0] ?? null;

  useEffect(() => {
    if (activeAccount && activeAccount.id !== selectedAccountId) {
      setSelectedAccountId(activeAccount.id);
    }
  }, [activeAccount, selectedAccountId]);

  useEffect(() => {
    const el = cardRef.current;
    if (!el) return;
    const notify = () => {
      const rect = el.getBoundingClientRect();
      if (rect.height > 100) {
        onMeasureHeight(Math.ceil(rect.height) + 20);
      }
    };
    notify();
    const observer = new ResizeObserver(notify);
    observer.observe(el);
    return () => observer.disconnect();
  }, [onMeasureHeight, showSettings, snapshot.accounts.length]);

  const effectiveState = activeAccount ? activeAccount.state : snapshot.aggregateState;
  const effectiveLabel = activeAccount ? activeAccount.stateLabel : snapshot.aggregateLabel;
  const effectiveSummary = activeAccount ? activeAccount.statusSummary : snapshot.aggregateSummary;
  const effectiveDetail = activeAccount?.statusDetail ?? null;
  const effectiveProgress = activeAccount
    ? activeAccount.progressPercent
    : snapshot.aggregateProgressPercent;

  const tone = stateToneClass(effectiveState);
  const isSyncing = effectiveState === "syncing";
  const isIndeterminate = isSyncing && effectiveProgress === null;
  const barWidth =
    effectiveState === "in_sync"
      ? 100
      : effectiveProgress !== null
        ? Math.max(4, Math.min(100, effectiveProgress))
        : 0;

  const uploadSpeed = activeAccount ? activeAccount.uploadSpeedBps : snapshot.uploadSpeedBps;
  const downloadSpeed = activeAccount ? activeAccount.downloadSpeedBps : snapshot.downloadSpeedBps;
  const etaStr = formatEta(activeAccount?.etaSeconds ?? snapshot.totalEtaSeconds);

  const totalUploadBytes = activeAccount?.bytesToUpload ?? 0;
  const doneUploadBytes = activeAccount?.bytesUploaded ?? 0;
  const totalDownloadBytes = activeAccount?.bytesToDownload ?? 0;
  const doneDownloadBytes = activeAccount?.bytesDownloaded ?? 0;
  const hasByteTotals = totalUploadBytes > 0 || totalDownloadBytes > 0;

  const primaryProviderCmd =
    activeAccount?.providerCommands.find(
      (c) =>
        c.enabled &&
        (c.label.toLowerCase().includes("pause") ||
          c.label.toLowerCase().includes("resume") ||
          c.kind === "primary"),
    ) ?? null;

  const handleHeaderPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || settings.locked) return;
    if ((e.target as HTMLElement).closest("button")) return;
    dragOrigin.current = { x: e.clientX, y: e.clientY, dragged: false };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handleHeaderPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const origin = dragOrigin.current;
    if (!origin || origin.dragged || settings.locked) return;
    if (Math.abs(e.clientX - origin.x) >= 5 || Math.abs(e.clientY - origin.y) >= 5) {
      origin.dragged = true;
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {
        // ignore
      }
      onStartDrag();
    }
  };

  const handleHeaderPointerUp = () => {
    dragOrigin.current = null;
  };

  const runProviderCommand = async (accountId: string, commandId: string) => {
    setActionError(null);
    setBusyActionId(commandId);
    try {
      await onInvokeProviderCommand(accountId, commandId);
    } catch (err) {
      setActionError(err instanceof Error ? err.message : "Could not invoke OneDrive action");
      onOpenOneDriveFlyout();
    } finally {
      setBusyActionId(null);
    }
  };

  const quota = activeAccount?.quota ?? null;
  const quotaTone =
    quota && quota.usedPercent >= 95
      ? "tone-error"
      : quota && quota.usedPercent >= 85
        ? "tone-warning"
        : "tone-syncing";

  return (
    <div
      ref={cardRef}
      className={`fluent-card ${tone} ${expandedUpward ? "origin-bottom" : "origin-top"}`}
      role="region"
      aria-label="OneDrive Sync Details"
    >
      {/* Draggable Header */}
      <div
        className={`card-header ${settings.locked ? "" : "is-draggable"}`}
        onPointerDown={handleHeaderPointerDown}
        onPointerMove={handleHeaderPointerMove}
        onPointerUp={handleHeaderPointerUp}
      >
        <div className="card-header-left">
          <div className={`status-orb status-orb-md ${tone}`}>
            <CloudStatusIcon state={effectiveState} size={17} />
          </div>
          <div className="card-title-group">
            <div className="card-title-row">
              <span className="card-app-title">
                {activeAccount ? activeAccount.displayName : "OneDrive"}
              </span>
              <span className={`state-chip ${tone}`}>{effectiveLabel}</span>
            </div>
            <span className="card-subtitle" title={effectiveSummary}>
              {effectiveSummary}
            </span>
          </div>
        </div>

        <div className="card-header-actions">
          <button
            type="button"
            className={`icon-btn ${!settings.autoHideWhenIdle ? "icon-btn-active" : ""}`}
            title={
              settings.autoHideWhenIdle
                ? "Auto-hide active — click to pin HUD permanently"
                : "Pinned always visible — click to enable auto-hide"
            }
            aria-label="Toggle pin always visible"
            aria-pressed={!settings.autoHideWhenIdle}
            onClick={() => onUpdateSettings({ autoHideWhenIdle: !settings.autoHideWhenIdle })}
          >
            <PinIcon size={14} pinned={!settings.autoHideWhenIdle} />
          </button>

          <button
            type="button"
            className={`icon-btn ${settings.locked ? "icon-btn-active" : ""}`}
            title={settings.locked ? "Position locked — click to unlock drag" : "Lock widget position"}
            aria-label="Toggle lock position"
            aria-pressed={settings.locked}
            onClick={() => onUpdateSettings({ locked: !settings.locked })}
          >
            <LockIcon size={14} locked={settings.locked} />
          </button>

          <button
            type="button"
            className={`icon-btn ${showSettings ? "icon-btn-active" : ""}`}
            title="HUD Settings"
            aria-label="HUD Settings"
            aria-pressed={showSettings}
            onClick={() => setShowSettings((prev) => !prev)}
          >
            <SettingsIcon size={14} />
          </button>

          <button
            type="button"
            className="icon-btn"
            title="Collapse to compact pill"
            aria-label="Collapse to compact pill"
            onClick={onCollapse}
          >
            <ChevronIcon size={14} rotated={!expandedUpward} />
          </button>
        </div>
      </div>

      {/* Multi-account Switcher (only shown when > 1 OneDrive account is configured) */}
      {snapshot.accounts.length > 1 && !showSettings ? (
        <div className="account-tabs" role="tablist" aria-label="OneDrive Accounts">
          {snapshot.accounts.map((acc) => {
            const selected = acc.id === activeAccount?.id;
            return (
              <button
                key={acc.id}
                type="button"
                role="tab"
                aria-selected={selected}
                className={`account-tab ${selected ? "account-tab-active" : ""}`}
                onClick={() => setSelectedAccountId(acc.id)}
              >
                <span className={`account-dot ${stateToneClass(acc.state)}`} />
                <span className="account-tab-label">
                  {acc.displayName.replace(/^OneDrive\s*-\s*/i, "")}
                </span>
              </button>
            );
          })}
        </div>
      ) : null}

      {/* Inline Settings View */}
      {showSettings ? (
        <div className="settings-drawer">
          <div className="settings-section-title">Behavior & Display</div>

          <label className="setting-row">
            <span className="setting-copy">
              <span className="setting-label">Auto-hide when idle</span>
              <span className="setting-hint">Fade away {settings.idleHideDelaySecs}s after Up to date</span>
            </span>
            <input
              type="checkbox"
              aria-label="Auto-hide when idle"
              className="fluent-switch"
              checked={settings.autoHideWhenIdle}
              onChange={(e) => onUpdateSettings({ autoHideWhenIdle: e.target.checked })}
            />
          </label>

          {settings.autoHideWhenIdle ? (
            <div className="setting-row setting-subrow">
              <span className="setting-label">Idle hide delay</span>
              <div className="segmented-mini">
                {[4, 6, 10, 15].map((sec) => (
                  <button
                    key={sec}
                    type="button"
                    className={`seg-btn ${settings.idleHideDelaySecs === sec ? "seg-btn-active" : ""}`}
                    onClick={() => onUpdateSettings({ idleHideDelaySecs: sec })}
                  >
                    {sec}s
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          <label className="setting-row">
            <span className="setting-copy">
              <span className="setting-label">Auto-hide in fullscreen</span>
              <span className="setting-hint">Hide during games, videos & presentations</span>
            </span>
            <input
              type="checkbox"
              aria-label="Auto-hide in fullscreen"
              className="fluent-switch"
              checked={settings.hideInFullscreen}
              onChange={(e) => onUpdateSettings({ hideInFullscreen: e.target.checked })}
            />
          </label>

          <label className="setting-row">
            <span className="setting-copy">
              <span className="setting-label">Start with Windows</span>
              <span className="setting-hint">Launch silently into System Tray at login</span>
            </span>
            <input
              type="checkbox"
              aria-label="Start with Windows"
              className="fluent-switch"
              checked={settings.startWithWindows}
              onChange={(e) => onUpdateSettings({ startWithWindows: e.target.checked })}
            />
          </label>

          <div className="setting-row">
            <span className="setting-label">Theme</span>
            <div className="segmented-mini">
              {(["auto", "light", "dark"] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className={`seg-btn ${settings.theme === mode ? "seg-btn-active" : ""}`}
                  onClick={() => onUpdateSettings({ theme: mode })}
                >
                  {mode === "auto" ? "Auto" : mode === "light" ? "Light" : "Dark"}
                </button>
              ))}
            </div>
          </div>

          <div className="setting-row">
            <span className="setting-label">Opacity</span>
            <div className="segmented-mini">
              {[0.7, 0.8, 0.9, 1.0].map((op) => (
                <button
                  key={op}
                  type="button"
                  className={`seg-btn ${Math.abs(settings.opacity - op) < 0.04 ? "seg-btn-active" : ""}`}
                  onClick={() => onUpdateSettings({ opacity: op })}
                >
                  {Math.round(op * 100)}%
                </button>
              ))}
            </div>
          </div>

          <div className="settings-footer-actions">
            <button type="button" className="btn-secondary" onClick={onResetPosition}>
              Reset Position
            </button>
            <button type="button" className="btn-danger-ghost" onClick={onQuit}>
              Quit HUD
            </button>
          </div>
        </div>
      ) : (
        <div className="card-content">
          {/* Attention / Error / Paused / Not Running Alert Banner */}
          {isAttentionState(effectiveState) ? (
            <div className={`alert-banner ${tone}`} role="alert">
              <div className="alert-copy">
                <div className="alert-title">
                  {effectiveState === "not_running"
                    ? "OneDrive is not running"
                    : effectiveState === "paused"
                      ? "Syncing is currently paused"
                      : effectiveState === "error"
                        ? "Sync issue requires attention"
                        : effectiveLabel}
                </div>
                <div className="alert-desc">
                  {effectiveState === "not_running"
                    ? "Start OneDrive to monitor file transfers."
                    : activeAccount && (activeAccount.errorCount > 0 || activeAccount.conflictCount > 0)
                      ? `${activeAccount.errorCount} transfer error(s), ${activeAccount.conflictCount} conflict(s).`
                      : effectiveDetail || effectiveSummary}
                </div>
              </div>
              <div className="alert-actions">
                {effectiveState === "not_running" ? (
                  <button type="button" className="btn-primary-sm" onClick={onLaunchOneDrive}>
                    Launch
                  </button>
                ) : (
                  <button type="button" className="btn-secondary-sm" onClick={onOpenOneDriveFlyout}>
                    Open
                  </button>
                )}
                <button
                  type="button"
                  className="icon-btn icon-btn-sm"
                  title="Dismiss notification until state changes"
                  aria-label="Dismiss notification"
                  onClick={onDismiss}
                >
                  <CloseIcon size={12} />
                </button>
              </div>
            </div>
          ) : null}

          {/* Progress & Transfer Metrics */}
          <div className="progress-section">
            <div className="progress-meta-row">
              <span className="progress-caption">
                {effectiveState === "unknown" ? (
                  <span className="skeleton-line" aria-label="Loading sync status" />
                ) : isSyncing ? (
                  hasByteTotals ? (
                    `${formatBytes(doneUploadBytes + doneDownloadBytes)} of ${formatBytes(totalUploadBytes + totalDownloadBytes)}`
                  ) : (
                    effectiveDetail || effectiveSummary
                  )
                ) : effectiveState === "in_sync" ? (
                  "All files are up to date"
                ) : (
                  effectiveDetail || effectiveSummary
                )}
              </span>
              <span className="progress-pct tabular">
                {effectiveProgress !== null ? `${Math.round(effectiveProgress)}%` : isSyncing ? "Scanning" : "—"}
              </span>
            </div>

            <div className="progress-track card-progress">
              {isIndeterminate ? (
                <div className={`progress-indeterminate ${tone}`} />
              ) : (
                <div
                  className={`progress-fill ${tone} ${isSyncing ? "progress-shimmer" : ""}`}
                  style={{ width: `${barWidth}%` }}
                />
              )}
            </div>

            <div className="transfer-stats-grid">
              <div className="stat-chip">
                <span className="stat-icon stat-up">
                  <ArrowUpIcon size={12} />
                </span>
                <div className="stat-body">
                  <span className="stat-value tabular">{formatSpeed(uploadSpeed)}</span>
                  <span className="stat-label tabular">
                    {activeAccount && activeAccount.filesToUpload > 0
                      ? `${activeAccount.filesToUpload} file${activeAccount.filesToUpload === 1 ? "" : "s"} up`
                      : "Upload"}
                  </span>
                </div>
              </div>

              <div className="stat-chip">
                <span className="stat-icon stat-down">
                  <ArrowDownIcon size={12} />
                </span>
                <div className="stat-body">
                  <span className="stat-value tabular">{formatSpeed(downloadSpeed)}</span>
                  <span className="stat-label tabular">
                    {activeAccount && activeAccount.filesToDownload > 0
                      ? `${activeAccount.filesToDownload} file${activeAccount.filesToDownload === 1 ? "" : "s"} down`
                      : etaStr || "Download"}
                  </span>
                </div>
              </div>
            </div>

            {etaStr && (activeAccount?.filesToDownload ?? 0) > 0 ? (
              <div className="eta-pill tabular">Estimated remaining: {etaStr}</div>
            ) : null}

            <SpeedSparkline historyBps={snapshot.speedHistoryBps} active={isSyncing} />
          </div>

          {/* Active & Recent Files List */}
          <div className="files-section">
            <div className="section-header">
              <span className="section-title">Recent File Activity</span>
              {activeAccount && activeAccount.recentFiles.length > 0 ? (
                <span className="section-count tabular">{activeAccount.recentFiles.length}</span>
              ) : null}
            </div>

            {activeAccount && activeAccount.recentFiles.length > 0 ? (
              <ul className="files-list" aria-label="Recently synced files">
                {activeAccount.recentFiles.slice(0, 8).map((file) => (
                  <li key={file.path}>
                    <button
                      type="button"
                      className="file-row"
                      title={`Reveal in Explorer: ${file.relativePath}`}
                      onClick={() => onRevealFile(file.path)}
                    >
                      <FileStateDot state={file.state} />
                      <div className="file-info">
                        <span className="file-name">{file.fileName}</span>
                        <span className="file-sub">
                          {file.relativePath !== file.fileName ? file.relativePath : "Root folder"}
                        </span>
                      </div>
                      <div className="file-meta tabular">
                        <span className="file-size">{formatBytes(file.sizeBytes)}</span>
                        <span className="file-time">{formatRelativeTime(file.updatedAtMs)}</span>
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="empty-files">
                No file changes observed yet in this session. Edited or synced files will appear here.
              </div>
            )}
          </div>

          {/* OneDrive Storage Quota */}
          {quota ? (
            <div className="quota-section">
              <div className="quota-row">
                <span className="quota-title">Cloud Storage</span>
                <span className="quota-label tabular">
                  {quota.label || `${formatBytes(quota.usedBytes)} / ${formatBytes(quota.totalBytes)}`}
                </span>
              </div>
              <div className="progress-track quota-track">
                <div
                  className={`progress-fill ${quotaTone}`}
                  style={{ width: `${Math.max(2, Math.min(100, quota.usedPercent))}%` }}
                />
              </div>
            </div>
          ) : null}

          {actionError ? <div className="inline-feedback">{actionError}</div> : null}

          {/* Quick Actions Footer */}
          <div className="card-footer">
            <div className="footer-actions">
              <button
                type="button"
                className="btn-secondary"
                disabled={!activeAccount?.folderPath}
                aria-disabled={!activeAccount?.folderPath}
                title={
                  activeAccount?.folderPath
                    ? `Open ${activeAccount.folderPath}`
                    : "No OneDrive folder detected"
                }
                onClick={() => activeAccount && onOpenFolder(activeAccount.folderPath)}
              >
                <FolderIcon size={13} />
                <span>Open Folder</span>
              </button>

              {!snapshot.onedriveRunning ? (
                <button type="button" className="btn-primary" onClick={onLaunchOneDrive}>
                  Launch OneDrive
                </button>
              ) : primaryProviderCmd && activeAccount ? (
                <button
                  type="button"
                  className="btn-secondary"
                  disabled={busyActionId === primaryProviderCmd.id}
                  aria-disabled={busyActionId === primaryProviderCmd.id}
                  onClick={() => runProviderCommand(activeAccount.id, primaryProviderCmd.id)}
                  title={primaryProviderCmd.description || primaryProviderCmd.label}
                >
                  {busyActionId === primaryProviderCmd.id ? "Working..." : primaryProviderCmd.label}
                </button>
              ) : (
                <button type="button" className="btn-secondary" onClick={onOpenOneDriveFlyout}>
                  Open OneDrive
                </button>
              )}
            </div>

            <span className="hotkey-hint" title="Global show/hide shortcut">
              {settings.hotkey}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
