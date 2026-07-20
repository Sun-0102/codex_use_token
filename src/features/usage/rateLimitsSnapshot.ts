import type {
  CodexCreditsSnapshot,
  CodexRateLimitBucket,
  CodexRateLimitsStatus,
  CodexRateLimitWindow,
} from "../../platform/runtime";
import { createQuotaWindow, type UsageSnapshot } from "./model";

export function buildCodexSnapshotFromRateLimits(
  status: CodexRateLimitsStatus | null,
): UsageSnapshot | null {
  if (status === null || status.state !== "available") return null;

  const bucket = selectDisplayBucket(status.buckets);
  if (bucket === null) return null;

  const windows = [
    createWindow("primary", "短周期", bucket.primary),
    createWindow("secondary", "长周期", bucket.secondary),
  ].filter((window) => window !== null);

  if (windows.length === 0) return null;

  return {
    source: "codex",
    capturedAtMs: status.capturedAtMs,
    planType: bucket.planType ?? firstProvidedPlan(status.buckets),
    creditsBalance: formatCredits(bucket.credits),
    windows,
  };
}

function selectDisplayBucket(
  buckets: CodexRateLimitBucket[],
): CodexRateLimitBucket | null {
  return (
    buckets.find((bucket) => bucket.source === "default") ??
    buckets.find((bucket) => bucket.primary !== null || bucket.secondary !== null) ??
    null
  );
}

function createWindow(
  id: "primary" | "secondary",
  label: string,
  window: CodexRateLimitWindow | null,
) {
  if (window === null) return null;

  return createQuotaWindow(id, label, {
    usedPercent: window.usedPercent,
    resetsAtUnixSeconds: window.resetsAt,
    windowDurationMins: window.windowDurationMins,
  });
}

function firstProvidedPlan(buckets: CodexRateLimitBucket[]): string | null {
  return buckets.find((bucket) => bucket.planType !== null)?.planType ?? null;
}

function formatCredits(credits: CodexCreditsSnapshot | null): string | null {
  if (credits === null) return null;
  if (credits.unlimited) return "Credits Unlimited";
  if (!credits.hasCredits) return "无 Credits";
  return credits.balance === null ? "Credits 已启用" : `Credits ${credits.balance}`;
}
