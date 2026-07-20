import type {
  CcSwitchUsageStatus,
  CodexUsageStatus,
} from "../../platform/runtime";

export interface TokenUsagePresentation {
  dailyLabel: string;
  todayTokens: string;
  totalLabel: string;
  lifetimeTokens: string;
  peakDailyTokens: string;
  trendDetail: string;
  isReal: boolean;
}

export function presentTokenUsage(
  status: CodexUsageStatus | null,
  ccSwitchStatus: CcSwitchUsageStatus | null = null,
  now = new Date(),
): TokenUsagePresentation {
  if (ccSwitchStatus?.state === "available" && ccSwitchStatus.today !== null) {
    const today = ccSwitchStatus.today;

    return {
      dailyLabel: "今日 Token",
      todayTokens: formatTokenCount(today.totalTokens),
      totalLabel: "缓存命中",
      lifetimeTokens: formatTokenCount(today.cacheReadTokens),
      peakDailyTokens: `新增 ${formatTokenCount(today.freshInputTokens)} · 输出 ${formatTokenCount(today.outputTokens)}`,
      trendDetail: `${new Intl.NumberFormat("zh-CN").format(today.requestCount)} 请求 · ${
        ccSwitchStatus.isStale ? "过期缓存" : "实时统计"
      }`,
      isReal: true,
    };
  }

  if (status === null || status.state !== "available") {
    return {
      dailyLabel: "账户日桶",
      todayTokens: "—",
      totalLabel: "账户累计",
      lifetimeTokens: "—",
      peakDailyTokens: "等待汇总",
      trendDetail: status?.state === "unavailable" ? status.message : "等待账户同步",
      isReal: false,
    };
  }

  const today = formatDateKey(now);
  const todayBucket =
    status.dailyUsageBuckets.find((bucket) => bucket.startDate === today) ??
    status.dailyUsageBuckets[status.dailyUsageBuckets.length - 1] ??
    null;
  const bucketCount = status.dailyUsageBuckets.length;

  return {
    dailyLabel:
      todayBucket === null || todayBucket.startDate === today
        ? "账户日桶"
        : "最近日桶",
    todayTokens:
      todayBucket === null ? "—" : formatTokenCount(todayBucket.tokens),
    totalLabel: "账户累计",
    lifetimeTokens: formatOptionalTokenCount(status.summary?.lifetimeTokens),
    peakDailyTokens: `日峰值 ${formatOptionalTokenCount(status.summary?.peakDailyTokens)}`,
    trendDetail:
      status.isStale
        ? `${bucketCount === 0 ? "account/usage" : `${bucketCount} 天桶`} · 过期缓存`
        : bucketCount === 0
          ? "account/usage · 非统计页"
          : `${bucketCount} 天桶 · 非统计页`,
    isReal: true,
  };
}

export function formatTokenCount(value: number): string {
  const absoluteValue = Math.abs(value);
  if (absoluteValue >= 100_000_000) {
    return `${formatCompactNumber(value / 100_000_000)}亿`;
  }

  if (absoluteValue >= 10_000) {
    return `${formatCompactNumber(value / 10_000)}万`;
  }

  return new Intl.NumberFormat("zh-CN").format(value);
}

function formatOptionalTokenCount(value: number | null | undefined): string {
  return value === null || value === undefined ? "—" : formatTokenCount(value);
}

function formatDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");

  return `${year}-${month}-${day}`;
}

function formatCompactNumber(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? rounded.toFixed(0) : rounded.toFixed(1);
}
