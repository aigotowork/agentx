import { useEffect, useState } from 'react';
import { CloudAdminError, createCloudAdminClient } from '@agentx/client';

export type AdminClient = ReturnType<typeof createCloudAdminClient>;
export type AdminOverview = Awaited<ReturnType<AdminClient['overview']>>;
export type Namespace = Awaited<ReturnType<AdminClient['getNamespace']>>;
export type NamespaceReceipt = Awaited<ReturnType<AdminClient['putNamespace']>>;
export type NamespaceTarget = Pick<Namespace, 'namespace_id' | 'credential_version'>;
export type NamespaceWriteAction = { kind: 'create' } | { kind: 'rotate'; namespace: NamespaceTarget };
export type Read<T> = { kind: 'loading' } | { kind: 'error'; error: unknown } | { kind: 'ready'; data: T };
export { cloudTokenPattern } from '@agentx/client';

/** Bind only this deployment, with finite calls and no browser credential caching. */
export function deploymentClient(token: string) {
  return createCloudAdminClient({ baseUrl: location.origin, token, fetch(input, init) {
    const caller = init?.signal ?? (input instanceof Request ? input.signal : undefined);
    const timeout = AbortSignal.timeout(15_000);
    const signal = caller ? AbortSignal.any([caller, timeout]) : timeout;
    return globalThis.fetch(input, { ...init, signal, cache: 'no-store', credentials: 'omit', redirect: 'error' });
  } });
}

/** Do not echo server text, which is outside the browser's credential boundary. */
export function explainError(error: unknown, fallback: string) {
  if (!(error instanceof CloudAdminError)) return fallback;
  if (error.code === 'unsupported_operation') return '此部署不支持这项操作。';
  if (error.code === 'stale_generation') return 'Namespace 代次已变化，请重新读取清单。';
  if (error.status === 401 || error.status === 403) return '管理员令牌无效或没有权限。';
  if (error.status === 404) return '该资源已不存在。请刷新部署。';
  if (error.status === 409) return 'Namespace 或令牌版本已变化。请刷新后核对。';
  return fallback;
}

export function staleInventory(error: unknown) {
  return error instanceof CloudAdminError && error.code === 'stale_generation';
}

/** Cancel obsolete calls and show only data for the current reader and retry. */
export function useRead<T>(reader: (signal: AbortSignal) => Promise<T>) {
  const [attempt, setAttempt] = useState(0);
  const [resolved, setResolved] = useState<{ reader: typeof reader; attempt: number; result: Read<T> }>();
  useEffect(() => {
    const controller = new AbortController();
    const finish = (result: Read<T>) => {
      if (!controller.signal.aborted) setResolved({ reader, attempt, result });
    };
    void reader(controller.signal).then(data => finish({ kind: 'ready', data }), error => finish({ kind: 'error', error }));
    return () => controller.abort();
  }, [reader, attempt]);
  const read: Read<T> = resolved?.reader === reader && resolved.attempt === attempt ? resolved.result : { kind: 'loading' };
  return { read, retry: () => setAttempt(value => value + 1) };
}

export const pageSize = 20;

export function useCursor() {
  const [history, setHistory] = useState<(string | undefined)[]>([undefined]);
  return {
    after: history.at(-1), index: history.length - 1,
    reset: () => setHistory([undefined]),
    back: () => setHistory(items => items.length > 1 ? items.slice(0, -1) : items),
    next: (cursor: string) => setHistory(items => items.at(-1) !== cursor ? [...items, cursor] : items),
  };
}
