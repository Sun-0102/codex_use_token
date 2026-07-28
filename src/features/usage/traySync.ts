import type { UsageSnapshot } from "./model";

export interface TrayUsagePercents {
  weeklyRemainingPercent: number;
}

export function trayUsagePercentsFromSnapshot(
  snapshot: UsageSnapshot,
): TrayUsagePercents | null {
  if (snapshot.source !== "codex") return null;

  const weekly = snapshot.windows.find((window) => isWeeklyTrayWindow(window));
  if (weekly === undefined) return null;

  return {
    weeklyRemainingPercent: weekly.remainingPercent,
  };
}

function isWeeklyTrayWindow(window: UsageSnapshot["windows"][number]): boolean {
  if (window.windowDurationMins !== null) return window.windowDurationMins === 10_080;
  return window.id === "secondary";
}
