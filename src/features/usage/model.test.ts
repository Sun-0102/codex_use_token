import { describe, expect, it } from "vitest";
import {
  createQuotaWindow,
  formatDuration,
  formatResetCountdown,
  markSnapshotStale,
} from "./model";

describe("createQuotaWindow", () => {
  it("converts Codex used percentage into remaining percentage", () => {
    const window = createQuotaWindow("primary", "5 小时", {
      usedPercent: 27,
      resetsAtUnixSeconds: 1_800_000_000,
      windowDurationMins: 300,
    });

    expect(window.remainingPercent).toBe(73);
    expect(window.usedPercent).toBe(27);
  });

  it("clamps malformed percentages to a safe display range", () => {
    expect(
      createQuotaWindow("primary", "primary", {
        usedPercent: 140,
        resetsAtUnixSeconds: null,
        windowDurationMins: null,
      }).remainingPercent,
    ).toBe(0);

    expect(
      createQuotaWindow("secondary", "secondary", {
        usedPercent: -4,
        resetsAtUnixSeconds: null,
        windowDurationMins: null,
      }).remainingPercent,
    ).toBe(100);
  });
});

describe("markSnapshotStale", () => {
  it("keeps the last real sample while making the stale source explicit", () => {
    expect(
      markSnapshotStale({
        source: "codex",
        capturedAtMs: 123,
        planType: "pro",
        creditsBalance: "Credits 12.50",
        windows: [
          {
            id: "primary",
            label: "短周期",
            usedPercent: 27,
            remainingPercent: 73,
            resetsAtUnixSeconds: 1_784_548_800,
            windowDurationMins: 300,
          },
        ],
      }),
    ).toMatchObject({
      source: "stale",
      capturedAtMs: 123,
      planType: "pro",
      creditsBalance: "Credits 12.50",
      windows: [{ remainingPercent: 73 }],
    });
  });
});

describe("quota formatting", () => {
  it("formats well-known quota durations", () => {
    expect(formatDuration(300)).toBe("5 小时窗口");
    expect(formatDuration(10_080)).toBe("1 周窗口");
  });

  it("formats reset countdowns from Unix seconds", () => {
    const now = Date.UTC(2026, 6, 20, 0, 0, 0);
    const reset = Date.UTC(2026, 6, 22, 3, 0, 0) / 1_000;

    expect(formatResetCountdown(reset, now)).toBe("2 天 3 小时后重置");
    expect(formatResetCountdown(null, now)).toBe("等待服务器同步");
  });
});
