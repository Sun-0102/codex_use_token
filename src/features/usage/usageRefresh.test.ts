import { afterEach, describe, expect, it, vi } from "vitest";
import {
  mergeRefreshStatus,
  shareRefreshInFlight,
  startUsageRefreshLoop,
} from "./usageRefresh";

afterEach(() => vi.restoreAllMocks());

describe("startUsageRefreshLoop", () => {
  it("refreshes immediately and on every backend tick", async () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const subscription: { emitRefresh: () => void } = {
      emitRefresh: () => undefined,
    };
    const stopListening = vi.fn();
    const subscribe = vi.fn(async (listener: () => void) => {
      subscription.emitRefresh = listener;
      return stopListening;
    });

    const stop = startUsageRefreshLoop(refresh, subscribe);
    await Promise.resolve();

    expect(refresh).toHaveBeenCalledTimes(1);
    expect(subscribe).toHaveBeenCalledTimes(1);

    subscription.emitRefresh();
    await Promise.resolve();
    expect(refresh).toHaveBeenCalledTimes(2);

    subscription.emitRefresh();
    await Promise.resolve();
    expect(refresh).toHaveBeenCalledTimes(3);

    stop();
    subscription.emitRefresh();
    await Promise.resolve();
    expect(refresh).toHaveBeenCalledTimes(3);
    expect(stopListening).toHaveBeenCalledTimes(1);
  });

  it("skips backend ticks while the previous refresh is still running", async () => {
    let finishRefresh: (() => void) | undefined;
    const subscription: { emitRefresh: () => void } = {
      emitRefresh: () => undefined,
    };
    const refresh = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finishRefresh = resolve;
        }),
    );
    const subscribe = async (listener: () => void) => {
      subscription.emitRefresh = listener;
      return () => undefined;
    };

    const stop = startUsageRefreshLoop(refresh, subscribe);
    await Promise.resolve();

    subscription.emitRefresh();
    subscription.emitRefresh();
    subscription.emitRefresh();
    expect(refresh).toHaveBeenCalledTimes(1);

    finishRefresh?.();
    await Promise.resolve();
    subscription.emitRefresh();
    expect(refresh).toHaveBeenCalledTimes(2);

    stop();
  });

  it("continues refreshing after a rejected round", async () => {
    const subscription: { emitRefresh: () => void } = {
      emitRefresh: () => undefined,
    };
    const refresh = vi
      .fn()
      .mockRejectedValueOnce(new Error("temporary failure"))
      .mockResolvedValue(undefined);
    const subscribe = async (listener: () => void) => {
      subscription.emitRefresh = listener;
      return () => undefined;
    };

    const stop = startUsageRefreshLoop(refresh, subscribe);
    await Promise.resolve();
    subscription.emitRefresh();
    await Promise.resolve();

    expect(refresh).toHaveBeenCalledTimes(2);
    stop();
  });

  it("unsubscribes if setup finishes after the loop has stopped", async () => {
    let finishSubscription: (stop: () => void) => void = () => undefined;
    const stopListening = vi.fn();
    const subscribe = () =>
      new Promise<() => void>((resolve) => {
        finishSubscription = resolve;
      });

    const stop = startUsageRefreshLoop(
      () => Promise.resolve(),
      subscribe,
    );
    stop();
    finishSubscription(stopListening);
    await Promise.resolve();

    expect(stopListening).toHaveBeenCalledTimes(1);
  });

  it("keeps the immediate refresh when event subscription fails", async () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const stop = startUsageRefreshLoop(refresh, () =>
      Promise.reject(new Error("event unavailable")),
    );

    await Promise.resolve();

    expect(refresh).toHaveBeenCalledTimes(1);
    stop();
  });
});

describe("shareRefreshInFlight", () => {
  it("shares one request across overlapping React effect lifecycles", async () => {
    let finishRefresh: ((value: number) => void) | undefined;
    const refresh = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          finishRefresh = resolve;
        }),
    );
    const sharedRefresh = shareRefreshInFlight(refresh);

    const first = sharedRefresh();
    const second = sharedRefresh();

    expect(first).toBe(second);
    expect(refresh).toHaveBeenCalledTimes(1);

    finishRefresh?.(42);
    await expect(second).resolves.toBe(42);

    void sharedRefresh();
    expect(refresh).toHaveBeenCalledTimes(2);
  });
});

describe("mergeRefreshStatus", () => {
  type TestStatus = {
    state: "available" | "unavailable";
    value: number | null;
    message: string;
    isStale?: boolean;
  };

  const isAvailable = (status: TestStatus) => status.state === "available";

  it("uses successful refreshed data and clears the stale marker", () => {
    const refreshed = mergeRefreshStatus<TestStatus>(
      {
        state: "available",
        value: 1,
        message: "old",
        isStale: true,
      },
      {
        state: "available",
        value: 2,
        message: "new",
      },
      isAvailable,
    );

    expect(refreshed).toEqual({
      state: "available",
      value: 2,
      message: "new",
      isStale: false,
    });
  });

  it("retains the last successful value and marks it stale after failure", () => {
    const refreshed = mergeRefreshStatus<TestStatus>(
      {
        state: "available",
        value: 1,
        message: "old",
      },
      {
        state: "unavailable",
        value: null,
        message: "refresh failed",
      },
      isAvailable,
    );

    expect(refreshed).toEqual({
      state: "available",
      value: 1,
      message: "refresh failed",
      isStale: true,
    });
  });

  it("keeps an initial failure when no successful value exists", () => {
    const failure: TestStatus = {
      state: "unavailable",
      value: null,
      message: "refresh failed",
    };

    expect(mergeRefreshStatus(null, failure, isAvailable)).toEqual({
      ...failure,
      isStale: false,
    });
  });
});
