/** @aiPurpose Describe native-directory availability without SDK, transport, storage or UI dependencies. */

/** @aiPurpose Carry the quality of the directory data actually displayed. @aiUsage A missing DTO means unknown; directory health never describes provider-log coverage or authorizes a command. */
export interface DirectoryHealth {
  source: "agentx-directory";
  status: "healthy" | "stale" | "unavailable" | "unknown";
  observedAt: string | null;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  complete: boolean;
  /** Volatile source health version, independent of persisted metric revisions. */
  revision: number;
  errorCode: "read_failed" | "timeout" | "invalidated" | "incomplete" | null;
}
