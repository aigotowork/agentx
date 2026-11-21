/** @aiPurpose Supply callable design and trace case inputs for the controller's independent acceptance.
 * No assertions, compiler runs or handler invocations occur at import or case construction time.
 */
import type { AuthoringModel, ObservedStep } from '../contracts.ts';
import { defineOperation, defineProcess, step } from '../index.ts';
import { prepareOrder } from './native-contracts.ts';
import { createOrderExample } from './order-inventory.ts';
import * as z from 'zod';
import { OrderRequest } from './native-contracts.ts';

/** @aiPurpose Construct meaningful structural mutations against the native two-module example.
 * @aiUsage Controller calls compile/inspect on each model and checks expected codes and severity.
 */
export function designCases(): readonly { readonly name: string; readonly model: AuthoringModel; readonly expectedCode?: string; readonly severity?: 'error' | 'warning' }[] {
  const example = createOrderExample();
  const { model, process } = example;
  const withProcess = (next: typeof process): AuthoringModel => ({ ...model, processes: [next] });
  const unreachable = step({ id: 'unused', label: 'Unreachable terminal', kind: 'end', outcome: 'unknown' });
  const duplicate = defineOperation({ ...example.prepare, id: 'inventory.duplicate', owner: example.inventory, method: prepareOrder, classify: () => 'ready' });
  return [
    { name: 'native-two-module-success-design', model },
    { name: 'removed-operation', model: { ...model, operations: model.operations.filter(item => item !== example.reserve) }, expectedCode: 'BROKEN_REF', severity: 'error' },
    { name: 'unbound-step', model: { ...model, bindings: [example.prepareBinding] }, expectedCode: 'UNBOUND_STEP', severity: 'error' },
    { name: 'wrong-operation-at-stable-step', model: { ...model, bindings: [example.prepareBinding, { step: example.reserveStep, operation: example.prepare }] }, expectedCode: 'WRONG_OPERATION_BINDING', severity: 'error' },
    { name: 'undeclared-module-call', model: createOrderExample({ allowInventoryDependency: false }).model, expectedCode: 'FORBIDDEN_MODULE_CALL', severity: 'error' },
    { name: 'dependency-cycle-default', model: createOrderExample({ dependencyCycle: true }).model, expectedCode: 'DEPENDENCY_CYCLE', severity: 'warning' },
    { name: 'dependency-cycle-strict', model: createOrderExample({ dependencyCycle: true, cyclePolicy: 'error' }).model, expectedCode: 'DEPENDENCY_CYCLE', severity: 'error' },
    { name: 'dependency-cycle-allowed', model: createOrderExample({ dependencyCycle: true, cyclePolicy: 'allow' }).model },
    { name: 'duplicate-native-method-ownership', model: { ...model, operations: [...model.operations, duplicate] }, expectedCode: 'DUPLICATE_OPERATION_OWNERSHIP', severity: 'error' },
    { name: 'unreachable-step', model: withProcess(defineProcess({ ...process, steps: [...process.steps, unreachable] })), expectedCode: 'UNREACHABLE_STEP', severity: 'error' },
    { name: 'dead-end-step', model: withProcess(defineProcess({ ...process, flows: process.flows.filter(item => item.from !== example.reserveStep) })), expectedCode: 'DEAD_END_STEP', severity: 'error' },
    { name: 'unhandled-result', model: withProcess(defineProcess({ ...process, flows: process.flows.filter(item => item.id !== 'rejected') })), expectedCode: 'UNHANDLED_OUTCOME', severity: 'error' },
    { name: 'unsafe-document-reference', model: { ...model, metadata: { documents: [{ id: 'unsafe', label: 'Unsafe', uri: 'javascript:alert(1)' }] } }, expectedCode: 'UNSAFE_DOCUMENT_URI', severity: 'error' },
  ];
}

/** @aiPurpose Supply explicit recorded-path examples without fabricating live execution evidence.
 * @aiUsage Pass each authored trace to checkTrace on createOrderExample().process; these are case inputs only.
 */
export function traceCases(): readonly { readonly name: string; readonly trace: readonly ObservedStep[]; readonly complete: boolean; readonly expectedCode?: string }[] {
  const prepared: ObservedStep = { sequence: 1, stepId: 'prepare-order', operationId: 'orders.prepare', status: 'returned', outcomeId: 'ready' };
  const reserved: ObservedStep = { sequence: 2, stepId: 'reserve-stock', operationId: 'inventory.reserve', status: 'returned', outcomeId: 'reserved' };
  return [
    { name: 'reserved', trace: [prepared, reserved], complete: true },
    { name: 'rejected', trace: [prepared, { ...reserved, outcomeId: 'rejected' }], complete: true },
    { name: 'unknown', trace: [prepared, { ...reserved, status: 'unknown', outcomeId: 'unknown', reason: 'Connection lost.' }], complete: true },
    { name: 'no-response', trace: [prepared, { ...reserved, status: 'no-response', outcomeId: 'no-response', reason: 'Observation ended before response.' }], complete: true },
    { name: 'failed', trace: [prepared, { ...reserved, status: 'failed', outcomeId: 'failed', reason: 'Native method threw.' }], complete: true },
    { name: 'prefix', trace: [prepared], complete: false },
    { name: 'unfinished-complete-request', trace: [prepared], complete: true, expectedCode: 'TRACE_INCOMPLETE' },
    { name: 'wrong-order', trace: [{ ...reserved, sequence: 1 }], complete: true, expectedCode: 'TRACE_ORDER' },
    { name: 'wrong-operation', trace: [prepared, { ...reserved, operationId: 'orders.prepare' }], complete: true, expectedCode: 'TRACE_OPERATION' },
    { name: 'unknown-is-not-success', trace: [prepared, { ...reserved, status: 'unknown', reason: 'Lost response.' }], complete: true, expectedCode: 'TRACE_STATUS' },
    { name: 'unresolved-missing-response', trace: [prepared, { ...reserved, status: 'no-response', outcomeId: undefined, reason: 'No response.' }], complete: true, expectedCode: 'TRACE_OUTCOME' },
    { name: 'undeclared-outcome', trace: [prepared, { ...reserved, outcomeId: 'invented' }], complete: true, expectedCode: 'TRACE_OUTCOME' },
    { name: 'sequence-gap', trace: [prepared, { ...reserved, sequence: 3 }], complete: true, expectedCode: 'TRACE_SEQUENCE' },
  ];
}

/** @aiPurpose Supply schema conversion cases with counters that reveal accidental author callback execution.
 * @aiUsage Controller converts each schema in both directions and checks expectedCode plus callbackCount() === 0.
 */
export function schemaCases() {
  let callbacks = 0;
  const cases: { name: string; native: z.ZodType; expectedCode?: string }[] = [
    { name: 'native-request', native: OrderRequest },
    { name: 'bigint', native: z.bigint(), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'date', native: z.date(), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'transform', native: z.string().transform(value => { callbacks++; return value.length; }), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'refinement', native: z.string().refine(() => { callbacks++; return true; }), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'dynamic-default', native: z.string().default(() => { callbacks++; return 'fallback'; }), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'dynamic-catch', native: z.string().catch(() => { callbacks++; return 'fallback'; }), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'lazy', native: z.lazy(() => { callbacks++; return z.string(); }), expectedCode: 'UNSUPPORTED_SCHEMA' },
    { name: 'nested-shape-getter', native: z.object({ get value() { callbacks++; return z.string(); } }), expectedCode: 'UNSUPPORTED_SCHEMA' },
  ];
  return { cases, callbackCount: () => callbacks };
}
