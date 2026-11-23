import type { SyncState } from "../types";

interface IconProps {
  size?: number;
  className?: string;
}

export function CloudStatusIcon({
  state,
  size = 18,
}: {
  state: SyncState;
  size?: number;
}) {
  if (state === "syncing") {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 20 20"
        fill="none"
        aria-hidden="true"
        className="icon-spin-slow"
      >
        <path
          d="M16.2 10a6.2 6.2 0 0 1-10.6 4.38"
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
        />
        <path
          d="M3.8 10a6.2 6.2 0 0 1 10.6-4.38"
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
          strokeOpacity="0.4"
        />
        <path
          d="M14.6 3.5v2.7h-2.7"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }

  if (state === "in_sync") {
    return (
      <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <path
          d="M5.2 10.4l3.1 3.1 6.5-6.8"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }

  if (state === "paused") {
    return (
      <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <rect x="5.5" y="4.5" width="3" height="11" rx="1.2" fill="currentColor" />
        <rect x="11.5" y="4.5" width="3" height="11" rx="1.2" fill="currentColor" />
      </svg>
    );
  }

  if (state === "error" || state === "warning") {
    return (
      <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <path
          d="M10 3.2L17.2 16a1 1 0 0 1-.87 1.5H3.67A1 1 0 0 1 2.8 16L10 3.2z"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
        <path d="M10 8v3.8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <circle cx="10" cy="14.3" r="0.95" fill="currentColor" />
      </svg>
    );
  }

  // Offline / Not running / Unknown cloud glyph
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <path
        d="M6.4 15h7.6a3.5 3.5 0 0 0 .55-6.96A4.7 4.7 0 0 0 5.6 7.3a3.2 3.2 0 0 0 .8 7.7z"
        stroke="currentColor"
        strokeWidth="1.65"
        strokeLinejoin="round"
      />
      {state === "not_running" || state === "offline" ? (
        <path
          d="M3.5 16.5L16.5 3.5"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      ) : null}
    </svg>
  );
}

export function ChevronIcon({
  size = 14,
  rotated = false,
}: IconProps & { rotated?: boolean }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      aria-hidden="true"
      style={{
        transform: rotated ? "rotate(180deg)" : undefined,
        transition: "transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1)",
      }}
    >
      <path
        d="M4 6.2L8 10.2L12 6.2"
        stroke="currentColor"
        strokeWidth="1.65"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function PinIcon({ size = 14, pinned = false }: IconProps & { pinned?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M9.8 2.4l3.8 3.8-1.3 1.3-1-.4-2.5 2.5.4 1.6-1.1 1.1-2.2-2.2-2.8 2.8-1-1 2.8-2.8-2.2-2.2 1.1-1.1 1.6.4 2.5-2.5-.4-1 1.3-1.3z"
        fill={pinned ? "currentColor" : "none"}
        stroke="currentColor"
        strokeWidth="1.35"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function LockIcon({ size = 14, locked = false }: IconProps & { locked?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <rect
        x="3.5"
        y="7.2"
        width="9"
        height="6.4"
        rx="1.6"
        stroke="currentColor"
        strokeWidth="1.4"
        fill={locked ? "currentColor" : "none"}
        fillOpacity={locked ? 0.2 : 0}
      />
      {locked ? (
        <path
          d="M5.5 7.2V5.3a2.5 2.5 0 0 1 5 0v1.9"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinecap="round"
        />
      ) : (
        <path
          d="M5.5 7.2V5.2a2.5 2.5 0 0 1 4.7-1.2"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinecap="round"
        />
      )}
    </svg>
  );
}

export function SettingsIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <circle cx="8" cy="8" r="2.2" stroke="currentColor" strokeWidth="1.4" />
      <path
        d="M8 2.2v1.4M8 12.4v1.4M2.2 8h1.4M12.4 8h1.4M3.9 3.9l1 1M11.1 11.1l1 1M12.1 3.9l-1 1M4.9 11.1l-1 1"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function FolderIcon({ size = 14 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M2.2 4.4A1.4 1.4 0 0 1 3.6 3h2.8l1.3 1.4h4.7a1.4 1.4 0 0 1 1.4 1.4v5.8a1.4 1.4 0 0 1-1.4 1.4H3.6a1.4 1.4 0 0 1-1.4-1.4V4.4z"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function ArrowUpIcon({ size = 12 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M7 11V3M7 3L3.8 6.2M7 3l3.2 3.2"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function ArrowDownIcon({ size = 12 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M7 3v8M7 11L3.8 7.8M7 11l3.2-3.2"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function CloseIcon({ size = 13 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M3.5 3.5l7 7M10.5 3.5l-7 7"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function FileStateDot({
  state,
}: {
  state: "syncing" | "synced" | "pending" | "error" | "cloud_only";
}) {
  if (state === "syncing" || state === "pending") {
    return (
      <span
        className="file-badge file-badge-syncing"
        title={state === "syncing" ? "Syncing" : "Pending sync"}
        aria-label={state === "syncing" ? "Syncing" : "Pending sync"}
      >
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" className="icon-spin-slow">
          <path
            d="M13 8a5 5 0 0 1-8.5 3.5M3 8a5 5 0 0 1 8.5-3.5"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
          />
        </svg>
      </span>
    );
  }
  if (state === "error") {
    return (
      <span className="file-badge file-badge-error" title="Sync error" aria-label="Sync error">
        !
      </span>
    );
  }
  if (state === "cloud_only") {
    return (
      <span className="file-badge file-badge-cloud" title="Online-only" aria-label="Online-only">
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none">
          <path
            d="M4.8 11.8h6.4a2.6 2.6 0 0 0 .4-5.16 3.5 3.5 0 0 0-6.6-.54 2.4 2.4 0 0 0-.2 5.7z"
            stroke="currentColor"
            strokeWidth="1.5"
          />
        </svg>
      </span>
    );
  }
  return (
    <span className="file-badge file-badge-synced" title="Synced" aria-label="Synced">
      <svg width="11" height="11" viewBox="0 0 16 16" fill="none">
        <path
          d="M3.8 8.3l2.7 2.7 5.7-5.9"
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </span>
  );
}
