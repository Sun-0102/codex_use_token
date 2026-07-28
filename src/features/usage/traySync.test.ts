import { describe, expect, it } from "vitest";
import { trayUsagePercentsFromSnapshot } from "./traySync";
import type { UsageSnapshot } from "./model";

describe("trayUsagePercentsFromSnapshot", () => {
  it("uses the same real snapshot as the detailed and compact windows", () => {
    const snapshot: UsageSnapshot = {
      source: "codex",
      capturedAtMs: 123,
      planType: "pro",
      creditsBalance: null,
      windows: [
        {
          id: "primary",
          label: "短周期",
          usedPercent: 27,
          remainingPercent: 73,
          resetsAtUnixSeconds: 1_784_548_800,
          windowDurationMins: 300,
        },
        {
          id: "secondary",
          label: "长周期",
          usedPercent: 59,
          remainingPercent: 41,
          resetsAtUnixSeconds: 1_784_822_400,
          windowDurationMins: 10_080,
        },
      ],
    };

    expect(trayUsagePercentsFromSnapshot(snapshot)).toEqual({
      weeklyRemainingPercent: 41,
    });
  });

  it("does not sync demo data to the tray as if it were live", () => {
    expect(
      trayUsagePercentsFromSnapshot({
        source: "demo",
        capturedAtMs: 123,
        planType: "Pro",
        creditsBalance: null,
        windows: [],
      }),
    ).toBeNull();
  });

  it("maps a lone weekly window to W instead of leaving the tray unchanged", () => {
    expect(
      trayUsagePercentsFromSnapshot({
        source: "codex",
        capturedAtMs: 123,
        planType: "prolite",
        creditsBalance: null,
        windows: [
          {
            id: "primary",
            label: "短周期",
            usedPercent: 25,
            remainingPercent: 75,
            resetsAtUnixSeconds: 1_784_822_400,
            windowDurationMins: 10_080,
          },
        ],
      }),
    ).toEqual({
      weeklyRemainingPercent: 75,
    });
  });

  it("does not send a five-hour-only snapshot to the tray", () => {
    expect(
      trayUsagePercentsFromSnapshot({
        source: "codex",
        capturedAtMs: 123,
        planType: "pro",
        creditsBalance: null,
        windows: [
          {
            id: "primary",
            label: "5 小时窗口",
            usedPercent: 10,
            remainingPercent: 90,
            resetsAtUnixSeconds: 1_784_548_800,
            windowDurationMins: 300,
          },
        ],
      }),
    ).toBeNull();
  });
});
