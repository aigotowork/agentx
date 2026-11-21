/** @aiPurpose Bind bearer transport to an explicitly selected cloud endpoint. */

export interface CloudClientOptions {
  baseUrl: string;
  token: string;
  fetch?: typeof fetch;
}

export type CloudWireResult = { data?: unknown; error?: unknown; response?: Response };

export class CloudHttpError extends Error {
  constructor(readonly code: string, message: string, readonly status?: number) {
    super(message);
    this.name = 'CloudHttpError';
  }
}

export function cloudBaseUrl(input: string, surface: 'admin' | 'scope'): string {
  let url: URL;
  try { url = new URL(input); } catch { throw new TypeError('Cloud endpoint must be an absolute URL'); }
  const loopback = url.hostname === 'localhost' || url.hostname === '[::1]' || /^127(?:\.\d{1,3}){3}$/.test(url.hostname);
  if (url.protocol !== 'https:' && !(url.protocol === 'http:' && loopback)) {
    throw new TypeError('Cloud endpoint must use HTTPS (HTTP is allowed on loopback)');
  }
  if (url.username || url.password || url.search || url.hash) {
    throw new TypeError('Cloud endpoint must not contain credentials, a query or a fragment');
  }
  let path = url.pathname.replace(/\/$/, '');
  if (surface === 'scope' && !path.endsWith('/api/v1')) path += '/api/v1';
  if (surface === 'admin' ? path !== '' : !/^\/scopes\/[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\/api\/v1$/.test(path)) {
    throw new TypeError(surface === 'admin' ? 'Admin endpoint must be a deployment root' : 'Scope endpoint must be /scopes/ID (optionally followed by /api/v1)');
  }
  return url.origin + path;
}

export function assertScopeId(id: string): void {
  if (typeof id !== 'string' || !/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(id)) {
    throw new TypeError('Scope ID must be 1–63 lowercase letters, digits or hyphens, with alphanumeric ends');
  }
}

export function assertCloudRequestId(id: string): void {
  if (typeof id !== 'string' || !/^[\x21-\x7e]{1,128}$/.test(id)) {
    throw new TypeError('Request ID must be 1–128 ASCII graphic characters');
  }
}

export function assertCloudToken(token: string): void {
  if (typeof token !== 'string' || !/^[A-Za-z0-9_-]{32,256}$/.test(token)) {
    throw new TypeError('Cloud token must contain 32–256 URL-safe characters (letters, digits, underscore, hyphen)');
  }
}

export function cloudFetchConfig(options: CloudClientOptions, surface: 'admin' | 'scope') {
  const baseUrl = cloudBaseUrl(options.baseUrl, surface);
  assertCloudToken(options.token);
  return {
    baseUrl, fetch: options.fetch, headers: { Authorization: `Bearer ${options.token}` },
    parseAs: 'text', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer',
  } as const;
}

function redact(value: string, secrets: readonly string[]): string {
  for (const secret of secrets) if (secret) value = value.replaceAll(secret, '[redacted]');
  return value;
}

function cloudError(result: CloudWireResult, secrets: readonly string[]): CloudHttpError {
  const status = result.response?.status;
  const body = result.error;
  if (status !== undefined && status >= 400 && body && typeof body === 'object'
      && 'code' in body && typeof body.code === 'string' && 'message' in body && typeof body.message === 'string') {
    return new CloudHttpError(redact(body.code, secrets), redact(body.message, secrets), status);
  }
  if (status !== undefined && status >= 400) return new CloudHttpError('InvalidErrorResponse', `Cloud request failed (HTTP ${status})`, status);
  if (status === undefined && body instanceof Error && body.name === 'TimeoutError') return new CloudHttpError('DeadlineExceeded', 'Cloud request deadline exceeded');
  return new CloudHttpError('TransportError', 'Cloud connection or response unavailable', status);
}

/** Do not retain generated Request, raw response text or exception causes: all may contain credentials. */
export async function cloudResponse<T>(pending: Promise<CloudWireResult>, decode: (text: string) => T, secrets: readonly string[]): Promise<T> {
  let result: CloudWireResult;
  try { result = await pending; } catch { throw new CloudHttpError('TransportError', 'Cloud connection or response unavailable'); }
  if (!result.response?.ok || typeof result.data !== 'string') throw cloudError(result, secrets);
  try { return decode(result.data); } catch { throw new CloudHttpError('InvalidReceipt', 'Cloud response does not match the emitted HTTP contract', result.response.status); }
}

/** A successful-but-malformed response, unstructured refusal, transport failure or 5xx cannot establish that a mutation did not apply. */
export function isCloudRejection(error: unknown): error is CloudHttpError {
  return error instanceof CloudHttpError && error.code !== 'InvalidErrorResponse'
    && error.status !== undefined && error.status >= 400 && error.status < 500;
}
