/** @aiPurpose Author an inspectable order/inventory graph from imported native schemas and method handles.
 * Default CLI export is pure inspection; observeOrder is an explicit runtime example and is never auto-run.
 */
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  bindStep, compile, defineModule, defineObject, defineOperation, defineProcess, flow, observeProcess, schema, step,
} from '../index.ts';
import type { AuthoringModel } from '../contracts.ts';
import { zodAdapter } from '../adapters/zod.ts';
import { OrderRequest, ReserveRequest, ReserveResult, prepareOrder, reserveInventory } from './native-contracts.ts';
import type { OrderInput } from './native-contracts.ts';

/** @aiPurpose Compose native contract handles with metadata, two modules and all handled outcomes.
 * @aiUsage Reuse returned bindings in your implementation and model in inspect/compile; construction invokes no business code.
 */
export function createOrderExample(options: { readonly allowInventoryDependency?: boolean; readonly dependencyCycle?: boolean; readonly cyclePolicy?: 'warning' | 'error' | 'allow' } = {}) {
  const orderModule = defineModule({
    id: 'orders', label: 'Orders', domainId: 'commerce', dependencies: options.allowInventoryDependency === false ? [] : ['inventory'],
    metadata: { responsibility: 'Prepare order requests and ask inventory to reserve stock.', tags: ['application'] },
  });
  const inventoryModule = defineModule({
    id: 'inventory', label: 'Inventory', domainId: 'commerce', dependencies: options.dependencyCycle ? ['orders'] : [],
    metadata: { responsibility: 'Own available stock and reservation results.', tags: ['domain'] },
  });
  const order = defineObject({ id: 'order-service', label: 'Order service', module: orderModule, kind: 'service' });
  const inventory = defineObject({ id: 'inventory-service', label: 'Inventory service', module: inventoryModule, kind: 'service' });
  const request = schema(OrderRequest, zodAdapter, { id: 'order-request', label: 'Order request' });
  const reservationRequest = schema(ReserveRequest, zodAdapter, { id: 'reservation-request', label: 'Reservation request' });
  const reservationResult = schema(ReserveResult, zodAdapter, { id: 'reservation-result', label: 'Reservation result' });
  const prepare = defineOperation({
    id: 'orders.prepare', label: 'prepareOrder', owner: order, kind: 'command', input: request, output: reservationRequest,
    method: prepareOrder, classify: () => 'ready', failureOutcomeId: 'failed',
    outcomes: [{ id: 'ready', label: 'Prepared', meaning: 'returned' }, { id: 'failed', label: 'Preparation failed', meaning: 'failed' }],
    metadata: { summary: 'Prepare an order using the native request contract.', sourceLinks: [{ label: 'Native methods', uri: 'tools/flow-atlas/authoring/examples/native-contracts.ts' }] },
  });
  const reserve = defineOperation({
    id: 'inventory.reserve', label: 'reserveInventory', owner: inventory, kind: 'command', input: reservationRequest, output: reservationResult,
    method: reserveInventory, classify: result => result.kind, failureOutcomeId: 'failed', unknownOutcomeId: 'unknown', noResponseOutcomeId: 'no-response',
    outcomes: [
      { id: 'reserved', label: 'Reserved', meaning: 'returned' }, { id: 'rejected', label: 'Insufficient stock', meaning: 'rejected' },
      { id: 'unknown', label: 'Result unknown', meaning: 'unknown' }, { id: 'failed', label: 'Invocation failed', meaning: 'failed' },
      { id: 'no-response', label: 'No response observed', meaning: 'no-response' },
    ],
    metadata: {
      responsibility: 'Return an authoritative result or preserve uncertainty.',
      sourceLinks: [{ label: 'Native schemas and method', uri: 'tools/flow-atlas/authoring/examples/native-contracts.ts' }],
      documents: [{ id: 'source-binding', label: 'Design/code correspondence', uri: 'tools/flow-atlas/design/source-binding.md' }],
      extensions: { 'example:scope': { storage: 'native in-memory inventory', concurrency: 'sequential' } },
    },
  });
  const begin = step({ id: 'start', label: 'Order requested', kind: 'start' });
  const prepareStep = step({ id: 'prepare-order', label: 'Prepare order', kind: 'operation', caller: order, operation: prepare });
  const reserveStep = step({ id: 'reserve-stock', label: 'Reserve stock', kind: 'operation', caller: order, operation: reserve });
  const completed = step({ id: 'reserved', label: 'Reservation confirmed', kind: 'end', outcome: 'completed' });
  const rejected = step({ id: 'rejected', label: 'Reservation rejected', kind: 'end', outcome: 'rejected' });
  const unknown = step({ id: 'unknown', label: 'Reservation result unknown', kind: 'end', outcome: 'unknown' });
  const failed = step({ id: 'failed', label: 'Invocation failed', kind: 'end', outcome: 'failed' });
  const process = defineProcess({
    id: 'place-order', label: 'Place order and reserve inventory', start: begin,
    steps: [begin, prepareStep, reserveStep, completed, rejected, unknown, failed],
    flows: [
      flow({ id: 'begin', label: 'Begin', from: begin, to: prepareStep }),
      flow({ id: 'prepared', label: 'Ready', from: prepareStep, to: reserveStep, outcomeId: 'ready' }),
      flow({ id: 'prepare-failed', label: 'Preparation failed', from: prepareStep, to: failed, outcomeId: 'failed' }),
      flow({ id: 'reserved', label: 'Confirmed', from: reserveStep, to: completed, outcomeId: 'reserved' }),
      flow({ id: 'rejected', label: 'Rejected', from: reserveStep, to: rejected, outcomeId: 'rejected' }),
      flow({ id: 'unknown', label: 'Unknown', from: reserveStep, to: unknown, outcomeId: 'unknown' }),
      flow({ id: 'failed', label: 'Failed', from: reserveStep, to: failed, outcomeId: 'failed' }),
      flow({ id: 'no-response', label: 'No response', from: reserveStep, to: unknown, outcomeId: 'no-response' }),
    ],
    metadata: { summary: 'Explicit order/inventory boundary, including rejection and uncertain results.' },
  });
  const prepareBinding = bindStep(prepareStep, prepare);
  const reserveBinding = bindStep(reserveStep, reserve);
  const model: AuthoringModel = {
    id: 'order-inventory', label: 'Order / inventory source design',
    domains: [{ id: 'commerce', label: 'Commerce' }], modules: [orderModule, inventoryModule], objects: [order, inventory],
    operations: [prepare, reserve], processes: [process], bindings: [prepareBinding, reserveBinding],
    policy: { dependencyCycles: options.cyclePolicy ?? 'warning' },
    metadata: { summary: 'Schemas and handlers are imported by identity; no business method runs during export.' },
  };
  return { model, process, orderModule, inventoryModule, order, inventory, prepare, reserve, prepareStep, reserveStep, prepareBinding, reserveBinding };
}

/** @aiPurpose Demonstrate ordinary consumer-controlled business calls through the exact authored bindings.
 * @aiUsage Call explicitly with a native order request; no graph scheduling or automatic branch execution occurs.
 */
export async function observeOrder(input: OrderInput, options: { readonly noResponse?: boolean } = {}) {
  const example = createOrderExample();
  const observation = observeProcess(example.process, example.model.bindings);
  try {
    const request = await observation.invoke(example.prepareBinding, input);
    if (options.noResponse) observation.record(example.reserveBinding, 'no-response', 'Consumer ended observation before any response; no reservation result is claimed.');
    else await observation.invoke(example.reserveBinding, request);
  } catch {
    // The wrapper records failure before rethrowing. This example returns the retained report.
  }
  return { trace: observation.trace(), report: observation.check({ complete: true }) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const result = compile(createOrderExample().model);
  if (!result.ok) { process.stderr.write(`${JSON.stringify(result.diagnostics, null, 2)}\n`); process.exitCode = 1; }
  else {
    const json = `${JSON.stringify(result.value, null, 2)}\n`;
    if (process.argv[2]) writeFileSync(resolve(process.argv[2]), json, { flag: 'wx' });
    else process.stdout.write(json);
  }
}
