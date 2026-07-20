import { describe, expect, it } from "vitest";
import { selectDisplaySnapshot } from "./displaySnapshot";
import type { UsageSnapshot } from "./model";

const demoSnapshot: UsageSnapshot = {
  source: "demo",
  capturedAtMs: 1,
  planType: "Pro",
  creditsBalance: null,
  windows: [],
};

const liveSnapshot: UsageSnapshot = {
  source: "codex",
  capturedAtMs: 2,
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
};

describe("selectDisplaySnapshot", () => {
  it("removes demo values when a real snapshot is available", () => {
    expect(
      selectDisplaySnapshot({
        demoSnapshot,
        liveSnapshot,
        lastLiveSnapshot: null,
        rateLimitsStatus: null,
      }),
    ).toBe(liveSnapshot);
  });

  it("keeps the last real snapshot as stale when the connection later fails", () => {
    expect(
      selectDisplaySnapshot({
        demoSnapshot,
        liveSnapshot: null,
        lastLiveSnapshot: liveSnapshot,
        rateLimitsStatus: {
          state: "unavailable",
          capturedAtMs: 3,
          buckets: [],
          message: "读取 Codex 限额超时",
        },
      }),
    ).toMatchObject({
      source: "stale",
      capturedAtMs: 2,
      windows: [{ remainingPercent: 73 }],
    });
  });

  it("does not pretend a failed first connection is real-time data", () => {
    expect(
      selectDisplaySnapshot({
        demoSnapshot,
        liveSnapshot: null,
        lastLiveSnapshot: null,
        rateLimitsStatus: {
          state: "unavailable",
          capturedAtMs: 3,
          buckets: [],
          message: "读取 Codex 限额超时",
        },
      }),
    ).toBe(demoSnapshot);
  });
});
