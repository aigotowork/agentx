/** @aiPurpose Expose all selected passive diagnostics while retaining original native evidence/error unions. */
import { createClient } from './generated/observation/client/index.js';
import * as wire from './generated/observation/sdk.gen.js';
import type { SchemaTypes } from './generated/observation/schema-types.gen.js';
import type { Native } from './codec.js';
import { parseObservationJson, observationWireValue } from './codec.js';
import { AdapterDisconnected, AdapterError, responseText, decodeResponse } from './transport.js';
import type { HttpOptions, CallOptions } from './transport.js';

/** @aiPurpose Derive native diagnostic values from actual marker-coded wire declarations. @aiUsage Integers are bigint; stale/gap/epoch/coverage and LocalFailure variants stay intact. */
export type ObservationTypes = { [K in keyof SchemaTypes]: Native<SchemaTypes[K]> };
type O = ObservationTypes;
type Read = O['BrowserRead'];
type Payload<K extends string> = Extract<Read, Record<K, unknown>>[K];
/** @aiPurpose Bind an optional original daemon reference to a passive read. @aiUsage Omission means null, never a refreshed or automatically rebased daemon reference. */
export interface ObservationOptions extends CallOptions { expectedDaemon?: O['DaemonRef'] | null }
/** @aiPurpose Expose finite passive operations with local cancellation. @aiUsage One client per observer lifetime; close stops local reads only and never starts or stops a daemon. */
export interface ObservationClient {
  scope(options?: CallOptions): Promise<O['ScopeRef']>;
  read(input: O['ReadCall'], options?: CallOptions): Promise<O['ReadResult']>;
  close(): void;
  runtime: {
    snapshot(options?: ObservationOptions): Promise<O['ReadResult']>;
    requests: {
      list(input: Payload<'Requests'>, options?: ObservationOptions): Promise<O['ReadResult']>;
      get(input: Payload<'RequestDetail'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    };
    connections(input: Payload<'Connections'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    peers(input: Payload<'Peers'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    hosts(input: Payload<'Hosts'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    clientReports: {
      list(input: Payload<'ClientReports'>, options?: ObservationOptions): Promise<O['ReadResult']>;
      get(input: Payload<'ClientReportDetail'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    };
  };
  configuration: {
    head(input: Payload<'ConfigurationHead'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    get(input: Payload<'ConfigurationLookup'>, options?: ObservationOptions): Promise<O['ReadResult']>;
    compare(input: Payload<'ConfigurationCompare'>, options?: ObservationOptions): Promise<O['ReadResult']>;
  };
  events: {
    describe(options?: ObservationOptions): Promise<O['ReadResult']>;
    read(input: Payload<'EventsRead'>, options?: ObservationOptions): Promise<O['ReadResult']>;
  };
}
/** @aiPurpose Connect the explicit Observation listener with no native acquisition or automatic retry. @aiUsage Preserve expected daemon/epoch/window operands across refresh and handle all returned evidence branches. */
export function createObservationClient(options: HttpOptions): ObservationClient {
  const client = createClient({ baseUrl: options.baseUrl.replace(/\/$/, ''), fetch: options.fetch, headers: options.headers, parseAs: 'text', credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer' });
  const pending = new Set<AbortController>();
  let closed = false, polling = false;
  const call = (read: Read, opts?: ObservationOptions) => api.read({ expected_daemon: opts?.expectedDaemon ?? null, read }, opts);
  const api: ObservationClient = {
    async scope(opts) { const text = await responseText(wire.observationScope({ client, body: {}, signal: opts?.signal })); return decodeResponse('ScopeRef', text, raw => parseObservationJson('ScopeRef', raw)); },
    async read(input, opts) {
      if (closed) throw new AdapterDisconnected('Observer closed');
      if (pending.size >= 8) throw new AdapterError(503, 'Busy');
      const isPoll = typeof input.read === 'object' && 'EventsRead' in input.read;
      if (isPoll && polling) throw new AdapterError(409, 'PollInFlight');
      const body = observationWireValue('ReadCall', input);
      const controller = new AbortController();
      const abort = () => controller.abort(opts?.signal?.reason);
      if (opts?.signal?.aborted) abort(); else opts?.signal?.addEventListener('abort', abort, { once: true });
      pending.add(controller); if (isPoll) polling = true;
      try {
        const text = await responseText(wire.observationRead({ client, body, signal: controller.signal }));
        return decodeResponse('ReadResult', text, raw => parseObservationJson('ReadResult', raw));
      } finally {
        pending.delete(controller); opts?.signal?.removeEventListener('abort', abort); if (isPoll) polling = false;
      }
    },
    close() { closed = true; for (const controller of pending) controller.abort(); },
    runtime: {
      snapshot: opts => call('Snapshot', opts),
      requests: { list: (input, opts) => call({ Requests: input }, opts), get: (input, opts) => call({ RequestDetail: input }, opts) },
      connections: (input, opts) => call({ Connections: input }, opts),
      peers: (input, opts) => call({ Peers: input }, opts),
      hosts: (input, opts) => call({ Hosts: input }, opts),
      clientReports: { list: (input, opts) => call({ ClientReports: input }, opts), get: (input, opts) => call({ ClientReportDetail: input }, opts) },
    },
    configuration: {
      head: (input, opts) => call({ ConfigurationHead: input }, opts),
      get: (input, opts) => call({ ConfigurationLookup: input }, opts),
      compare: (input, opts) => call({ ConfigurationCompare: input }, opts),
    },
    events: { describe: opts => call('EventsDescribe', opts), read: (input, opts) => call({ EventsRead: input }, opts) },
  };
  return api;
}
