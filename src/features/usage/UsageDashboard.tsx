import {
  useRef,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import type {
  CodexAccountStatus,
  CodexCliStatus,
  CodexSessionUsageStatus,
  CodexThreadTokenUsageStatus,
  CodexUsageStatus,
  UsageWindowMode,
} from "../../platform/runtime";
import { presentCliStatus } from "./cliStatus";
import { shouldStartCompactDrag } from "./compactDrag";
import {
  formatDuration,
  formatResetCountdown,
  type QuotaWindow,
  type UsageSnapshot,
} from "./model";
import { presentTokenUsage } from "./tokenUsage";
import { presentThreadTokenUsage } from "./threadTokenUsage";

interface UsageDashboardProps {
  snapshot: UsageSnapshot;
  cliStatus: CodexCliStatus | null;
  accountStatus: CodexAccountStatus | null;
  usageStatus: CodexUsageStatus | null;
  sessionUsageStatus: CodexSessionUsageStatus | null;
  threadTokenUsageStatus: CodexThreadTokenUsageStatus | null;
  mode: UsageWindowMode;
  canCollapse: boolean;
  onModeChange: (mode: UsageWindowMode) => void;
  onStartDragging: () => Promise<void>;
  onHide: () => void;
}

type GaugeStyle = CSSProperties & {
  "--remaining": string;
};

function CompactWidget({
  snapshot,
  onExpand,
  onStartDragging,
}: {
  snapshot: UsageSnapshot;
  onExpand: () => void;
  onStartDragging: () => Promise<void>;
}) {
  const isLive = snapshot.source === "codex";
  const isStale = snapshot.source === "stale";
  const weeklyWindow = selectWeeklyQuotaWindow(snapshot.windows);
  const sourceLabel = isLive ? "实时" : isStale ? "缓存" : "演示";
  const pointerGesture = useRef<{
    pointerId: number;
    originX: number;
    originY: number;
    didDrag: boolean;
  } | null>(null);

  const handlePointerDown = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;

    pointerGesture.current = {
      pointerId: event.pointerId,
      originX: event.clientX,
      originY: event.clientY,
      didDrag: false,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove = (event: PointerEvent<HTMLButtonElement>) => {
    const gesture = pointerGesture.current;
    if (
      gesture === null ||
      gesture.pointerId !== event.pointerId ||
      gesture.didDrag ||
      (event.buttons & 1) === 0 ||
      !shouldStartCompactDrag(
        { x: gesture.originX, y: gesture.originY },
        { x: event.clientX, y: event.clientY },
      )
    ) {
      return;
    }

    gesture.didDrag = true;
    void onStartDragging().catch(() => undefined);
  };

  const handlePointerUp = (event: PointerEvent<HTMLButtonElement>) => {
    const gesture = pointerGesture.current;
    if (gesture === null || gesture.pointerId !== event.pointerId) return;

    pointerGesture.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (!gesture.didDrag) onExpand();
  };

  const handlePointerCancel = (event: PointerEvent<HTMLButtonElement>) => {
    if (pointerGesture.current?.pointerId === event.pointerId) {
      pointerGesture.current = null;
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key !== "Enter" && event.key !== " ") return;

    event.preventDefault();
    onExpand();
  };

  return (
    <main className="compact-canvas">
      <button
        className={`compact-orb${isLive ? " is-live" : ""}${
          isStale ? " is-stale" : ""
        }`}
        type="button"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerCancel}
        onKeyDown={handleKeyDown}
        title={`周额度${weeklyWindow === null ? "等待同步" : `剩余 ${weeklyWindow.remainingPercent}%`} · ${sourceLabel} · 点击查看详情`}
        aria-label={`周额度${weeklyWindow === null ? "等待同步" : `剩余 ${weeklyWindow.remainingPercent}%`}，${sourceLabel}数据，点击查看详情`}
      >
        <svg viewBox="0 0 72 72" aria-hidden="true">
          <defs>
            <linearGradient
              id="compact-orb-gradient"
              x1="0"
              y1="0"
              x2="1"
              y2="1"
            >
              <stop offset="0%" stopColor="#ff3d72" />
              <stop offset="100%" stopColor="#7548ff" />
            </linearGradient>
          </defs>
          <circle className="compact-orb-track" cx="36" cy="36" r="29" />
          <circle
            className="compact-orb-progress"
            cx="36"
            cy="36"
            r="29"
            pathLength="100"
            strokeDasharray="100"
            strokeDashoffset={100 - (weeklyWindow?.remainingPercent ?? 0)}
          />
        </svg>
        <span className="compact-orb-reading">
          <strong>
            {weeklyWindow === null ? "—" : `${weeklyWindow.remainingPercent}%`}
          </strong>
          <small>{sourceLabel}</small>
        </span>
      </button>
    </main>
  );
}

function QuotaGauge({ window }: { window: QuotaWindow | null }) {
  return (
    <div className="quota-gauge" aria-hidden="true">
      <svg viewBox="0 0 128 128">
        <defs>
          <linearGradient id="reserve-gauge-gradient" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#ff3d72" />
            <stop offset="58%" stopColor="#ff4d7e" />
            <stop offset="100%" stopColor="#7548ff" />
          </linearGradient>
        </defs>
        <circle className="gauge-track" cx="64" cy="64" r="49" />
        <circle
          className="gauge-progress"
          cx="64"
          cy="64"
          r="49"
          pathLength="100"
          strokeDasharray="100"
          strokeDashoffset={100 - (window?.remainingPercent ?? 0)}
        />
      </svg>
      <div className="gauge-center">
        <strong>
          {window === null ? "—" : `${window.remainingPercent}%`}
        </strong>
        <span>剩余</span>
      </div>
    </div>
  );
}

function QuotaDetail({ window }: { window: QuotaWindow }) {
  return (
    <div className={`quota-detail is-${window.id}`}>
      <div>
        <span>周期</span>
        <strong>{formatDuration(window.windowDurationMins)}</strong>
        <small>{formatResetCountdown(window.resetsAtUnixSeconds)}</small>
      </div>
      <b>
        {window.remainingPercent}
        <span>%</span>
      </b>
    </div>
  );
}

export function UsageDashboard({
  snapshot,
  cliStatus,
  accountStatus,
  usageStatus,
  sessionUsageStatus,
  threadTokenUsageStatus,
  mode,
  canCollapse,
  onModeChange,
  onStartDragging,
  onHide,
}: UsageDashboardProps) {
  const capturedAt = new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(snapshot.capturedAtMs);
  const isLive = snapshot.source === "codex";
  const isStale = snapshot.source === "stale";
  const cliPresentation = presentCliStatus(cliStatus);
  const accountPresentation = presentAccountStatus(accountStatus);
  const tokenUsage = presentTokenUsage(usageStatus, sessionUsageStatus);
  const threadTokenUsage = presentThreadTokenUsage(threadTokenUsageStatus);
  const weeklyWindow = selectWeeklyQuotaWindow(snapshot.windows);
  const quotaStyle: GaugeStyle = {
    "--remaining": `${weeklyWindow?.remainingPercent ?? 0}%`,
  };
  const connectionDetail = presentConnectionDetail({
    accountStatus,
    accountDetail: accountPresentation.detail,
    cliStatus,
    cliDetail: cliPresentation.detail,
    isLive,
    isStale,
  });

  if (mode === "compact") {
    return (
      <CompactWidget
        snapshot={snapshot}
        onExpand={() => onModeChange("detailed")}
        onStartDragging={onStartDragging}
      />
    );
  }

  return (
    <main className="window-canvas">
      <section className="status-panel">
        <header className="panel-header">
          <div className="panel-drag-region" data-tauri-drag-region="deep">
            <span className="brand-mark" aria-hidden="true">
              CR
            </span>
            <div className="brand-copy">
              <strong>Codex 余量</strong>
              <span>
                {isLive
                  ? "实时读取本机 Codex 用量"
                  : isStale
                    ? "显示上次读取的 Codex 用量"
                    : "Codex 用量界面预览"}
              </span>
            </div>
            <span
              className={`source-badge${isLive ? " is-live" : ""}${
                isStale ? " is-stale" : ""
              }`}
            >
              <i aria-hidden="true" />
              {isLive ? "额度实时" : isStale ? "过期缓存" : "演示"}
            </span>
          </div>
          <div className="panel-actions">
            {canCollapse && (
              <button
                className="window-action collapse-button"
                type="button"
                onClick={() => onModeChange("compact")}
                aria-label="收起为用量悬浮条"
              >
                <span aria-hidden="true" />
              </button>
            )}
            <button
              className="window-action hide-button"
              type="button"
              onClick={onHide}
              aria-label="隐藏用量窗口"
            >
              <span aria-hidden="true" />
            </button>
          </div>
        </header>

        <div className="panel-content">
          <section className="quota-hero" aria-label="Codex 配额余量">
            <div className="quota-overview">
              <QuotaGauge window={weeklyWindow} />
              <div className="quota-details">
                {weeklyWindow === null ? (
                  <div className="quota-empty">
                    <span>周期</span>
                    <strong>等待周额度同步</strong>
                  </div>
                ) : (
                  <QuotaDetail window={weeklyWindow} />
                )}
              </div>
            </div>
            <div className="quota-rail" style={quotaStyle}>
              <span className="quota-rail-fill" aria-hidden="true" />
              <div>
                <small>
                  {weeklyWindow === null ? "等待周额度同步" : "周额度余量"}
                </small>
                <strong>
                  {weeklyWindow === null
                    ? "—"
                    : `已用 ${100 - weeklyWindow.remainingPercent}%`}
                </strong>
              </div>
            </div>
          </section>

          <section className="metric-grid" aria-label="今日 Token 监控">
            <article className="metric-card token-card">
              <span>{tokenUsage.dailyLabel}</span>
              <strong>{tokenUsage.todayTokens}</strong>
              <small>{tokenUsage.trendDetail}</small>
            </article>
            <article className="metric-card cache-card">
              <span>{tokenUsage.totalLabel}</span>
              <strong>{tokenUsage.lifetimeTokens}</strong>
              <small>{tokenUsage.peakDailyTokens}</small>
            </article>
          </section>

          <section className="task-strip" aria-label="当前任务 Token">
            <span className="task-mark" aria-hidden="true" />
            <div>
              <small>当前任务 Token</small>
              <strong>{threadTokenUsage.label}</strong>
              <em>{threadTokenUsage.detail}</em>
            </div>
            <span className="task-value">{threadTokenUsage.total}</span>
          </section>

          <footer className="panel-footer">
            <span>
              {isLive ? "实时" : isStale ? "过期缓存" : "演示"} · {capturedAt} ·
              仅本机
            </span>
            <span title={connectionDetail}>
              {connectionDetail}
            </span>
          </footer>
        </div>
      </section>
    </main>
  );
}

export function presentConnectionDetail({
  accountStatus,
  accountDetail,
  cliStatus,
  cliDetail,
  isLive,
  isStale,
}: {
  accountStatus: CodexAccountStatus | null;
  accountDetail: string | null;
  cliStatus: CodexCliStatus | null;
  cliDetail: string;
  isLive: boolean;
  isStale: boolean;
}): string {
  if (
    accountStatus?.state === "unavailable" &&
    cliStatus !== null &&
    cliStatus.state !== "available"
  ) {
    return [accountDetail, cliDetail].filter(Boolean).join(" · ");
  }

  const detail =
    accountStatus?.state === "signedIn"
      ? accountStatus.isStale
        ? isStale
          ? "账户与额度均为过期缓存"
          : "账户过期缓存 · 额度实时"
        : isLive
          ? "真实账户信息 · 额度实时"
          : isStale
            ? "真实账户信息 · 额度过期"
            : "真实账户信息 · 额度待同步"
      : (accountDetail ?? cliDetail);

  return cliStatus?.isStale ? `${detail} · CLI 状态过期` : detail;
}

export function presentAccountStatus(status: CodexAccountStatus | null): {
  label: string;
  detail: string | null;
  planDetail: string;
} {
  if (status === null) {
    return {
      label: "读取账户中",
      detail: "正在调用 account/read",
      planDetail: "等待账户同步",
    };
  }

  switch (status.state) {
    case "signedIn":
      return {
        label: "账户已连接",
        detail: status.isStale
          ? "账户信息 · 过期缓存"
          : "真实账户信息 · 额度待同步",
        planDetail: status.isStale
          ? "真实套餐 · 过期缓存"
          : "真实套餐 · Credits 未提供",
      };
    case "signedOut":
      return {
        label: status.isStale ? "账户状态缓存" : "账户未登录",
        detail: status.isStale ? `过期缓存 · ${status.message}` : status.message,
        planDetail: status.isStale ? "账户状态 · 过期缓存" : "账户未登录",
      };
    case "unavailable":
      return {
        label: "账户不可用",
        detail: status.message,
        planDetail: "等待账户同步",
      };
  }
}

export function presentPlanCredits(
  accountStatus: CodexAccountStatus | null,
  snapshot: UsageSnapshot,
): {
  plan: string;
  detail: string;
} {
  const plan = snapshot.planType ?? accountStatus?.planType ?? null;

  if (accountStatus?.state === "signedIn" || snapshot.source !== "demo") {
    return {
      plan: plan ?? "未知套餐",
      detail:
        snapshot.creditsBalance ??
        (snapshot.source === "stale" ? "Credits 未提供 · 过期缓存" : "Credits 未提供"),
    };
  }

  if (accountStatus?.state === "signedOut") {
    return {
      plan: "—",
      detail: accountStatus.isStale ? "账户状态 · 过期缓存" : "账户未登录",
    };
  }

  return {
    plan: "—",
    detail: "等待账户同步",
  };
}

export function selectLowestQuotaWindow(windows: QuotaWindow[]): QuotaWindow | null {
  return windows.reduce<QuotaWindow | null>(
    (lowest, window) =>
      lowest === null || window.remainingPercent < lowest.remainingPercent
        ? window
        : lowest,
    null,
  );
}

export function selectWeeklyQuotaWindow(
  windows: QuotaWindow[],
): QuotaWindow | null {
  return (
    windows.find((window) => window.id === "secondary") ??
    windows.find((window) => (window.windowDurationMins ?? 0) >= 24 * 60) ??
    null
  );
}
