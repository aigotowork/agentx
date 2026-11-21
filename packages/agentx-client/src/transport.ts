/** @aiPurpose Share HTTP admission and transport failure knowledge without flattening domain result unions. */

/** @aiPurpose Represent a finite listener HTTP admission failure. @aiUsage Distinguish status/code from native Rejected or LocalFailure values in successful responses. */
export class AdapterError extends Error {
  constructor(readonly status: number, readonly code: string) { super(code); this.name = 'AdapterError'; }
}
/** @aiPurpose Retain transport uncertainty for reads with no effect claim. @aiUsage Preserve existing evidence and caller references; this does not prove daemon failure. */
export class AdapterDisconnected extends Error {
  constructor(cause: unknown) { super('HTTP adapter connection/response unavailable', { cause }); this.name = 'AdapterDisconnected'; }
}
/** @aiPurpose Configure a listener explicitly. @aiUsage Supply independent management/workbench/observation URLs and optional standard Fetch implementation. */
export interface HttpOptions { baseUrl: string; fetch?: typeof fetch; headers?: HeadersInit }
/** @aiPurpose Control a single HTTP observation. @aiUsage Abort stops this client's observation only. */
export interface CallOptions { signal?: AbortSignal }
/** @aiPurpose Preserve generated response text without parsing through Number. @aiUsage Only the matching schema codec may interpret this successful response. */
export async function responseText(pending: Promise<{ data?: unknown; error?: unknown; response?: Response }>): Promise<string> {
  const result = await pending;
  if (result.response && !result.response.ok) {
    const error = result.error;
    const code = error && typeof error === 'object' && 'error' in error && typeof error.error === 'string' ? error.error : 'ResponseUnavailable';
    throw new AdapterError(result.response.status, code);
  }
  if (result.error !== undefined || !result.response?.ok) throw new AdapterDisconnected(result.error);
  if (typeof result.data !== 'string') throw new AdapterDisconnected(new TypeError('Expected raw generated transport text'));
  return result.data;
}
/** @aiPurpose Preserve a detached complete write intent before dispatch. @aiUsage Validate/encode first, then freeze the decoded copy used by error recovery. */
export function freezeInput<T>(value: T): T {
  if (value && typeof value === 'object') { Object.freeze(value); for (const item of Object.values(value)) freezeInput(item); }
  return value;
}
/** @aiPurpose Retain a nonjournal mutation with unknown transport result. @aiUsage Inspect the original operation and input; use that domain's reconciliation policy instead of automatically replaying. */
export class MutationOutcomeUnknown<T = unknown> extends Error {
  readonly code = 'OutcomeUnknown';
  constructor(readonly operation: string, readonly input: Readonly<T>, cause: unknown) {
    super(`${operation} outcome unknown; original input retained`, { cause }); this.name = 'MutationOutcomeUnknown';
  }
}

/** @aiPurpose Retain original wire evidence when a successful HTTP response violates its emitted schema. @aiUsage Inspect rawText and schema without treating protocol failure as confirmed nonexecution. */
export class ProtocolError extends Error {
  readonly code = 'ProtocolError';
  constructor(readonly schema: string, readonly rawText: string, cause: unknown) {
    super(`Response does not match emitted ${schema} schema`, { cause }); this.name = 'ProtocolError';
  }
}
/** @aiPurpose Validate raw response text while retaining malformed protocol evidence. @aiUsage Pass only the actual matching listener codec callback. */
export function decodeResponse<T>(schema: string, text: string, decode: (text: string) => T): T {
  try { return decode(text); } catch (cause) { throw new ProtocolError(schema, text, cause); }
}
