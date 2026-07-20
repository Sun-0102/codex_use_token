import type { UsageSnapshot } from "./model";

export interface TrayUsagePercents {
  primaryRemainingPercent: number | null;
  secondaryRemainingPercent: number | null;
}

export function trayUsagePercentsFromSnapshot(
  snapshot: UsageSnapshot,
): TrayUsagePercents | null {
  if (snapshot.source !== "codex") return null;

  const primary = snapshot.windows.find((window) => isPrimaryTrayWindow(window));
  const secondary = snapshot.windows.find((window) => isSecondaryTrayWindow(window));
  if (primary === undefined && secondary === undefined) return null;

  return {
    primaryRemainingPercent: primary?.remainingPercent ?? null,
    secondaryRemainingPercent: secondary?.remainingPercent ?? null,
  };
}

function isPrimaryTrayWindow(window: UsageSnapshot["windows"][number]): boolean {
  if (window.windowDurationMins !== null) return window.windowDurationMins === 300;
  return window.id === "primary";
}

function isSecondaryTrayWindow(window: UsageSnapshot["windows"][number]): boolean {
  if (window.windowDurationMins !== null) return window.windowDurationMins === 10_080;
  return window.id === "secondary";
}
