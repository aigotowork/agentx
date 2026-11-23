import type { CloudScopeTypes } from '@agentx/client';
import type { Session } from './model';

export type Revisions = CloudScopeTypes['SpaceChanges'];

function pause(ms: number, signal: AbortSignal) {
  return new Promise<void>(resolve => {
    function finish() { clearTimeout(timer); signal.removeEventListener('abort', finish); resolve(); }
    const timer = setTimeout(finish, ms);
    signal.addEventListener('abort', finish, { once: true });
    if (signal.aborted) finish();
  });
}

/** One active request per visible workbench. A timeout returns the same small revision record. */
export async function watchChanges(session: Session, signal: AbortSignal,
  changed: (next: Revisions, previous?: Revisions) => Promise<void>, failed: (failure: unknown) => boolean,
  fallback: () => void) {
  let previous: Revisions | undefined;
  let backoff = 1000;
  while (!signal.aborted) {
    const cursor = previous && [previous.epoch, previous.roster, previous.messages, previous.transcript].join('.');
    try {
      const query = { generation: session.generation, cursor };
      const next = await (session.access === 'admin' ? session.client.spaceChanges(session.scope, query, { signal }) : session.client.spaceChanges(query, { signal }));
      signal.throwIfAborted();
      await changed(next, previous);
      previous = next;
      backoff = 1000;
      // Coalesce bursts while hosts are producing output.
      await pause(500, signal);
    } catch (error) {
      if (signal.aborted || failed(error)) return;
      if (typeof error === 'object' && error && 'status' in error && error.status === 404) {
        fallback();
        return;
      }
      await pause(backoff, signal);
      backoff = Math.min(30_000, backoff * 2);
    }
  }
}
