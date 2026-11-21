/** Named public SDK access; pages retain only view state and journal orchestration. */
import { createManagementClient } from '@agentx/client';
import type { ManagementTypes } from '@agentx/client';
export { ManagementError as ApiError, parseJson, stringifyJson, parseManagementJson } from '@agentx/client';

// Embedded routes use their listener. A separately hosted app may explicitly select
// a management endpoint without a proxy; no endpoint is inferred from another domain.
const location = globalThis.location;
const baseUrl = location ? new URLSearchParams(location.search).get('managementUrl') ?? location.origin : 'http://localhost';
// Keep the embedded namespace compatible; explicitly selected listeners must
// never inherit another listener's drafts, selection or replay credentials.
const listener = new URL(baseUrl, location?.href ?? 'http://localhost');
export const managementEndpoint = listener.origin;
export const storageSuffix = location && listener.href !== `${location.origin}/` ? `:${encodeURIComponent(listener.href)}` : '';
export const management = createManagementClient({ baseUrl, fetch: (input, init) => {
  // Generated operations pass a Request, including its caller-owned read signal.
  const callerSignal = init?.signal ?? (input instanceof Request ? input.signal : undefined);
  const signal = callerSignal ? AbortSignal.any([callerSignal, AbortSignal.timeout(15000)]) : AbortSignal.timeout(15000);
  return globalThis.fetch(input, { ...init, signal });
} });

/** Keep the retained browser intent; dispatch only the selected named SDK method. */
export function submitManagement(kind: string, body: ManagementTypes['SubmitLaunch'] | ManagementTypes['SubmitMessage'] | ManagementTypes['SubmitTarget'] | ManagementTypes['SubmitDirectoryMessage'] | ManagementTypes['SubmitLabels']) {
  if (kind === 'launch' && 'launch' in body) return management.agents.launch(body);
  if (kind === 'directory-send' && 'directory' in body) return management.directory.send(body);
  if (kind === 'send' && 'message' in body && 'agent' in body) return management.send(body);
  if (!('agent' in body)) throw new TypeError('Expected retained exact target input');
  switch (kind) {
    case 'labels': return management.agents.labels.patch(body);
    case 'stop': case 'kill': return management.agents.kill(body);
    case 'stop-participation': return management.agents.stopParticipation(body);
    case 'fork': return management.agents.fork(body);
    case 'resume': return management.agents.resume(body);
    case 'transcript': return management.agents.transcript(body);
    default: throw new TypeError('Unknown management intent');
  }
}
