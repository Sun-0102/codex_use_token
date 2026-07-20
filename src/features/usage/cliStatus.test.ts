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
      detail: "v0.144.5 · 实时适配器待接入",
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
      detail: "v0.144.5 · 实时适配器待接入 · 过期缓存",
      tone: "ready",
    });
  });
});
