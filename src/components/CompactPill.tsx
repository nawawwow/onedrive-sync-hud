import { useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { HudSnapshot } from "../types";
import { formatSpeed, isAttentionState, stateToneClass } from "../utils/format";
import { ChevronIcon, CloudStatusIcon } from "./Icons";

interface CompactPillProps {
  snapshot: HudSnapshot;
  locked: boolean;
  onExpand: () => void;
  onStartDrag: () => void;
}

export function CompactPill({
  snapshot,
  locked,
  onExpand,
  onStartDrag,
}: CompactPillProps) {
  const dragOrigin = useRef<{ x: number; y: number; dragged: boolean } | null>(null);

  const state = snapshot.aggregateState;
  const tone = stateToneClass(state);
  const isSyncing = state === "syncing";
  const isIndeterminate = isSyncing && snapshot.aggregateProgressPercent === null;
  const barWidth =
    state === "in_sync"
      ? 100
      : snapshot.aggregateProgressPercent !== null
        ? Math.max(4, Math.min(100, snapshot.aggregateProgressPercent))
        : 0;

  const rightMetric = (() => {
    if (isSyncing) {
      if (snapshot.aggregateProgressPercent !== null) {
        return `${Math.round(snapshot.aggregateProgressPercent)}%`;
      }
      if (snapshot.totalSpeedBps >= 1024) {
        return formatSpeed(snapshot.totalSpeedBps);
      }
      if (snapshot.totalFilesRemaining > 0) {
        return `${snapshot.totalFilesRemaining} left`;
      }
      return "Active";
    }
    if (state === "in_sync") {
      return "100%";
    }
    if (state === "paused") {
      return "Paused";
    }
    if (state === "not_running") {
      return "Off";
    }
    if (state === "error") {
      return "Error";
    }
    if (state === "warning") {
      return "Check";
    }
    return "";
  })();

  const primaryLine = (() => {
    if (isSyncing && snapshot.totalSpeedBps >= 1024 && snapshot.aggregateProgressPercent !== null) {
      return `${snapshot.aggregateLabel} · ${formatSpeed(snapshot.totalSpeedBps)}`;
    }
    if (snapshot.aggregateSummary && snapshot.aggregateSummary !== snapshot.aggregateLabel) {
      return snapshot.aggregateSummary;
    }
    return snapshot.aggregateLabel;
  })();

  const handlePointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    dragOrigin.current = { x: e.clientX, y: e.clientY, dragged: false };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const origin = dragOrigin.current;
    if (!origin || origin.dragged || locked) return;
    const dx = Math.abs(e.clientX - origin.x);
    const dy = Math.abs(e.clientY - origin.y);
    if (dx >= 5 || dy >= 5) {
      origin.dragged = true;
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {
        // Ignore if capture was already released
      }
      onStartDrag();
    }
  };

  const handlePointerUp = (e: ReactPointerEvent<HTMLDivElement>) => {
    const origin = dragOrigin.current;
    dragOrigin.current = null;
    try {
      if (e.currentTarget.hasPointerCapture(e.pointerId)) {
        e.currentTarget.releasePointerCapture(e.pointerId);
      }
    } catch {
      // Ignore capture cleanup error
    }
    if (origin && !origin.dragged) {
      onExpand();
    }
  };

  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={`OneDrive ${snapshot.aggregateLabel}. Click to expand details${locked ? "" : ", drag to move"}`}
      className={`fluent-pill ${tone} ${isAttentionState(state) ? "pill-attention" : ""} ${locked ? "is-locked" : ""}`}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onExpand();
        }
      }}
    >
      <div className={`status-orb ${tone}`}>
        <CloudStatusIcon state={state} size={16} />
      </div>

      <div className="pill-body">
        <div className="pill-top-row">
          <span className="pill-title" title={primaryLine}>
            {primaryLine}
          </span>
          {rightMetric ? <span className="pill-metric tabular">{rightMetric}</span> : null}
        </div>

        <div
          className="progress-track pill-progress"
          role="progressbar"
          aria-valuenow={snapshot.aggregateProgressPercent !== null ? Math.round(barWidth) : undefined}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          {isIndeterminate ? (
            <div className={`progress-indeterminate ${tone}`} />
          ) : (
            <div
              className={`progress-fill ${tone} ${isSyncing ? "progress-shimmer" : ""}`}
              style={{ width: `${barWidth}%` }}
            />
          )}
        </div>
      </div>

      <span className="pill-chevron" aria-hidden="true">
        <ChevronIcon size={13} rotated={true} />
      </span>
    </div>
  );
}
