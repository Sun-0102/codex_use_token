import type { CodexRateLimitsStatus } from "../../platform/runtime";
import { markSnapshotStale, type UsageSnapshot } from "./model";

interface SelectDisplaySnapshotInput {
  demoSnapshot: UsageSnapshot;
  liveSnapshot: UsageSnapshot | null;
  lastLiveSnapshot: UsageSnapshot | null;
  rateLimitsStatus: CodexRateLimitsStatus | null;
}

export function selectDisplaySnapshot({
  demoSnapshot,
  liveSnapshot,
  lastLiveSnapshot,
  rateLimitsStatus,
}: SelectDisplaySnapshotInput): UsageSnapshot {
  if (liveSnapshot !== null) return liveSnapshot;

  if (rateLimitsStatus?.state === "unavailable" && lastLiveSnapshot !== null) {
    return markSnapshotStale(lastLiveSnapshot);
  }

  return demoSnapshot;
}
