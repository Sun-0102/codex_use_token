import type { CodexCliStatus } from "../../platform/runtime";

export interface CliStatusPresentation {
  label: string;
  detail: string;
  tone: "neutral" | "ready" | "error";
}

export function presentCliStatus(
  status: CodexCliStatus | null,
): CliStatusPresentation {
  if (status === null) {
    return {
      label: "正在检测",
      detail: "检查本机 Codex CLI",
      tone: "neutral",
    };
  }

  switch (status.state) {
    case "available":
      return {
        label: "CLI 已就绪",
        detail: status.version
          ? `v${status.version} · 实时适配器待接入`
          : "已登录 · 实时适配器待接入",
        tone: "ready",
      };
    case "notInstalled":
      return {
        label: "未检测到 CLI",
        detail: "请先安装 Codex CLI",
        tone: "error",
      };
    case "notLoggedIn":
      return {
        label: "CLI 未登录",
        detail: "请在终端运行 codex login",
        tone: "error",
      };
    case "incompatible":
      return {
        label: "CLI 版本过低",
        detail: status.message,
        tone: "error",
      };
    case "launchFailed":
      return {
        label: "CLI 启动失败",
        detail: status.message,
        tone: "error",
      };
  }
}
