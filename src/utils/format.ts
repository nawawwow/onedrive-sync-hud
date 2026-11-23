import type { SyncState } from "../types";

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIdx = 0;
  while (value >= 1024 && unitIdx < units.length - 1) {
    value /= 1024;
    unitIdx += 1;
  }
  if (unitIdx === 0) {
    return `${Math.round(value)} B`;
  }
  const decimals = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(decimals)} ${units[unitIdx]}`;
}

export function formatSpeed(bps: number): string {
  if (!Number.isFinite(bps) || bps < 1024) {
    return "0 KB/s";
  }
  return `${formatBytes(bps)}/s`;
}

export function formatEta(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds <= 0) {
    return "";
  }
  if (seconds < 60) {
    return `${Math.max(1, Math.round(seconds))}s left`;
  }
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (mins < 60) {
    return secs > 0 ? `${mins}m ${secs}s left` : `${mins}m left`;
  }
  const hrs = Math.floor(mins / 60);
  const remMins = mins % 60;
  return remMins > 0 ? `${hrs}h ${remMins}m left` : `${hrs}h left`;
}

export function formatRelativeTime(timestampMs: number, nowMs: number = Date.now()): string {
  const diffSec = Math.max(0, Math.floor((nowMs - timestampMs) / 1000));
  if (diffSec < 5) return "just now";
  if (diffSec < 60) return `${diffSec}s ago`;
  const mins = Math.floor(diffSec / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ago`;
}

export function resolveEffectiveTheme(
  settingTheme: "auto" | "light" | "dark",
  systemTheme: string,
): "light" | "dark" {
  if (settingTheme === "light" || settingTheme === "dark") {
    return settingTheme;
  }
  return systemTheme === "dark" ? "dark" : "light";
}

export function stateToneClass(state: SyncState): string {
  switch (state) {
    case "syncing":
      return "tone-syncing";
    case "in_sync":
      return "tone-synced";
    case "paused":
    case "warning":
      return "tone-warning";
    case "error":
      return "tone-error";
    default:
      return "tone-neutral";
  }
}

export function isAttentionState(state: SyncState): boolean {
  return (
    state === "error" ||
    state === "warning" ||
    state === "paused" ||
    state === "offline" ||
    state === "not_running"
  );
}
