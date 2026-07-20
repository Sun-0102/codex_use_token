import type { CodexThreadTokenUsageStatus } from "../../platform/runtime";
import { formatTokenCount } from "./tokenUsage";

export interface ThreadTokenUsagePresentation {
  label: string;
  detail: string;
  total: string;
}

export function presentThreadTokenUsage(
  status: CodexThreadTokenUsageStatus | null,
): ThreadTokenUsagePresentation {
  if (status === null || status.state !== "available" || status.usage === null) {
    return {
      label: "等待当前任务",
      detail: status?.state === "unavailable" ? status.message : "有任务更新时显示明细",
      total: "—",
    };
  }

  return {
    label: `输入 ${formatTokenCount(status.usage.inputTokens)} / 输出 ${formatTokenCount(
      status.usage.outputTokens,
    )}`,
    detail: `缓存 ${formatTokenCount(
      status.usage.cachedInputTokens,
    )} · 推理 ${formatTokenCount(status.usage.reasoningOutputTokens)}${
      status.isStale ? " · 过期缓存" : ""
    }`,
    total: formatTokenCount(status.usage.totalTokens),
  };
}
