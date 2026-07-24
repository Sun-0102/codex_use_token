import { describe, expect, it } from "vitest";
import { presentTokenUsage } from "./tokenUsage";

describe("presentTokenUsage", () => {
  it("prefers today's Codex session usage over account daily buckets", () => {
    expect(
      presentTokenUsage(
        {
          state: "available",
          capturedAtMs: 123,
          message: "已读取 1 个每日 Token 用量桶",
          summary: {
            lifetimeTokens: 363_885_618,
            peakDailyTokens: 131_841_925,
            longestRunningTurnSec: null,
            currentStreakDays: null,
            longestStreakDays: null,
          },
          dailyUsageBuckets: [{ startDate: "2026-07-19", tokens: 1_299_164 }],
        },
        {
          state: "available",
          capturedAtMs: 456,
          message: "已从 Codex 本地会话日志统计今日用量：1343 个请求",
          today: {
            requestCount: 1_343,
            inputTokens: 151_257_000,
            freshInputTokens: 5_619_000,
            outputTokens: 522_617,
            cacheReadTokens: 145_638_000,
            cacheCreationTokens: 0,
            totalTokens: 151_779_617,
          },
        },
        new Date(2026, 6, 20),
      ),
    ).toEqual({
      dailyLabel: "今日 Token",
      todayTokens: "1.5亿",
      totalLabel: "缓存命中",
      lifetimeTokens: "1.5亿",
      peakDailyTokens: "新增 561.9万 · 输出 52.3万",
      trendDetail: "1,343 请求 · 本地会话",
      isReal: true,
    });
  });

  it("shows today's real tokens, lifetime total, peak, and trend coverage", () => {
    expect(
      presentTokenUsage(
        {
          state: "available",
          capturedAtMs: 123,
          message: "已读取 2 个每日 Token 用量桶",
          summary: {
            lifetimeTokens: 1_234_567,
            peakDailyTokens: 98_765,
            longestRunningTurnSec: 321,
            currentStreakDays: 4,
            longestStreakDays: 11,
          },
          dailyUsageBuckets: [
            { startDate: "2026-07-19", tokens: 12_345 },
            { startDate: "2026-07-20", tokens: 23_456 },
          ],
        },
        null,
        new Date(2026, 6, 20),
      ),
    ).toEqual({
      dailyLabel: "账户日桶",
      todayTokens: "2.3万",
      totalLabel: "账户累计",
      lifetimeTokens: "123.5万",
      peakDailyTokens: "日峰值 9.9万",
      trendDetail: "2 天桶 · 非统计页",
      isReal: true,
    });
  });

  it("labels retained Codex session values as stale after a refresh failure", () => {
    expect(
      presentTokenUsage(null, {
        state: "available",
        capturedAtMs: 456,
        message: "无法读取 Codex 本地会话 Token 统计",
        isStale: true,
        today: {
          requestCount: 12,
          inputTokens: 10_000,
          freshInputTokens: 2_000,
          outputTokens: 500,
          cacheReadTokens: 8_000,
          cacheCreationTokens: 0,
          totalTokens: 10_500,
        },
      }),
    ).toMatchObject({
      todayTokens: "1.1万",
      trendDetail: "12 请求 · 过期缓存",
      isReal: true,
    });
  });

  it("labels retained account usage values as stale after a refresh failure", () => {
    expect(
      presentTokenUsage({
        state: "available",
        capturedAtMs: 123,
        message: "读取 Codex Token 用量超时",
        isStale: true,
        summary: {
          lifetimeTokens: 20_000,
          peakDailyTokens: 10_000,
          longestRunningTurnSec: null,
          currentStreakDays: null,
          longestStreakDays: null,
        },
        dailyUsageBuckets: [{ startDate: "2026-07-20", tokens: 10_000 }],
      }),
    ).toMatchObject({
      todayTokens: "1万",
      lifetimeTokens: "2万",
      trendDetail: "1 天桶 · 过期缓存",
      isReal: true,
    });
  });

  it("labels fallback buckets as the latest account bucket, not today", () => {
    expect(
      presentTokenUsage(
        {
          state: "available",
          capturedAtMs: 123,
          message: "已读取 1 个每日 Token 用量桶",
          summary: {
            lifetimeTokens: null,
            peakDailyTokens: null,
            longestRunningTurnSec: null,
            currentStreakDays: null,
            longestStreakDays: null,
          },
          dailyUsageBuckets: [{ startDate: "2026-07-19", tokens: 12_345 }],
        },
        null,
        new Date(2026, 6, 20),
      ),
    ).toMatchObject({
      dailyLabel: "最近日桶",
      todayTokens: "1.2万",
      trendDetail: "1 天桶 · 非统计页",
    });
  });

  it("compacts very large token totals for small cards", () => {
    expect(
      presentTokenUsage({
        state: "available",
        capturedAtMs: 123,
        message: "已读取 1 个每日 Token 用量桶",
        summary: {
          lifetimeTokens: 363_885_618,
          peakDailyTokens: 131_841_925,
          longestRunningTurnSec: null,
          currentStreakDays: null,
          longestStreakDays: null,
        },
        dailyUsageBuckets: [{ startDate: "2026-07-20", tokens: 23_456 }],
      }),
    ).toMatchObject({
      lifetimeTokens: "3.6亿",
      peakDailyTokens: "日峰值 1.3亿",
    });
  });

  it("keeps unavailable usage separate from live quota data", () => {
    expect(
      presentTokenUsage({
        state: "unavailable",
        capturedAtMs: 0,
        summary: null,
        dailyUsageBuckets: [],
        message: "读取 Codex Token 用量超时",
      }),
    ).toMatchObject({
      dailyLabel: "账户日桶",
      todayTokens: "—",
      totalLabel: "账户累计",
      lifetimeTokens: "—",
      peakDailyTokens: "等待汇总",
      trendDetail: "读取 Codex Token 用量超时",
      isReal: false,
    });
  });
});
