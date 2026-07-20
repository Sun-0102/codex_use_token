import { useEffect, useMemo, useState } from "react";
import { UsageDashboard } from "./features/usage/UsageDashboard";
import { buildDemoSnapshot } from "./features/usage/demoSnapshot";
import { selectDisplaySnapshot } from "./features/usage/displaySnapshot";
import type { UsageSnapshot } from "./features/usage/model";
import { buildCodexSnapshotFromRateLimits } from "./features/usage/rateLimitsSnapshot";
import { trayUsagePercentsFromSnapshot } from "./features/usage/traySync";
import {
  hideUsageWindow,
  readCodexAccountStatus,
  readCodexCliStatus,
  readCodexRateLimitsStatus,
  readCodexThreadTokenUsageStatus,
  readCodexUsageStatus,
  readCcSwitchUsageStatus,
  setUsageWindowMode,
  updateTrayUsage,
  type CcSwitchUsageStatus,
  type CodexAccountStatus,
  type CodexCliStatus,
  type CodexRateLimitsStatus,
  type CodexThreadTokenUsageStatus,
  type CodexUsageStatus,
  type UsageWindowMode,
} from "./platform/runtime";
import "./App.css";

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
  const [ccSwitchUsageStatus, setCcSwitchUsageStatus] =
    useState<CcSwitchUsageStatus | null>(null);
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

    void readCodexCliStatus()
      .then((status) => {
        if (isMounted) setCliStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setCliStatus({
            state: "launchFailed",
            executablePath: null,
            version: null,
            message: "无法调用本机 CLI 探测服务",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    const trayUsage = trayUsagePercentsFromSnapshot(snapshot);
    if (trayUsage === null) return;

    void updateTrayUsage(
      trayUsage.primaryRemainingPercent,
      trayUsage.secondaryRemainingPercent,
    ).catch(() => undefined);
  }, [snapshot]);

  useEffect(() => {
    let isMounted = true;

    void readCodexThreadTokenUsageStatus()
      .then((status) => {
        if (isMounted) setThreadTokenUsageStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setThreadTokenUsageStatus({
            state: "unavailable",
            capturedAtMs: 0,
            usage: null,
            message: "无法调用本机 Codex 线程 Token 通知服务",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    void readCcSwitchUsageStatus()
      .then((status) => {
        if (isMounted) setCcSwitchUsageStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setCcSwitchUsageStatus({
            state: "unavailable",
            capturedAtMs: 0,
            today: null,
            message: "无法读取 cc-switch 今日 Token 统计",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    void readCodexUsageStatus()
      .then((status) => {
        if (isMounted) setUsageStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setUsageStatus({
            state: "unavailable",
            capturedAtMs: 0,
            summary: null,
            dailyUsageBuckets: [],
            message: "无法调用本机 Codex Token 用量读取服务",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    void readCodexRateLimitsStatus()
      .then((status) => {
        if (isMounted) setRateLimitsStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setRateLimitsStatus({
            state: "unavailable",
            capturedAtMs: 0,
            buckets: [],
            message: "无法调用本机 Codex 限额读取服务",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    void readCodexAccountStatus()
      .then((status) => {
        if (isMounted) setAccountStatus(status);
      })
      .catch(() => {
        if (isMounted) {
          setAccountStatus({
            state: "unavailable",
            planType: null,
            accountType: null,
            capturedAtMs: 0,
            message: "无法调用本机 Codex 账户读取服务",
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

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
      ccSwitchUsageStatus={ccSwitchUsageStatus}
      threadTokenUsageStatus={threadTokenUsageStatus}
      mode={windowMode}
      canCollapse={supportsCompactMode}
      onModeChange={changeWindowMode}
      onHide={() => void hideUsageWindow().catch(() => undefined)}
    />
  );
}

export default App;
