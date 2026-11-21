/** @aiPurpose Passive management of retained per-Agent analysis jobs; never an execution or runner-availability claim. */
export const INSIGHT_JOBS_LIMIT = 140;

/** Completeness applies only to the manager's retained window, not all historical jobs. */
export interface InsightJobsSource {
  status: "available" | "partial";
  complete: boolean;
  truncated: boolean;
  /** Number of records in the manager before the response cap. */
  total: number;
  errorCode: "storage_error" | null;
}

/** Native summaries keep their original status, runner, coverage, usage and report semantics. */
export interface InsightJobsSnapshot<Summary> {
  revision: string;
  generatedAt: string;
  jobs: Summary[];
  source: InsightJobsSource;
  retention: { scope: "retained"; reportLimit: number; statusLimit: number; reportBytesLimit: number };
}
