import { invoke } from "@tauri-apps/api/core";

export interface RuntimeHealth {
  appVersion: string;
  monitorState: "notConnected" | "connecting" | "connected" | "error";
}

export type CodexCliState =
  | "available"
  | "notInstalled"
  | "notLoggedIn"
  | "incompatible"
  | "launchFailed";

export interface CodexCliStatus {
  state: CodexCliState;
  executablePath: string | null;
  version: string | null;
  message: string;
}

export type CodexAccountState = "signedIn" | "signedOut" | "unavailable";

export interface CodexAccountStatus {
  state: CodexAccountState;
  planType: string | null;
  accountType: string | null;
  capturedAtMs: number;
  message: string;
}

export type CodexRateLimitsState = "available" | "unavailable";
export type CodexRateLimitBucketSource = "default" | "byLimitId";

export interface CodexRateLimitWindow {
  usedPercent: number;
  resetsAt: number | null;
  windowDurationMins: number | null;
}

export interface CodexCreditsSnapshot {
  hasCredits: boolean;
  unlimited: boolean;
  balance: string | null;
}

export interface CodexRateLimitBucket {
  source: CodexRateLimitBucketSource;
  key: string | null;
  limitId: string | null;
  limitName: string | null;
  planType: string | null;
  primary: CodexRateLimitWindow | null;
  secondary: CodexRateLimitWindow | null;
  credits: CodexCreditsSnapshot | null;
}

export interface CodexRateLimitsStatus {
  state: CodexRateLimitsState;
  capturedAtMs: number;
  buckets: CodexRateLimitBucket[];
  message: string;
}

export type CodexUsageState = "available" | "unavailable";

export interface CodexUsageSummary {
  lifetimeTokens: number | null;
  peakDailyTokens: number | null;
  longestRunningTurnSec: number | null;
  currentStreakDays: number | null;
  longestStreakDays: number | null;
}

export interface CodexDailyUsageBucket {
  startDate: string;
  tokens: number;
}

export interface CodexUsageStatus {
  state: CodexUsageState;
  capturedAtMs: number;
  summary: CodexUsageSummary | null;
  dailyUsageBuckets: CodexDailyUsageBucket[];
  message: string;
}

export type CcSwitchUsageState = "available" | "unavailable";

export interface CcSwitchDailyUsage {
  requestCount: number;
  inputTokens: number;
  freshInputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  totalTokens: number;
  totalCostUsd: number;
}

export interface CcSwitchUsageStatus {
  state: CcSwitchUsageState;
  capturedAtMs: number;
  today: CcSwitchDailyUsage | null;
  message: string;
}

export type CodexThreadTokenUsageState =
  | "available"
  | "waiting"
  | "unavailable";

export interface CodexThreadTokenUsage {
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
}

export interface CodexThreadTokenUsageStatus {
  state: CodexThreadTokenUsageState;
  capturedAtMs: number;
  usage: CodexThreadTokenUsage | null;
  message: string;
}

export type UsageWindowMode = "compact" | "detailed";

export function readRuntimeHealth(): Promise<RuntimeHealth> {
  return invoke<RuntimeHealth>("runtime_health");
}

export function readCodexCliStatus(): Promise<CodexCliStatus> {
  return invoke<CodexCliStatus>("codex_cli_status");
}

export function readCodexAccountStatus(): Promise<CodexAccountStatus> {
  return invoke<CodexAccountStatus>("codex_account_status");
}

export function readCodexRateLimitsStatus(): Promise<CodexRateLimitsStatus> {
  return invoke<CodexRateLimitsStatus>("codex_rate_limits_status");
}

export function readCodexUsageStatus(): Promise<CodexUsageStatus> {
  return invoke<CodexUsageStatus>("codex_usage_status");
}

export function readCcSwitchUsageStatus(): Promise<CcSwitchUsageStatus> {
  return invoke<CcSwitchUsageStatus>("cc_switch_usage_status");
}

export function readCodexThreadTokenUsageStatus(): Promise<CodexThreadTokenUsageStatus> {
  return invoke<CodexThreadTokenUsageStatus>("codex_thread_token_usage_status");
}

export function hideUsageWindow(): Promise<void> {
  return invoke<void>("hide_usage_window");
}

export function setUsageWindowMode(mode: UsageWindowMode): Promise<void> {
  return invoke<void>("set_usage_window_mode", { mode });
}

export function updateTrayUsage(
  primaryRemainingPercent: number | null,
  secondaryRemainingPercent: number | null,
): Promise<void> {
  return invoke<void>("update_tray_usage", {
    primaryRemainingPercent,
    secondaryRemainingPercent,
  });
}
