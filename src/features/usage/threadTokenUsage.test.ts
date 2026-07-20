import { describe, expect, it } from "vitest";
import { presentThreadTokenUsage } from "./threadTokenUsage";

describe("presentThreadTokenUsage", () => {
  it("shows current visible thread token details separately from account usage", () => {
    expect(
      presentThreadTokenUsage({
        state: "available",
        capturedAtMs: 123,
        message: "已接收当前连接可见任务的 Token 用量通知",
        usage: {
          inputTokens: 100,
          cachedInputTokens: 25,
          outputTokens: 40,
          reasoningOutputTokens: 9,
          totalTokens: 174,
        },
      }),
    ).toEqual({
      label: "输入 100 / 输出 40",
      detail: "缓存 25 · 推理 9",
      total: "174",
    });
  });

  it("shows waiting state when no current thread notification is visible", () => {
    expect(
      presentThreadTokenUsage({
        state: "waiting",
        capturedAtMs: 456,
        usage: null,
        message: "等待当前 app-server 连接的 thread/tokenUsage/updated 通知",
      }),
    ).toEqual({
      label: "等待当前任务",
      detail: "有任务更新时显示明细",
      total: "—",
    });
  });

  it("labels retained thread usage as stale after a refresh failure", () => {
    expect(
      presentThreadTokenUsage({
        state: "available",
        capturedAtMs: 123,
        message: "等待当前任务通知",
        isStale: true,
        usage: {
          inputTokens: 100,
          cachedInputTokens: 25,
          outputTokens: 40,
          reasoningOutputTokens: 9,
          totalTokens: 174,
        },
      }),
    ).toMatchObject({
      detail: "缓存 25 · 推理 9 · 过期缓存",
      total: "174",
    });
  });
});
