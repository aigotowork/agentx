import type { ScopeSelection } from '@agentx/client';

/** Only the page host supplies this value; a route never supplies trust. */
export type SpaceHostBinding = Readonly<{ baseUrl?: string; expectedScope?: string; pageLimit?: string; embedded?: boolean }>;

export function hostSelection(binding: SpaceHostBinding = {}): { selection: ScopeSelection; pageLimit: number } {
  const { baseUrl, expectedScope, pageLimit } = binding;
  if (!baseUrl || !expectedScope) throw new Error('需要宿主配置的管理地址与精确作用域');
  const limit = pageLimit === undefined ? 20 : Number(pageLimit);
  if (!Number.isInteger(limit) || limit < 1 || limit > 20) throw new Error('成员页大小需要 1–20');
  return { selection: { baseUrl, expectedScope, fetch: fetchWithDeadline }, pageLimit: limit };
}

const fetchWithDeadline: typeof fetch = (input, init) => {
  const caller = init?.signal ?? (input instanceof Request ? input.signal : undefined);
  const timeout = AbortSignal.timeout(15000);
  return globalThis.fetch(input, { ...init, signal: caller ? AbortSignal.any([caller, timeout]) : timeout, redirect: 'error' });
};

