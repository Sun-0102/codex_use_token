export const USAGE_REFRESH_EVENT = "codex-reserve://usage-refresh";

type StopListening = () => void;
type SubscribeToRefresh = (
  onRefresh: () => void,
) => Promise<StopListening>;

interface StaleAwareStatus {
  message: string;
  isStale?: boolean;
}

export function mergeRefreshStatus<T extends StaleAwareStatus>(
  previous: T | null,
  incoming: T,
  isUsable: (status: T) => boolean,
): T {
  if (isUsable(incoming)) {
    return { ...incoming, isStale: false };
  }

  if (previous !== null && isUsable(previous)) {
    return {
      ...previous,
      message: incoming.message,
      isStale: true,
    };
  }

  return { ...incoming, isStale: false };
}

export function startUsageRefreshLoop(
  refresh: () => Promise<void>,
  subscribe: SubscribeToRefresh,
): () => void {
  let stopped = false;
  let refreshInFlight = false;
  let stopListening: StopListening | null = null;

  const refreshOnce = async () => {
    if (stopped || refreshInFlight) return;

    refreshInFlight = true;
    try {
      await refresh();
    } catch {
      // Individual readers expose their own unavailable state. Keep the loop alive
      // if an unexpected error escapes the refresh round.
    } finally {
      refreshInFlight = false;
    }
  };

  void refreshOnce();
  void subscribe(() => {
    void refreshOnce();
  }).then(
    (stop) => {
      if (stopped) {
        stop();
      } else {
        stopListening = stop;
      }
    },
    () => undefined,
  );

  return () => {
    stopped = true;
    stopListening?.();
  };
}

export function shareRefreshInFlight<T>(
  refresh: () => Promise<T>,
): () => Promise<T> {
  let inFlight: Promise<T> | null = null;

  return () => {
    if (inFlight !== null) return inFlight;

    const current = refresh();
    inFlight = current;
    void current.then(
      () => {
        if (inFlight === current) inFlight = null;
      },
      () => {
        if (inFlight === current) inFlight = null;
      },
    );
    return current;
  };
}
