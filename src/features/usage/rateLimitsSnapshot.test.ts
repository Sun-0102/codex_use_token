import { describe, expect, it } from "vitest";
import { buildCodexSnapshotFromRateLimits } from "./rateLimitsSnapshot";

describe("buildCodexSnapshotFromRateLimits", () => {
  it("converts real used percentages into remaining quota windows", () => {
    const snapshot = buildCodexSnapshotFromRateLimits({
      state: "available",
      capturedAtMs: 123,
      message: "已读取 1 个真实限额桶",
      buckets: [
        {
          source: "default",
          key: null,
          limitId: "codex",
          limitName: "Codex",
          planType: "pro",
          primary: {
            usedPercent: 27,
            windowDurationMins: 300,
            resetsAt: 1_784_548_800,
          },
          secondary: {
            usedPercent: 59,
            windowDurationMins: 10_080,
            resetsAt: 1_784_822_400,
          },
          credits: {
            hasCredits: true,
            unlimited: false,
            balance: "12.50",
          },
        },
      ],
    });

    expect(snapshot).toMatchObject({
      source: "codex",
      capturedAtMs: 123,
      planType: "pro",
      creditsBalance: "Credits 12.50",
    });
    expect(snapshot?.windows).toEqual([
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
    ]);
  });

  it("falls back to demo when no real quota window is available", () => {
    expect(
      buildCodexSnapshotFromRateLimits({
        state: "available",
        capturedAtMs: 456,
        message: "稀疏响应",
        buckets: [
          {
            source: "default",
            key: null,
            limitId: "codex",
            limitName: null,
            planType: null,
            primary: null,
            secondary: null,
            credits: null,
          },
        ],
      }),
    ).toBeNull();
  });
});
