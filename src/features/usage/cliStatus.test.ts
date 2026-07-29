import { describe, expect, it } from "vitest";
import { presentCliStatus } from "./cliStatus";

describe("presentCliStatus", () => {
  it("keeps CLI readiness separate from live usage connectivity", () => {
    expect(
      presentCliStatus({
        state: "available",
        executablePath: "/test/codex",
        version: "0.144.5",
        message: "Codex CLI 已安装并登录",
      }),
    ).toEqual({
      label: "CLI 已就绪",
      detail: "v0.144.5 · 已登录",
      tone: "ready",
    });
  });

  it("gives an actionable login instruction", () => {
    expect(
      presentCliStatus({
        state: "notLoggedIn",
        executablePath: "/test/codex",
        version: "0.144.5",
        message: "Codex CLI 尚未登录",
      }),
    ).toMatchObject({
      label: "CLI 未登录",
      detail: "请在终端运行 codex login",
      tone: "error",
    });
  });

  it("lists the searched CLI locations when no executable is found", () => {
    expect(
      presentCliStatus({
        state: "notInstalled",
        executablePath: null,
        version: null,
        message: "未检测到 Codex CLI",
      }),
    ).toMatchObject({
      label: "未检测到 CLI",
      detail: "已检查系统 PATH 和常见安装目录",
      tone: "error",
    });
  });

  it("shows the attempted CLI path when launch fails", () => {
    expect(
      presentCliStatus({
        state: "launchFailed",
        executablePath: "/Users/test/.nvm/versions/node/v24.18.0/bin/codex",
        version: "0.144.5",
        message: "Codex CLI 登录状态检查失败，请在终端运行 codex login status",
      }),
    ).toMatchObject({
      label: "CLI 启动失败",
      detail:
        "Codex CLI 登录状态检查失败，请在终端运行 codex login status · /Users/test/.nvm/versions/node/v24.18.0/bin/codex",
      tone: "error",
    });
  });

  it("labels retained CLI status as stale", () => {
    expect(
      presentCliStatus({
        state: "available",
        executablePath: "/test/codex",
        version: "0.144.5",
        message: "无法调用本机 CLI 探测服务",
        isStale: true,
      }),
    ).toEqual({
      label: "CLI 已就绪",
      detail: "v0.144.5 · 已登录 · 过期缓存",
      tone: "ready",
    });
  });
});
