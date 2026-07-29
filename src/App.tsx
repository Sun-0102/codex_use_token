import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { UsageDashboard } from "./features/usage/UsageDashboard";
import { buildDemoSnapshot } from "./features/usage/demoSnapshot";
import { selectDisplaySnapshot } from "./features/usage/displaySnapshot";
import type { UsageSnapshot } from "./features/usage/model";
import { buildCodexSnapshotFromRateLimits } from "./features/usage/rateLimitsSnapshot";
import { trayUsagePercentsFromSnapshot } from "./features/usage/traySync";
import {
  mergeRefreshStatus,
  shareRefreshInFlight,
  startUsageRefreshLoop,
  USAGE_REFRESH_EVENT,
} from "./features/usage/usageRefresh";
import {
  hideUsageWindow,
  readCodexAccountStatus,
  readCodexCliStatus,
  readCodexRateLimitsStatus,
  readCodexThreadTokenUsageStatus,
  readCodexUsageStatus,
  readCodexSessionUsageStatus,
  setUsageWindowMode,
  startUsageWindowDragging,
  updateTrayUsage,
  type CodexSessionUsageStatus,
  type CodexAccountStatus,
  type CodexCliStatus,
  type CodexRateLimitsStatus,
  type CodexThreadTokenUsageStatus,
  type CodexUsageStatus,
  type UsageWindowMode,
} from "./platform/runtime";
import "./App.css";

interface UsageMonitorRound {
  cliStatus: CodexCliStatus;
  accountStatus: CodexAccountStatus;
  rateLimitsStatus: CodexRateLimitsStatus;
  usageStatus: CodexUsageStatus;
  sessionUsageStatus: CodexSessionUsageStatus;
  threadTokenUsageStatus: CodexThreadTokenUsageStatus;
}

async function readUsageMonitorRound(): Promise<UsageMonitorRound> {
  const capturedAtMs = Date.now();
  const [
    cliStatus,
    accountStatus,
    rateLimitsStatus,
    usageStatus,
    sessionUsageStatus,
    threadTokenUsageStatus,
  ] = await Promise.all([
    readCodexCliStatus().catch<CodexCliStatus>(() => ({
      state: "launchFailed",
      executablePath: null,
      version: null,
      message: "无法调用本机 CLI 探测服务",
    })),
    readCodexAccountStatus().catch<CodexAccountStatus>(() => ({
      state: "unavailable",
      planType: null,
      accountType: null,
      capturedAtMs,
      message: "无法调用本机 Codex 账户读取服务",
    })),
    readCodexRateLimitsStatus().catch<CodexRateLimitsStatus>(() => ({
      state: "unavailable",
      capturedAtMs,
      buckets: [],
      message: "无法调用本机 Codex 限额读取服务",
    })),
    readCodexUsageStatus().catch<CodexUsageStatus>(() => ({
      state: "unavailable",
      capturedAtMs,
      summary: null,
      dailyUsageBuckets: [],
      message: "无法调用本机 Codex Token 用量读取服务",
    })),
    readCodexSessionUsageStatus().catch<CodexSessionUsageStatus>(() => ({
      state: "unavailable",
      capturedAtMs,
      today: null,
      message: "无法读取 Codex 本地会话 Token 统计",
    })),
    readCodexThreadTokenUsageStatus().catch<CodexThreadTokenUsageStatus>(
      () => ({
        state: "unavailable",
        capturedAtMs,
        usage: null,
        message: "无法调用本机 Codex 线程 Token 通知服务",
      }),
    ),
  ]);

  return {
    cliStatus,
    accountStatus,
    rateLimitsStatus,
    usageStatus,
    sessionUsageStatus,
    threadTokenUsageStatus,
  };
}

const readSharedUsageMonitorRound = shareRefreshInFlight(readUsageMonitorRound);

function App() {
  const demoSnapshot = useMemo(() => buildDemoSnapshot(), []);
  const [supportsCompactMode] = useState(() => window.innerHeight <= 100);
  const [windowMode, setWindowMode] = useState<UsageWindowMode>(() =>
    window.innerHeight <= 100 ? "compact" : "detailed",
  );
  const [cliStatus, setCliStatus] = useState<CodexCliStatus | null>(null);
  const [accountStatus, setAccountStatus] =
    useState<CodexAccountStatus | null>(null);
  const [rateLimitsStatus, setRateLimitsStatus] =
    useState<CodexRateLimitsStatus | null>(null);
  const [usageStatus, setUsageStatus] = useState<CodexUsageStatus | null>(null);
  const [sessionUsageStatus, setSessionUsageStatus] =
    useState<CodexSessionUsageStatus | null>(null);
  const [threadTokenUsageStatus, setThreadTokenUsageStatus] =
    useState<CodexThreadTokenUsageStatus | null>(null);
  const [lastLiveSnapshot, setLastLiveSnapshot] =
    useState<UsageSnapshot | null>(null);
  const liveSnapshot = useMemo(
    () => buildCodexSnapshotFromRateLimits(rateLimitsStatus),
    [rateLimitsStatus],
  );
  const snapshot = selectDisplaySnapshot({
    demoSnapshot,
    liveSnapshot,
    lastLiveSnapshot,
    rateLimitsStatus,
  });

  useEffect(() => {
    if (liveSnapshot !== null) {
      setLastLiveSnapshot(liveSnapshot);
    }
  }, [liveSnapshot]);

  useEffect(() => {
    let isMounted = true;
    const stopRefreshLoop = startUsageRefreshLoop(async () => {
      const {
        cliStatus: nextCliStatus,
        accountStatus: nextAccountStatus,
        rateLimitsStatus: nextRateLimitsStatus,
        usageStatus: nextUsageStatus,
        sessionUsageStatus: nextSessionUsageStatus,
        threadTokenUsageStatus: nextThreadTokenUsageStatus,
      } = await readSharedUsageMonitorRound();

      if (!isMounted) return;

      setCliStatus((previous) =>
        mergeRefreshStatus(
          previous,
          nextCliStatus,
          (status) => status.state !== "launchFailed",
        ),
      );
      setAccountStatus((previous) =>
        mergeRefreshStatus(
          previous,
          nextAccountStatus,
          (status) => status.state !== "unavailable",
        ),
      );
      setRateLimitsStatus(nextRateLimitsStatus);
      setUsageStatus((previous) =>
        mergeRefreshStatus(
          previous,
          nextUsageStatus,
          (status) => status.state === "available",
        ),
      );
      setSessionUsageStatus((previous) =>
        mergeRefreshStatus(
          previous,
          nextSessionUsageStatus,
          (status) => status.state === "available" && status.today !== null,
        ),
      );
      setThreadTokenUsageStatus((previous) =>
        mergeRefreshStatus(
          previous,
          nextThreadTokenUsageStatus,
          (status) => status.state === "available" && status.usage !== null,
        ),
      );
    }, (onRefresh) => listen(USAGE_REFRESH_EVENT, onRefresh));

    return () => {
      isMounted = false;
      stopRefreshLoop();
    };
  }, []);

  useEffect(() => {
    const trayUsage = trayUsagePercentsFromSnapshot(snapshot);
    if (trayUsage === null) return;

    void updateTrayUsage(trayUsage.weeklyRemainingPercent).catch(() => undefined);
  }, [snapshot]);

  const changeWindowMode = (nextMode: UsageWindowMode) => {
    const previousMode = windowMode;
    setWindowMode(nextMode);
    void setUsageWindowMode(nextMode).catch(() => setWindowMode(previousMode));
  };

  return (
    <UsageDashboard
      snapshot={snapshot}
      cliStatus={cliStatus}
      accountStatus={accountStatus}
      usageStatus={usageStatus}
      sessionUsageStatus={sessionUsageStatus}
      threadTokenUsageStatus={threadTokenUsageStatus}
      mode={windowMode}
      canCollapse={supportsCompactMode}
      onModeChange={changeWindowMode}
      onStartDragging={startUsageWindowDragging}
      onHide={() => void hideUsageWindow().catch(() => undefined)}
    />
  );
}

export default App;
