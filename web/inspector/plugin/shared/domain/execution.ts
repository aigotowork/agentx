/** Passive management projections. These observations never authorize delivery or prove task completion. */
import type { DirectoryHealth } from "./source-health";

export const EXECUTION_DEFERRED_LIMIT = 600;
export const EXECUTION_RESUME_LIMIT = 400;
export const EXECUTION_TARGET_LIMIT = EXECUTION_DEFERRED_LIMIT + EXECUTION_RESUME_LIMIT;

/** Quality describes only this source's retained management window, not all historical operations. */
export interface ExecutionSource<T> {
  status: "available" | "partial" | "unavailable" | "unknown";
  complete: boolean;
  truncated: boolean;
  /** Usable records before the response cap; null when the source cannot establish a count. */
  total: number | null;
  errorCode: "read_failed" | "invalid_data" | "write_failed" | "recovered" | null;
  entries: T[];
}

/** Only identities referenced by returned entries are included. A missing match stays null. */
export interface ExecutionTarget {
  agentId: string;
  agent: { id: string; title: string | null; provider: string; workspaceId: string | null } | null;
  workspace: { id: string; name: string; projectId: string; projectName: string } | null;
}

/** Host identity belongs to the selected plugin installation/transport, not an inferred name or filesystem path. */
export interface ExecutionSnapshot<Deferred, Resume> {
  revision: string;
  generatedAt: string;
  deferred: ExecutionSource<Deferred>;
  resume: ExecutionSource<Resume>;
  directoryHealth: DirectoryHealth;
  targets: ExecutionTarget[];
}
