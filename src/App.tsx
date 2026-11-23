import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CompactPill } from "./components/CompactPill";
import { ExpandedPanel } from "./components/ExpandedPanel";
import type { AppSettings, HudSnapshot, SettingsPatch } from "./types";
import { resolveEffectiveTheme } from "./utils/format";

const DEFAULT_SETTINGS: AppSettings = {
  autoHideWhenIdle: true,
  idleHideDelaySecs: 6,
  hideInFullscreen: true,
  locked: false,
  opacity: 1.0,
  theme: "auto",
  startWithWindows: false,
  positionX: null,
  positionY: null,
  hotkey: "Ctrl+Alt+O",
};

const INITIAL_SNAPSHOT: HudSnapshot = {
  onedriveRunning: true,
  onedriveExePath: null,
  aggregateState: "unknown",
  aggregateLabel: "Checking OneDrive...",
  aggregateSummary: "Connecting to Windows Sync Provider",
  aggregateProgressPercent: null,
  totalSpeedBps: 0,
  uploadSpeedBps: 0,
  downloadSpeedBps: 0,
  totalFilesRemaining: 0,
  totalBytesRemaining: 0,
  totalEtaSeconds: null,
  speedHistoryBps: Array(30).fill(0),
  systemTheme: "light",
  isFullscreen: false,
  accounts: [],
  updatedAtMs: 0,
};

export function App() {
  const [snapshot, setSnapshot] = useState<HudSnapshot>(INITIAL_SNAPSHOT);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [expanded, setExpanded] = useState(false);
  const [expandedUpward, setExpandedUpward] = useState(true);
  const lastHeightRef = useRef<number>(456);

  useEffect(() => {
    let active = true;

    invoke<HudSnapshot>("get_snapshot")
      .then((snap) => {
        if (active) setSnapshot(snap);
      })
      .catch(() => {});

    invoke<AppSettings>("get_settings")
      .then((s) => {
        if (active) setSettings(s);
      })
      .catch(() => {});

    const unlistenPromises = [
      listen<HudSnapshot>("hud-snapshot", (event) => {
        if (active) setSnapshot(event.payload);
      }),
      listen<AppSettings>("hud-settings", (event) => {
        if (active) setSettings(event.payload);
      }),
      listen<boolean>("hud-expanded-changed", (event) => {
        if (active) setExpanded(event.payload);
      }),
    ];

    return () => {
      active = false;
      Promise.all(unlistenPromises).then((fns) => {
        fns.forEach((fn) => fn());
      });
    };
  }, []);

  // Apply resolved Windows Light/Dark theme attribute
  const effectiveTheme = resolveEffectiveTheme(settings.theme, snapshot.systemTheme);
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", effectiveTheme);
  }, [effectiveTheme]);

  // Collapse on Escape key for clean keyboard ergonomics
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && expanded) {
        setExpanded(false);
        invoke("set_expanded", { expanded: false, targetHeight: null }).catch(() => {});
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [expanded]);

  const handleExpand = useCallback(async () => {
    setExpanded(true);
    try {
      const upward = await invoke<boolean>("set_expanded", {
        expanded: true,
        targetHeight: lastHeightRef.current,
      });
      setExpandedUpward(upward);
    } catch {
      // ignore
    }
  }, []);

  const handleCollapse = useCallback(async () => {
    setExpanded(false);
    try {
      await invoke("set_expanded", { expanded: false, targetHeight: null });
    } catch {
      // ignore
    }
  }, []);

  const handleMeasureHeight = useCallback(
    (logicalHeight: number) => {
      if (!expanded) return;
      if (Math.abs(logicalHeight - lastHeightRef.current) < 6) return;
      lastHeightRef.current = logicalHeight;
      invoke<boolean>("set_expanded", {
        expanded: true,
        targetHeight: logicalHeight,
      })
        .then((upward) => setExpandedUpward(upward))
        .catch(() => {});
    },
    [expanded],
  );

  const handleStartDrag = useCallback(async () => {
    if (settings.locked) return;
    try {
      await getCurrentWindow().startDragging();
      await invoke("finish_drag");
    } catch {
      // ignore
    }
  }, [settings.locked]);

  const handleUpdateSettings = useCallback(async (patch: SettingsPatch) => {
    try {
      const updated = await invoke<AppSettings>("update_settings", { patch });
      setSettings(updated);
    } catch {
      // ignore
    }
  }, []);

  const handleDismiss = useCallback(() => {
    setExpanded(false);
    invoke("dismiss_hud").catch(() => {});
  }, []);

  const handleResetPosition = useCallback(() => {
    setExpanded(false);
    invoke("reset_position").catch(() => {});
  }, []);

  const handleOpenFolder = useCallback((folderPath: string) => {
    invoke("open_onedrive_folder", { folderPath }).catch(() => {});
  }, []);

  const handleRevealFile = useCallback((filePath: string) => {
    invoke("reveal_file_in_explorer", { filePath }).catch(() => {});
  }, []);

  const handleLaunchOneDrive = useCallback(() => {
    invoke("launch_onedrive").catch(() => {});
  }, []);

  const handleOpenOneDriveFlyout = useCallback(() => {
    invoke("open_onedrive_flyout").catch(() => {});
  }, []);

  const handleInvokeProviderCommand = useCallback(
    async (accountId: string, commandId: string) => {
      await invoke("invoke_onedrive_command", { accountId, commandId });
    },
    [],
  );

  const handleQuit = useCallback(() => {
    invoke("quit_app").catch(() => {});
  }, []);

  return (
    <main
      className="hud-stage"
      style={{ opacity: settings.opacity }}
      onMouseEnter={() => {
        invoke("set_hovered", { hovered: true }).catch(() => {});
      }}
      onMouseLeave={() => {
        invoke("set_hovered", { hovered: false }).catch(() => {});
      }}
    >
      {expanded ? (
        <ExpandedPanel
          snapshot={snapshot}
          settings={settings}
          expandedUpward={expandedUpward}
          onCollapse={handleCollapse}
          onStartDrag={handleStartDrag}
          onUpdateSettings={handleUpdateSettings}
          onDismiss={handleDismiss}
          onResetPosition={handleResetPosition}
          onOpenFolder={handleOpenFolder}
          onRevealFile={handleRevealFile}
          onLaunchOneDrive={handleLaunchOneDrive}
          onOpenOneDriveFlyout={handleOpenOneDriveFlyout}
          onInvokeProviderCommand={handleInvokeProviderCommand}
          onQuit={handleQuit}
          onMeasureHeight={handleMeasureHeight}
        />
      ) : (
        <CompactPill
          snapshot={snapshot}
          locked={settings.locked}
          onExpand={handleExpand}
          onStartDrag={handleStartDrag}
        />
      )}
    </main>
  );
}
