import { describe, expect, it } from "vitest";
import {
  presentAccountStatus,
  presentConnectionDetail,
  presentPlanCredits,
} from "./UsageDashboard";

describe("presentAccountStatus", () => {
  it("marks signed-in account data as real while quota remains pending", () => {
    expect(
      presentAccountStatus({
        state: "signedIn",
        planType: "pro",
        accountType: "chatgpt",
        capturedAtMs: 123,
        message: "真实账户已连接，套餐 pro",
      }),
    ).toEqual({
      label: "账户已连接",
      detail: "真实账户信息 · 额度待同步",
      planDetail: "真实套餐 · Credits 未提供",
    });
  });

  it("surfaces signed-out instructions without pretending usage is live", () => {
    expect(
      presentAccountStatus({
        state: "signedOut",
        planType: null,
        accountType: null,
        capturedAtMs: 456,
        message: "Codex CLI 需要重新登录 OpenAI 账户",
      }),
    ).toMatchObject({
      label: "账户未登录",
      detail: "Codex CLI 需要重新登录 OpenAI 账户",
      planDetail: "账户未登录",
    });
  });

  it("labels retained signed-in account data as stale", () => {
    expect(
      presentAccountStatus({
        state: "signedIn",
        planType: "pro",
        accountType: "chatgpt",
        capturedAtMs: 123,
        message: "读取账户失败",
        isStale: true,
      }),
    ).toEqual({
      label: "账户已连接",
      detail: "账户信息 · 过期缓存",
      planDetail: "真实套餐 · 过期缓存",
    });
  });

  it("does not present a retained signed-out state as current", () => {
    expect(
      presentAccountStatus({
        state: "signedOut",
        planType: null,
        accountType: null,
        capturedAtMs: 123,
        message: "读取账户失败",
        isStale: true,
      }),
    ).toEqual({
      label: "账户状态缓存",
      detail: "过期缓存 · 读取账户失败",
      planDetail: "账户状态 · 过期缓存",
    });
  });
});

describe("presentConnectionDetail", () => {
  it("keeps CLI stale visible when account and quota are live", () => {
    expect(
      presentConnectionDetail({
        accountStatus: {
          state: "signedIn",
          planType: "pro",
          accountType: "chatgpt",
          capturedAtMs: 123,
          message: "真实账户已连接，套餐 pro",
        },
        accountDetail: "真实账户信息 · 额度待同步",
        cliStatus: {
          state: "available",
          executablePath: "/usr/local/bin/codex",
          version: "0.1.0",
          message: "Codex CLI 0.1.0 可用",
          isStale: true,
        },
        cliDetail: "v0.1.0 · 实时适配器待接入 · 过期缓存",
        isLive: true,
        isStale: false,
      }),
    ).toBe("真实账户信息 · 额度实时 · CLI 状态过期");
  });
});

describe("presentPlanCredits", () => {
  it("uses real snapshot credits instead of obsolete T302 pending copy", () => {
    expect(
      presentPlanCredits(
        {
          state: "signedIn",
          planType: "free",
          accountType: "chatgpt",
          capturedAtMs: 123,
          message: "真实账户已连接，套餐 free",
        },
        {
          source: "codex",
          capturedAtMs: 123,
          planType: "free",
          creditsBalance: "无 Credits",
          windows: [],
        },
      ),
    ).toEqual({
      plan: "free",
      detail: "无 Credits",
    });
  });

  it("falls back to neutral copy when credits are not provided", () => {
    expect(
      presentPlanCredits(
        {
          state: "signedIn",
          planType: "pro",
          accountType: "chatgpt",
          capturedAtMs: 123,
          message: "真实账户已连接，套餐 pro",
        },
        {
          source: "codex",
          capturedAtMs: 123,
          planType: "pro",
          creditsBalance: null,
          windows: [],
        },
      ),
    ).toEqual({
      plan: "pro",
      detail: "Credits 未提供",
    });
  });
});
