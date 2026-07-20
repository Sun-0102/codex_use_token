export type UsageSource = "demo" | "codex" | "stale";

export interface RawRateLimitWindow {
  usedPercent: number;
  resetsAtUnixSeconds: number | null;
  windowDurationMins: number | null;
}

export interface QuotaWindow {
  id: "primary" | "secondary";
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetsAtUnixSeconds: number | null;
  windowDurationMins: number | null;
}

export interface UsageSnapshot {
  source: UsageSource;
  capturedAtMs: number;
  planType: string | null;
  creditsBalance: string | null;
  windows: QuotaWindow[];
}

const clampPercent = (value: number) => Math.min(100, Math.max(0, value));

export function createQuotaWindow(
  id: QuotaWindow["id"],
  label: string,
  raw: RawRateLimitWindow,
): QuotaWindow {
  const usedPercent = clampPercent(Math.round(raw.usedPercent));

  return {
    id,
    label,
    usedPercent,
    remainingPercent: 100 - usedPercent,
    resetsAtUnixSeconds: raw.resetsAtUnixSeconds,
    windowDurationMins: raw.windowDurationMins,
  };
}

export function markSnapshotStale(snapshot: UsageSnapshot): UsageSnapshot {
  return {
    ...snapshot,
    source: "stale",
    windows: snapshot.windows.map((window) => ({ ...window })),
  };
}

export function formatDuration(minutes: number | null): string {
  if (minutes === null) return "等待同步";
  if (minutes % 10_080 === 0) return `${minutes / 10_080} 周窗口`;
  if (minutes % 60 === 0) return `${minutes / 60} 小时窗口`;
  return `${minutes} 分钟窗口`;
}

export function formatResetCountdown(
  resetsAtUnixSeconds: number | null,
  nowMs = Date.now(),
): string {
  if (resetsAtUnixSeconds === null) return "等待服务器同步";

  const remainingMs = resetsAtUnixSeconds * 1_000 - nowMs;
  if (remainingMs <= 0) return "正在重置";

  const totalMinutes = Math.ceil(remainingMs / 60_000);
  const days = Math.floor(totalMinutes / 1_440);
  const hours = Math.floor((totalMinutes % 1_440) / 60);
  const minutes = totalMinutes % 60;

  if (days > 0) return `${days} 天 ${hours} 小时后重置`;
  if (hours > 0) return `${hours} 小时 ${minutes} 分钟后重置`;
  return `${minutes} 分钟后重置`;
}
