import { createQuotaWindow, type UsageSnapshot } from "./model";

export function buildDemoSnapshot(now = Date.now()): UsageSnapshot {
  return {
    source: "demo",
    capturedAtMs: now,
    planType: "Pro",
    creditsBalance: null,
    windows: [
      createQuotaWindow("primary", "短周期", {
        usedPercent: 27,
        resetsAtUnixSeconds: Math.floor(
          (now + 2.6 * 60 * 60 * 1_000) / 1_000,
        ),
        windowDurationMins: 300,
      }),
      createQuotaWindow("secondary", "长周期", {
        usedPercent: 59,
        resetsAtUnixSeconds: Math.floor(
          (now + 3.3 * 24 * 60 * 60 * 1_000) / 1_000,
        ),
        windowDurationMins: 10_080,
      }),
    ],
  };
}
