import { describe, expect, it } from "vitest";
import {
  selectLowestQuotaWindow,
  selectWeeklyQuotaWindow,
} from "./UsageDashboard";
import type { QuotaWindow } from "./model";

describe("selectLowestQuotaWindow", () => {
  it("does not treat a missing secondary quota window as 0%", () => {
    const primary: QuotaWindow = {
      id: "primary",
      label: "1 周窗口",
      usedPercent: 25,
      remainingPercent: 75,
      resetsAtUnixSeconds: 1_784_548_800,
      windowDurationMins: 10_080,
    };

    expect(selectLowestQuotaWindow([primary])).toBe(primary);
  });

  it("identifies which visible quota window has the lowest remaining percent", () => {
    const primary: QuotaWindow = {
      id: "primary",
      label: "5 小时窗口",
      usedPercent: 100,
      remainingPercent: 0,
      resetsAtUnixSeconds: 1_784_548_800,
      windowDurationMins: 300,
    };
    const secondary: QuotaWindow = {
      id: "secondary",
      label: "1 周窗口",
      usedPercent: 25,
      remainingPercent: 75,
      resetsAtUnixSeconds: 1_784_822_400,
      windowDurationMins: 10_080,
    };

    expect(selectLowestQuotaWindow([primary, secondary])).toBe(primary);
  });
});

describe("selectWeeklyQuotaWindow", () => {
  it("keeps the weekly quota and omits the five-hour quota", () => {
    const primary: QuotaWindow = {
      id: "primary",
      label: "5 小时窗口",
      usedPercent: 59,
      remainingPercent: 41,
      resetsAtUnixSeconds: 1_784_548_800,
      windowDurationMins: 300,
    };
    const secondary: QuotaWindow = {
      id: "secondary",
      label: "1 周窗口",
      usedPercent: 27,
      remainingPercent: 73,
      resetsAtUnixSeconds: 1_784_822_400,
      windowDurationMins: 10_080,
    };

    expect(selectWeeklyQuotaWindow([primary, secondary])).toBe(secondary);
  });

  it("does not fall back to a five-hour quota when weekly data is missing", () => {
    const primary: QuotaWindow = {
      id: "primary",
      label: "5 小时窗口",
      usedPercent: 59,
      remainingPercent: 41,
      resetsAtUnixSeconds: 1_784_548_800,
      windowDurationMins: 300,
    };

    expect(selectWeeklyQuotaWindow([primary])).toBeNull();
  });

  it("recognizes a weekly-only window even when it arrives as primary", () => {
    const weeklyOnly: QuotaWindow = {
      id: "primary",
      label: "1 周窗口",
      usedPercent: 25,
      remainingPercent: 75,
      resetsAtUnixSeconds: 1_784_822_400,
      windowDurationMins: 10_080,
    };

    expect(selectWeeklyQuotaWindow([weeklyOnly])).toBe(weeklyOnly);
  });
});
