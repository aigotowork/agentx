/** Page connection over public passive Observation SDK; native unions stay intact. */
import { createObservationClient } from '@agentx/client';
import type { ObservationClient } from '@agentx/client';
import type { ReadCall, ReadResult, ScopeRef } from './native.js';
export type * from './native.js';
export { AdapterError, AdapterDisconnected } from '@agentx/client';

/**
 * @aiPurpose Own one independently opened investigation connection.
 * @aiUsage Await connect and close on page exit; SDK owns finite reads, cancellation and EventsRead overlap guards.
 */
export class ObservationApi {
  private constructor(readonly scope: ScopeRef, private readonly client: ObservationClient) {}
  static async connect(baseUrl = new URLSearchParams(location.search).get('observationUrl') ?? location.origin): Promise<ObservationApi> {
    const client = createObservationClient({ baseUrl });
    return new ObservationApi(await client.scope(), client);
  }
  read(call: ReadCall, signal?: AbortSignal): Promise<ReadResult> { return this.client.read(call, { signal }); }
  async close(): Promise<void> { this.client.close(); }
}
