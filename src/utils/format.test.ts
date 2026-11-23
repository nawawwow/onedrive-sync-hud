import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatEta,
  formatRelativeTime,
  formatSpeed,
  resolveEffectiveTheme,
} from "./format";

describe("format utilities", () => {
  it("formats bytes cleanly across units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.50 KB");
    expect(formatBytes(12.4 * 1024 * 1024)).toBe("12.4 MB");
    expect(formatBytes(256 * 1024 * 1024 * 1024)).toBe("256 GB");
  });

  it("formats speed with /s suffix and floors tiny idle churn", () => {
    expect(formatSpeed(0)).toBe("0 KB/s");
    expect(formatSpeed(500)).toBe("0 KB/s");
    expect(formatSpeed(2.5 * 1024 * 1024)).toBe("2.50 MB/s");
  });

  it("formats ETA into compact human-readable spans", () => {
    expect(formatEta(null)).toBe("");
    expect(formatEta(0)).toBe("");
    expect(formatEta(42)).toBe("42s left");
    expect(formatEta(125)).toBe("2m 5s left");
    expect(formatEta(3660)).toBe("1h 1m left");
  });

  it("formats relative timestamps accurately", () => {
    const now = 1_000_000;
    expect(formatRelativeTime(now - 2_000, now)).toBe("just now");
    expect(formatRelativeTime(now - 35_000, now)).toBe("35s ago");
    expect(formatRelativeTime(now - 180_000, now)).toBe("3m ago");
  });

  it("resolves theme preference against Windows system theme", () => {
    expect(resolveEffectiveTheme("auto", "dark")).toBe("dark");
    expect(resolveEffectiveTheme("auto", "light")).toBe("light");
    expect(resolveEffectiveTheme("light", "dark")).toBe("light");
    expect(resolveEffectiveTheme("dark", "light")).toBe("dark");
  });
});
