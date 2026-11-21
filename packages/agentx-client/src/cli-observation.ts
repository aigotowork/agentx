/** @aiPurpose Bind all passive Observation commands while preserving exact daemon, epoch, window and evidence results. */
import { createObservationClient } from './observation.js';
import type { ObservationTypes as O } from './observation.js';
import { parseObservationJson } from './codec.js';
import { UsageError, required, integer, inputText, restrict } from './cli-runtime.js';
import type { CliContext, CliResult } from './cli-runtime.js';

/** @aiPurpose Execute the selected passive read without daemon acquisition or automatic rebase. @aiUsage Reference/window files use actual marker JSON; LocalFailure and native stale/gap results remain values. */
export async function runObservation(ctx: CliContext): Promise<CliResult> {
  const v = ctx.values, [, group, action, left, right] = ctx.positionals;
  const api = createObservationClient({ baseUrl: ctx.url('observation'), fetch: ctx.fetch });
  const file = async <K extends keyof O>(name: K, path: string | undefined) => parseObservationJson(name, await inputText(required(path, `${name} FILE|-`)));
  const expected = async () => v['expected-daemon'] ? await file('DaemonRef', v['expected-daemon']) : null;
  const result = (value: O['ReadResult']): CliResult => ({ value, schema: 'ReadResult', domain: 'observation' });
  const page = (): O['RuntimeList'] => ({ before: v.before ? integer(v.before, '--before') : null, limit: v.limit ? integer(v.limit, '--limit') : 32n });
  const options = async () => ({ signal: ctx.signal, expectedDaemon: await expected() });
  const common = ['expected-daemon'];
  try {
    if (group === 'scope') { restrict(ctx, [], 2); return { value: await api.scope({ signal: ctx.signal }), schema: 'ScopeRef', domain: 'observation' }; }
    if (group === 'snapshot') { restrict(ctx, common, 2); return result(await api.runtime.snapshot(await options())); }
    if (group === 'requests' && action === 'list') {
      restrict(ctx, [...common, 'selection', 'before', 'limit'], 3);
      return result(await api.runtime.requests.list({ select: v.selection ? await file('RequestSelection', v.selection) : 'AllRetained', page: page() }, await options()));
    }
    if (group === 'requests' && action === 'show') { restrict(ctx, common, 4); return result(await api.runtime.requests.get(await file('RequestObservationRef', left), await options())); }
    if (group === 'connections' || group === 'peers' || group === 'hosts') {
      restrict(ctx, [...common, 'before', 'limit'], 2); return result(await api.runtime[group](page(), await options()));
    }
    if (group === 'reports' && action === 'list') { restrict(ctx, [...common, 'before', 'limit'], 3); return result(await api.runtime.clientReports.list(page(), await options())); }
    if (group === 'reports' && action === 'show') { restrict(ctx, common, 4); return result(await api.runtime.clientReports.get(await file('ClientReportRef', left), await options())); }
    if (group === 'config' && action === 'head') {
      restrict(ctx, [...common, 'expected-epoch'], 3); return result(await api.configuration.head({ expected: v['expected-epoch'] ? await file('ObservationEpoch', v['expected-epoch']) : null }, await options()));
    }
    if (group === 'config' && action === 'show') { restrict(ctx, common, 4); return result(await api.configuration.get(await file('ConfigSnapshotRef', left), await options())); }
    if (group === 'config' && action === 'compare') {
      restrict(ctx, [...common, 'field'], 5);
      // ConfigFieldId itself is a native tagged value, so each --field is JSON, not an invented dotted selector grammar.
      const fields: O['ConfigFieldSelection'] = v.field ? { Only: v.field.map(field => parseObservationJson('ConfigFieldId', field)) } : 'All';
      return result(await api.configuration.compare({ left: await file('ConfigSnapshotRef', left), right: await file('ConfigSnapshotRef', right), fields }, await options()));
    }
    if (group === 'events' && action === 'describe') { restrict(ctx, common, 3); return result(await api.events.describe(await options())); }
    if (group === 'events' && action === 'read') {
      restrict(ctx, [...common, 'window', 'limit'], 3);
      return result(await api.events.read({ window: await file('ObservationWindow', v.window), limit: v.limit ? integer(v.limit, '--limit') : 128n }, await options()));
    }
    throw new UsageError('Unknown Observation command; use --help');
  } finally { api.close(); }
}
