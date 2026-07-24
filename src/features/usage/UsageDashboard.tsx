import type { CSSProperties } from "react";
import type {
  CodexAccountStatus,
  CodexCliStatus,
  CodexSessionUsageStatus,
  CodexThreadTokenUsageStatus,
  CodexUsageStatus,
  UsageWindowMode,
} from "../../platform/runtime";
import { presentCliStatus } from "./cliStatus";
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
  onHide: () => void;
}

const RESERVE_THRESHOLD_PERCENT = 20;

type GaugeStyle = CSSProperties & {
  "--remaining": string;
};

function CompactQuota({ window }: { window: QuotaWindow }) {
  const gaugeStyle: GaugeStyle = {
    "--remaining": `${window.remainingPercent}%`,
  };

  return (
    <span className={`compact-quota is-${window.id}`}>
      <span className="compact-reading">
        <small>{window.id === "primary" ? "5H" : "W"}</small>
        <strong>{window.remainingPercent}%</strong>
      </span>
      <span className="compact-track" aria-hidden="true" style={gaugeStyle}>
        <i />
      </span>
    </span>
  );
}

function CompactWidget({
  snapshot,
  onExpand,
}: {
  snapshot: UsageSnapshot;
  onExpand: () => void;
}) {
  const isLive = snapshot.source === "codex";
  const isStale = snapshot.source === "stale";

  return (
    <main className="compact-canvas">
      <section className="compact-widget">
        <div
          className="compact-drag-region"
          data-tauri-drag-region
          title="拖动悬浮窗"
        >
          <span className="compact-brand" aria-hidden="true">
            CR
          </span>
          <span
            className={`compact-source-dot${isLive ? " is-live" : ""}${
              isStale ? " is-stale" : ""
            }`}
            role="img"
            aria-label={
              isLive
                ? "当前额度为实时数据"
                : isStale
                  ? "当前为过期缓存数据"
                  : "当前为演示数据"
            }
          />
        </div>
        <button
          className="compact-expand"
          type="button"
          onClick={onExpand}
          aria-label="展开 Codex 用量详情"
        >
          {snapshot.windows.map((window) => (
            <CompactQuota key={window.id} window={window} />
          ))}
          <span className="expand-chevron" aria-hidden="true" />
        </button>
      </section>
    </main>
  );
}

function QuotaOrbit({ windows }: { windows: QuotaWindow[] }) {
  const primary = windows.find((window) => window.id === "primary");
  const secondary = windows.find((window) => window.id === "secondary");
  const lowestWindow = selectLowestQuotaWindow(windows);

  return (
    <div className="quota-orbit" aria-hidden="true">
      <svg viewBox="0 0 128 128">
        <circle className="orbit-track" cx="64" cy="64" r="53" />
        <circle
          className="orbit-progress orbit-primary"
          cx="64"
          cy="64"
          r="53"
          pathLength="100"
          strokeDasharray="100"
          strokeDashoffset={100 - (primary?.remainingPercent ?? 0)}
        />
        <circle className="orbit-track orbit-track-inner" cx="64" cy="64" r="39" />
        <circle
          className="orbit-progress orbit-secondary"
          cx="64"
          cy="64"
          r="39"
          pathLength="100"
          strokeDasharray="100"
          strokeDashoffset={100 - (secondary?.remainingPercent ?? 0)}
        />
      </svg>
      <div className="orbit-center">
        <span>{lowestWindow === null ? "等待余量" : `${lowestWindow.label}余量`}</span>
        <strong>
          {lowestWindow === null ? "—" : `${lowestWindow.remainingPercent}%`}
        </strong>
      </div>
    </div>
  );
}

function QuotaLegend({ window }: { window: QuotaWindow }) {
  return (
    <div className={`quota-legend is-${window.id}`}>
      <span className="legend-dot" aria-hidden="true" />
      <div>
        <strong>{formatDuration(window.windowDurationMins)}</strong>
        <small>{formatResetCountdown(window.resetsAtUnixSeconds)}</small>
      </div>
      <b>{window.remainingPercent}<span>%</span></b>
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
  const planCredits = presentPlanCredits(accountStatus, snapshot);
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
      />
    );
  }

  return (
    <main className="window-canvas">
      <section className="status-panel">
        <header className="panel-header">
          <div className="panel-drag-region" data-tauri-drag-region>
            <span className="brand-mark" aria-hidden="true">
              CR
            </span>
            <div className="brand-copy">
              <strong>Codex Reserve</strong>
              <span>用量监控</span>
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
            <QuotaOrbit windows={snapshot.windows} />
            <div className="quota-summary">
              <div className="summary-heading">
                <span>剩余用量</span>
                <small>低于 {RESERVE_THRESHOLD_PERCENT}% 提醒</small>
              </div>
              <div className="quota-legends">
                {snapshot.windows.map((window) => (
                  <QuotaLegend key={window.id} window={window} />
                ))}
              </div>
            </div>
          </section>

          <section className="insight-card" aria-label="用量建议">
            <span className="insight-mark" aria-hidden="true" />
            <div>
              <small>当前任务 Token</small>
              <strong>{threadTokenUsage.label}</strong>
              <em>{threadTokenUsage.detail}</em>
            </div>
            <span className="insight-value">{threadTokenUsage.total}</span>
          </section>

          <section className="metric-grid" aria-label="详细监控信息">
            <article className="metric-card">
              <span>{tokenUsage.dailyLabel}</span>
              <strong>{tokenUsage.todayTokens}</strong>
              <small>{tokenUsage.trendDetail}</small>
            </article>
            <article className="metric-card">
              <span>{tokenUsage.totalLabel}</span>
              <strong>{tokenUsage.lifetimeTokens}</strong>
              <small>{tokenUsage.peakDailyTokens}</small>
            </article>
            <article
              className={`metric-card connection-card ${
                accountStatus?.state === "signedIn"
                  ? "is-ready"
                  : isLive
                    ? "is-live"
                    : `is-${cliPresentation.tone}`
              }`}
            >
              <span>Codex 连接</span>
              <strong>
                {accountStatus?.state === "signedIn"
                  ? "账户已连接"
                  : accountPresentation.label}
              </strong>
              <small>{connectionDetail}</small>
            </article>
            <article className="metric-card">
              <span>套餐与 Credits</span>
              <strong>{planCredits.plan}</strong>
              <small>{planCredits.detail}</small>
            </article>
          </section>

          <footer className="panel-footer">
            <span>
              {isLive ? "额度实时更新" : isStale ? "过期缓存" : "演示数据"} ·{" "}
              {capturedAt}
            </span>
            <span>数据仅保存在本机</span>
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
