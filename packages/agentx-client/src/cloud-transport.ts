import { assertCloudIdentifiers, cloudTokenPattern } from './cloud-identifiers.js';
import { createSchemaCodec } from './schema-codec.js';
/** @aiPurpose Bind bearer transport to an explicitly selected cloud endpoint. */

export interface CloudClientOptions {
  baseUrl: string;
  token: string;
  fetch?: typeof fetch;
}

export type CloudWireResult = { data?: unknown; error?: unknown; response?: Response; request?: Request };

export class CloudHttpError extends Error {
  constructor(readonly code: string, message: string, readonly status?: number) {
    super(message);
    this.name = 'CloudHttpError';
  }
}

type InventoryQuery = { generation?: string; after?: string };

function unsupportedOperation(status?: number) {
  return new CloudHttpError('unsupported_operation', 'This cloud API does not support the requested operation', status);
}

/** Older HTTP consumers can bootstrap a page; continue only with that page's generation. */
export function inventoryGeneration(query?: InventoryQuery): string | undefined {
  const expected = query?.generation;
  if (expected !== undefined && (typeof expected !== 'string' || !expected)
      || query?.after && !expected) throw new TypeError('Cloud inventory cursors require the generation from their original page');
  return expected;
}

export function createCloudCodec(document: Parameters<typeof createSchemaCodec>[0]) {
  const codec = createSchemaCodec(document);
  // A DTO can be closed when used as a write input and still evolve additively as a resource response.
  const schemas = JSON.parse(JSON.stringify(document.components.schemas,
    (key, value) => key === 'additionalProperties' && value === false ? undefined : value));
  const resourceCodec = createSchemaCodec({ components: { schemas } });
  const parse = (schema: string, text: string, resource = false) => {
    const value = (resource ? resourceCodec : codec).parse(schema, text);
    assertCloudIdentifiers(value, true);
    return value;
  };
  return {
    parse,
    page(schema: 'HostPage' | 'AgentPage' | 'WorkspacePage', text: string, expected: string | undefined): unknown {
      const envelope: unknown = JSON.parse(text);
      if (envelope && typeof envelope === 'object' && 'items' in envelope && 'next' in envelope && !('generation' in envelope)) {
        throw unsupportedOperation();
      }
      const page = parse(schema, text, true) as { generation: string };
      if (!page.generation) throw new TypeError('Cloud inventory page has no Namespace generation');
      if (expected !== undefined && page.generation !== expected) {
        throw new CloudHttpError('stale_generation', 'Namespace generation has changed');
      }
      return page;
    },
    stringify(schema: string, value: unknown) {
      assertCloudIdentifiers(value);
      return codec.stringify(schema, value);
    },
  };
}

export function cloudBaseUrl(input: string, surface: 'admin' | 'namespace'): string {
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
  if (surface === 'namespace' && !path.endsWith('/api/v1')) path += '/api/v1';
  if (surface === 'admin' ? path !== '' : !/^\/namespaces\/[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\/api\/v1$/.test(path)) {
    throw new TypeError(surface === 'admin' ? 'Admin endpoint must be a deployment root' : 'Namespace endpoint must be /namespaces/ID (optionally followed by /api/v1)');
  }
  return url.origin + path;
}

export function assertNamespaceId(id: string): void {
  if (typeof id !== 'string' || !/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(id)) {
    throw new TypeError('Namespace ID must be 1–63 lowercase letters, digits or hyphens, with alphanumeric ends');
  }
}

export function assertCloudRequestId(id: string): void {
  if (typeof id !== 'string' || !/^[\x21-\x7e]{1,128}$/.test(id)) {
    throw new TypeError('Request ID must be 1–128 ASCII graphic characters');
  }
}

export function assertCloudToken(token: string): void {
  if (typeof token !== 'string' || !cloudTokenPattern.test(token)) {
    throw new TypeError('Cloud token must contain 12 ASCII letters or digits');
  }
}

export function cloudFetchConfig(options: CloudClientOptions, surface: 'admin' | 'namespace') {
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
  if (status === 405 || status === 501 || (status === 404 && body && typeof body === 'object'
    && 'code' in body && body.code === 'not_found' && 'message' in body && body.message === 'Namespace communication route was not found')) {
    return unsupportedOperation(status);
  }
  if (status !== undefined && status >= 400 && body && typeof body === 'object'
      && 'code' in body && typeof body.code === 'string' && 'message' in body && typeof body.message === 'string') {
    return new CloudHttpError(redact(body.code, secrets), redact(body.message, secrets), status);
  }
  // Older Admin routers return an empty 404 for a missing route. A resource miss
  // is structured. Only GETs may use this fallback; writes keep their uncertainty.
  if (status === 404 && result.request?.method === 'GET' && !body) return unsupportedOperation(status);
  if (status === 404 && result.request?.method === 'GET' && typeof body === 'object'
      && body !== null && Object.keys(body).length === 0) return unsupportedOperation(status);
  if (status !== undefined && status >= 400) return new CloudHttpError('InvalidErrorResponse', `Cloud request failed (HTTP ${status})`, status);
  if (status === undefined && body instanceof Error && body.name === 'TimeoutError') return new CloudHttpError('DeadlineExceeded', 'Cloud request deadline exceeded');
  return new CloudHttpError('TransportError', 'Cloud connection or response unavailable', status);
}

/** Do not retain generated Request, raw response text or exception causes: all may contain credentials. */
export async function cloudResponse<T>(pending: Promise<CloudWireResult>, decode: (text: string) => T, secrets: readonly string[]): Promise<T> {
  let result: CloudWireResult;
  try { result = await pending; } catch { throw new CloudHttpError('TransportError', 'Cloud connection or response unavailable'); }
  if (!result.response?.ok || typeof result.data !== 'string') throw cloudError(result, secrets);
  try { return decode(result.data); } catch (error) {
    if (error instanceof CloudHttpError) throw error;
    throw new CloudHttpError('InvalidReceipt', 'Cloud response does not match the emitted HTTP contract', result.response.status);
  }
}

/** A successful-but-malformed response, unstructured refusal, transport failure or 5xx cannot establish that a mutation did not apply. */
export function isCloudRejection(error: unknown): error is CloudHttpError {
  return error instanceof CloudHttpError && error.code !== 'InvalidErrorResponse'
    && error.status !== undefined && error.status >= 400 && error.status < 500;
}
