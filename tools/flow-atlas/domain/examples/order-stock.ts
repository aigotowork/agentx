/** @aiPurpose Author a source-bound Order/StockPosition design and pure scenarios without running its native commands. */
import * as z from 'zod';
import { schema } from '../../authoring/handles.ts';
import { zodAdapter } from '../../authoring/adapters/zod.ts';
import { portableCopy } from '../../authoring/portable.ts';
import { same } from '../schema.ts';
import { bindNativeMethod } from '../binding.ts';
import type { DomainModel, DomainScenario, Evidence, InputField, ScenarioVisit, SchemaHandle, StateWrite } from '../contracts.ts';
import { BeginInput, BeginResult, ConfirmResult, Order, OrderState, RejectResult, Rejection, ReservationIntent, ReservationReceipt, ReservationRefusal, ReserveResult, StockPosition, StockState } from './native.ts';
import type { CheckoutMode, NativeCallWitness, NativeCheckoutRun, OrderSnapshot, Receipt, StockSnapshot } from './native.ts';

const handle = <T>(native: { parse(value: unknown): T }, id: string, label: string) => schema(native, zodAdapter, { id, label });
const text = z.string().min(1);
const h = {
  empty: handle(z.object({}), 'schema.empty', 'No owned state'),
  orderId: handle(text, 'schema.order-id', 'Order ID'), sku: handle(text, 'schema.sku', 'SKU'),
  quantity: handle(z.number().int().min(1), 'schema.quantity', 'Item quantity'), intentKey: handle(text, 'schema.intent-key', 'Reservation intent key'),
  reservationId: handle(text, 'schema.reservation-id', 'Reservation ID'),
  order: handle(OrderState, 'schema.order-state', 'Native Order state'), stock: handle(StockState, 'schema.stock-state', 'Native StockPosition state'),
  begin: handle(BeginInput, 'schema.begin-input', 'Begin reservation input'), intent: handle(ReservationIntent, 'schema.reservation-intent', 'Native reservation intent'),
  receipt: handle(ReservationReceipt, 'schema.reservation-receipt', 'Native reservation receipt'), refusal: handle(ReservationRefusal, 'schema.reservation-refusal', 'Native reservation refusal'),
  rejection: handle(Rejection, 'schema.rejection', 'Business refusal reason'),
  beginResult: handle(BeginResult, 'schema.begin-result', 'Native begin result'),
  reserveResult: handle(ReserveResult, 'schema.reserve-result', 'Native reserve result'),
  confirmResult: handle(ConfirmResult, 'schema.confirm-result', 'Native confirm result'),
  rejectResult: handle(RejectResult, 'schema.reject-result', 'Native reject result'),
};
// Prototype method values establish native correspondence; no receiver is instantiated or method invoked.
const methods = {
  begin: bindNativeMethod({ id: 'native.order.begin', label: 'Order.beginReservation', ownerId: 'object.order', input: h.begin, output: h.beginResult, method: Order.prototype.beginReservation }),
  reserve: bindNativeMethod({ id: 'native.stock.reserve', label: 'StockPosition.reserve', ownerId: 'object.stock', input: h.intent, output: h.reserveResult, method: StockPosition.prototype.reserve }),
  confirm: bindNativeMethod({ id: 'native.order.confirm', label: 'Order.confirm', ownerId: 'object.order', input: h.receipt, output: h.confirmResult, method: Order.prototype.confirm }),
  reject: bindNativeMethod({ id: 'native.order.reject', label: 'Order.reject', ownerId: 'object.order', input: h.refusal, output: h.rejectResult, method: Order.prototype.reject }),
};
const intentFields: readonly InputField[] = [
  { id: 'param.order-id', label: 'Order ID', path: ['orderId'], valueId: 'value.order-id' },
  { id: 'param.sku', label: 'SKU', path: ['sku'], valueId: 'value.sku' },
  { id: 'param.quantity', label: 'Quantity', path: ['quantity'], valueId: 'value.quantity' },
  { id: 'param.intent', label: 'Intent key', path: ['intentKey'], valueId: 'value.intent' },
];
const receiptFields: readonly InputField[] = [...intentFields, { id: 'param.reservation-id', label: 'Reservation ID', path: ['reservationId'], valueId: 'value.reservation-id' }];
const beginFields = intentFields.filter(field => field.path[0] === 'intentKey');
const binding = { status: 'source-bound' as const, sourceIds: ['source.native'] };
const outcome = (id: string, label: string, meaning: 'returned' | 'rejected' | 'unknown', payload?: SchemaHandle, fields?: readonly InputField[]) => ({ id, label, meaning, ...(payload ? { payload } : {}), ...(fields ? { fields } : {}) });
const reason = 'Receipt does not match the awaiting order, item, quantity and intent.';

/** @aiPurpose Construct a reusable pure source-first example with decisive authored scenarios.
 * @aiUsage compile/inspect/export call this factory without invoking any Order/StockPosition command or parse hook.
 */
export function createOrderModel(): DomainModel {
  return {
    definition: {
      id: 'order-stock-workbench', label: '订单与库存领域设计',
      metadata: { summary: '同一领域设计的对象、方法、参数、状态和场景投影', documents: [{ id: 'doc.workbench', label: 'Domain model design', uri: '../design/workbench.md' }] },
      sources: [
        { id: 'source.native', label: 'Real in-memory owning commands', uri: 'tools/flow-atlas/domain/examples/native.ts', basis: 'source' },
        { id: 'source.fixture', label: 'Authored scenario witnesses', uri: 'tools/flow-atlas/domain/examples/order-stock.ts', basis: 'design' },
        { id: 'source.transport', label: 'Explicit Checkout reply loss/corruption injection', uri: 'tools/flow-atlas/domain/examples/native.ts', basis: 'source' },
      ],
      contexts: [{ id: 'context.sales', label: '销售', responsibility: '订单意图、确认与用例组织' }, { id: 'context.inventory', label: '库存', responsibility: 'SKU 库存及预留权威状态' }],
      modules: [
        { id: 'module.orders', label: 'Order', contextId: 'context.sales', responsibility: '订单生命周期和凭据匹配', dependencies: [] },
        { id: 'module.inventory', label: 'Inventory', contextId: 'context.inventory', responsibility: '库存预留及可用量不变量', dependencies: [] },
        { id: 'module.checkout', label: 'Checkout', contextId: 'context.sales', responsibility: '调用拥有者的公开能力，保留未知', dependencies: ['module.orders', 'module.inventory'] },
      ],
      values: [
        { id: 'value.order-id', label: 'OrderId', schema: h.orderId, meaning: '持续的订单业务身份' },
        { id: 'value.sku', label: 'Sku', schema: h.sku, meaning: '库存位置的业务身份' },
        { id: 'value.quantity', label: 'Quantity', schema: h.quantity, meaning: '同一 SKU 的件数', unit: 'item' },
        { id: 'value.intent', label: 'IntentKey', schema: h.intentKey, meaning: '一次业务预留意图，不是 callId' },
        { id: 'value.reservation-id', label: 'ReservationId', schema: h.reservationId, meaning: '库存拥有者签发的预留身份' },
      ],
      objects: [
        {
          id: 'object.order', label: 'Order', moduleId: 'module.orders', kind: 'entity', responsibility: '从自己的订单行构造意图，只消费匹配的权威凭据', state: h.order,
          businessIdentity: [['orderId']], consistencyRootId: 'object.order',
          fields: [
            { id: 'field.order-id', label: 'OrderId', path: ['orderId'], valueId: 'value.order-id' }, { id: 'field.order-sku', label: 'SKU', path: ['sku'], valueId: 'value.sku' },
            { id: 'field.order-quantity', label: 'Quantity', path: ['quantity'], valueId: 'value.quantity' }, { id: 'field.order-status', label: 'Status', path: ['status'] },
            { id: 'field.order-intent', label: 'Intent', path: ['intentKey'], valueId: 'value.intent' }, { id: 'field.order-receipt', label: 'Receipt', path: ['receipt'] },
          ],
          lifecycle: { path: ['status'], transitions: [
            { id: 'transition.begin', label: 'Begin reservation', from: 'draft', to: 'awaiting', operationId: 'operation.begin', outcomeId: 'ready' },
            { id: 'transition.confirm', label: 'Confirm matching receipt', from: 'awaiting', to: 'confirmed', operationId: 'operation.confirm', outcomeId: 'confirmed' },
            { id: 'transition.reject', label: 'Record authoritative refusal', from: 'awaiting', to: 'rejected', operationId: 'operation.reject', outcomeId: 'recorded' },
          ] }, metadata: { sourceLinks: [{ label: 'Order', uri: 'tools/flow-atlas/domain/examples/native.ts' }] },
        },
        {
          id: 'object.stock', label: 'StockPosition', moduleId: 'module.inventory', kind: 'entity', responsibility: '拥有 onHand 和预留记录；available 为只读派生量', state: h.stock, businessIdentity: [['sku']], consistencyRootId: 'object.stock',
          fields: [
            { id: 'field.stock-sku', label: 'SKU', path: ['sku'], valueId: 'value.sku' }, { id: 'field.on-hand', label: 'On hand', path: ['onHand'] },
            { id: 'field.reservations', label: 'Reservations', path: ['reservations'] },
            { id: 'field.available', label: 'Available', path: ['available'], derived: { description: 'onHand − sum(reservations.quantity)', sourcePaths: [['onHand'], ['reservations']], projection: { kind: 'subtract-sum', totalPath: ['onHand'], itemsPath: ['reservations'], itemPath: ['quantity'] } } },
          ], metadata: { sourceLinks: [{ label: 'StockPosition', uri: 'tools/flow-atlas/domain/examples/native.ts' }] },
        },
        { id: 'object.checkout', label: 'Checkout', moduleId: 'module.checkout', kind: 'process', responsibility: '组织调用；不直接改订单或库存字段', state: h.empty, fields: [] },
      ],
      rules: [
        { id: 'rule.stock-nonnegative', label: 'On hand ≥ 0', ownerId: 'object.stock', description: '实物库存非负；预留不修改 onHand。', basis: 'source-bound', sourceIds: ['source.native'], check: { left: { kind: 'state', path: ['onHand'] }, operator: 'gte', right: { kind: 'literal', value: 0 } } },
        { id: 'rule.available', label: 'Available ≥ 0', ownerId: 'object.stock', description: '原生 reserve 在提交前检查可用量，编译器不执行该谓词；独立 native cases 验证。', basis: 'source-bound', sourceIds: ['source.native'] },
        ...(['orderId', 'sku', 'quantity', 'intentKey'] as const).map(key => ({ id: `rule.receipt-${key}`, label: `Receipt ${key} matches`, ownerId: 'object.order', description: `确认必须与订单自己的 ${key} 一致。`, basis: 'source-bound' as const, sourceIds: ['source.native'], outcomeIds: ['confirmed'], check: { left: { kind: 'input' as const, path: [key] }, operator: 'eq' as const, right: { kind: 'state' as const, path: [key] } } })),
      ],
      operations: [
        { id: 'operation.begin', label: 'beginReservation', ownerId: 'object.order', kind: 'command', input: h.begin, inputFields: beginFields, outcomes: [outcome('ready', 'Ready', 'returned', h.intent, intentFields), outcome('begin-refused', 'Cannot begin', 'rejected', h.rejection)], effects: [{ outcomeId: 'ready', fieldPaths: [['status'], ['intentKey']] }], ruleIds: [], binding, native: methods.begin },
        { id: 'operation.reserve', label: 'reserve', ownerId: 'object.stock', kind: 'command', input: h.intent, inputFields: intentFields, outcomes: [outcome('reserved', 'Reserved', 'returned', h.receipt, receiptFields), outcome('refused', 'Insufficient inventory', 'rejected', h.refusal, intentFields), outcome('unknown', 'Reply not received', 'unknown')], effects: [{ outcomeId: 'reserved', fieldPaths: [['reservations']] }], ruleIds: ['rule.stock-nonnegative', 'rule.available'], binding, native: methods.reserve },
        { id: 'operation.confirm', label: 'confirm', ownerId: 'object.order', kind: 'command', input: h.receipt, inputFields: receiptFields, outcomes: [outcome('confirmed', 'Confirmed', 'returned', h.receipt, receiptFields), outcome('invalid-receipt', 'Receipt mismatch', 'rejected', h.rejection)], effects: [{ outcomeId: 'confirmed', fieldPaths: [['status'], ['receipt']] }], ruleIds: ['rule.receipt-orderId', 'rule.receipt-sku', 'rule.receipt-quantity', 'rule.receipt-intentKey'], binding, native: methods.confirm },
        { id: 'operation.reject', label: 'reject', ownerId: 'object.order', kind: 'command', input: h.refusal, inputFields: intentFields, outcomes: [outcome('recorded', 'Rejection recorded', 'returned'), outcome('invalid-refusal', 'Refusal mismatch', 'rejected', h.rejection)], effects: [{ outcomeId: 'recorded', fieldPaths: [['status']] }], ruleIds: [], binding, native: methods.reject },
      ],
      relationships: [{ id: 'relation.checkout-order', label: 'Calls order commands', fromObjectId: 'object.checkout', toObjectId: 'object.order', kind: 'uses' }, { id: 'relation.checkout-stock', label: 'Requests inventory', fromObjectId: 'object.checkout', toObjectId: 'object.stock', kind: 'uses' }, { id: 'relation.order-stock', label: 'Receipt refers to reservation', fromObjectId: 'object.order', toObjectId: 'object.stock', kind: 'association', cardinality: '0..1' }],
      processes: [{
        id: 'process.checkout', label: 'Checkout', input: h.begin, inputFields: beginFields,
        roles: [{ id: 'role.checkout', label: 'Use case', objectId: 'object.checkout' }, { id: 'role.order', label: 'Selected order', objectId: 'object.order' }, { id: 'role.stock', label: 'SKU stock', objectId: 'object.stock' }],
        startId: 'start', steps: [
          { id: 'start', label: 'Start', kind: 'start' },
          { id: 'begin', label: 'Build owned intent', kind: 'operation', roleId: 'role.order', callerRoleId: 'role.checkout', operationId: 'operation.begin', bindings: [{ id: 'binding.begin', label: 'Use case input → begin', targetPath: [], source: { kind: 'input', path: [] } }] },
          { id: 'reserve', label: 'Reserve inventory', kind: 'operation', roleId: 'role.stock', callerRoleId: 'role.checkout', operationId: 'operation.reserve', bindings: [{ id: 'binding.reserve', label: 'begin.ready → reserve', targetPath: [], source: { kind: 'result', stepId: 'begin', outcomeId: 'ready', path: [] } }] },
          { id: 'confirm', label: 'Confirm receipt', kind: 'operation', roleId: 'role.order', callerRoleId: 'role.checkout', operationId: 'operation.confirm', bindings: [{ id: 'binding.confirm', label: 'reserve.reserved → confirm', targetPath: [], source: { kind: 'result', stepId: 'reserve', outcomeId: 'reserved', path: [] } }] },
          { id: 'reject', label: 'Record stock refusal', kind: 'operation', roleId: 'role.order', callerRoleId: 'role.checkout', operationId: 'operation.reject', bindings: [{ id: 'binding.reject', label: 'reserve.refused → reject', targetPath: [], source: { kind: 'result', stepId: 'reserve', outcomeId: 'refused', path: [] } }] },
          { id: 'completed', label: 'Confirmed', kind: 'end', outcome: 'completed' }, { id: 'rejected', label: 'Rejected', kind: 'end', outcome: 'rejected' }, { id: 'unknown', label: 'Caller uncertain', kind: 'end', outcome: 'unknown' },
        ],
        flows: [
          { id: 'flow.start', label: 'Begin', from: 'start', to: 'begin' }, { id: 'flow.ready', label: 'Ready', from: 'begin', to: 'reserve', outcomeId: 'ready' }, { id: 'flow.begin-refused', label: 'Cannot begin', from: 'begin', to: 'rejected', outcomeId: 'begin-refused' },
          { id: 'flow.reserved', label: 'Receipt received', from: 'reserve', to: 'confirm', outcomeId: 'reserved' }, { id: 'flow.refused', label: 'Stock refused', from: 'reserve', to: 'reject', outcomeId: 'refused' }, { id: 'flow.unknown', label: 'No reply', from: 'reserve', to: 'unknown', outcomeId: 'unknown' },
          { id: 'flow.confirmed', label: 'Matching receipt', from: 'confirm', to: 'completed', outcomeId: 'confirmed' }, { id: 'flow.invalid-receipt', label: 'Receipt mismatch', from: 'confirm', to: 'rejected', outcomeId: 'invalid-receipt' },
          { id: 'flow.recorded', label: 'Refusal recorded', from: 'reject', to: 'rejected', outcomeId: 'recorded' }, { id: 'flow.invalid-refusal', label: 'Refusal mismatch', from: 'reject', to: 'rejected', outcomeId: 'invalid-refusal' },
        ],
      }],
    },
    scenarios: (['success', 'refusal', 'lost-response', 'wrong-receipt', 'stale-receipt', 'quantity-mismatch'] as const).map(mode => authoredScenario(mode)),
  };
}

function projectRun(run: NativeCheckoutRun, evidence: Evidence): DomainScenario {
  const initialOrder = run.calls.find(call => call.stepId === 'begin')!.before;
  const initialStock = run.calls.find(call => call.stepId === 'reserve')!.before;
  const visits: ScenarioVisit[] = run.calls.map((call, index) => {
    const instanceId = call.stepId === 'reserve' ? 'stock-book' : 'order-42-view';
    const writes: StateWrite[] = Object.entries(call.after).filter(([key, value]) => !same(value, (call.before as unknown as Record<string, unknown>)[key])).map(([key, value]) => ({ instanceId, path: [key], value: portableCopy(value), ...(key === 'status' ? { transitionId: `transition.${call.stepId}` } : {}) }));
    return { id: `${run.mode}:visit:${index}`, label: call.stepId, stepId: call.stepId, callId: `${run.mode}:call:${index}`, input: portableCopy(call.input), completion: call.completion, outcomeId: call.outcomeId, ...(Object.hasOwn(call, 'payload') ? { payload: portableCopy(call.payload) } : {}), evidence, ...(writes.length ? { effect: { outcomeId: call.ownerOutcomeId!, writes, evidence } } : {}) };
  });
  const labels: Record<CheckoutMode, string> = { success: '成功：匹配凭据', refusal: '拒绝：库存不足', 'lost-response': '已提交，但回复丢失', 'wrong-receipt': '拒绝：其他订单的凭据', 'stale-receipt': '拒绝：旧意图凭据', 'quantity-mismatch': '拒绝：凭据数量不匹配' };
  return {
    id: `scenario.${run.mode}`, label: labels[run.mode], processId: 'process.checkout', input: { intentKey: 'intent-42' },
    metadata: { summary: run.mode.includes('receipt') || run.mode === 'quantity-mismatch' ? '显式注入传输凭据错误；库存拥有者实际提交的凭据保持原样。' : '订单业务状态与调用方所知分别保留。' },
    instances: [
      { id: 'order-42-view', label: 'Order 42', objectId: 'object.order', businessIdentity: 'order-42', initial: portableCopy(initialOrder) },
      { id: 'order-43-view', label: 'Order 43', objectId: 'object.order', businessIdentity: 'order-43', initial: portableCopy(run.otherOrder) },
      { id: 'stock-book', label: 'Book inventory', objectId: 'object.stock', businessIdentity: 'book', initial: portableCopy(initialStock) },
      { id: 'checkout-run', label: 'Checkout', objectId: 'object.checkout', initial: {} },
    ], roles: { 'role.order': 'order-42-view', 'role.stock': 'stock-book', 'role.checkout': 'checkout-run' }, visits, terminalStepId: run.outcome === 'completed' ? 'completed' : run.outcome === 'unknown' ? 'unknown' : 'rejected',
    expectations: [
      { id: 'expect.order-status', label: 'Order status', kind: 'state', instanceId: 'order-42-view', path: ['status'], equals: run.order.status },
      { id: 'expect.other-order', label: 'Second instance unchanged', kind: 'state', instanceId: 'order-43-view', path: [], equals: portableCopy(run.otherOrder) },
      { id: 'expect.on-hand', label: 'onHand stays 20', kind: 'state', instanceId: 'stock-book', path: ['onHand'], equals: 20 },
      { id: 'expect.reservations', label: 'Actual reservation authority', kind: 'state', instanceId: 'stock-book', path: ['reservations'], equals: portableCopy(run.stock.reservations) },
      { id: 'expect.knowledge', label: 'Caller reservation knowledge', kind: 'call', visitId: visits[1].id, outcomeId: run.mode === 'lost-response' ? 'unknown' : run.mode === 'refusal' ? 'refused' : 'reserved' },
      { id: 'expect.terminal', label: 'Use case outcome', kind: 'terminal', outcome: run.outcome },
    ],
  };
}
/** @aiPurpose Explicitly import real bounded native witnesses while preserving observed provenance.
 * @aiUsage Call runCheckout yourself first; this pure adapter never invokes native owners or promotes authored fixtures.
 */
export function scenarioFromNativeRun(run: NativeCheckoutRun): DomainScenario {
  const actual = projectRun(run, { kind: 'observed', sourceIds: ['source.native', 'source.transport'], recordedAt: run.recordedAt });
  return { ...actual, expectations: authoredScenario(run.mode).expectations };
}

/** @aiPurpose Build inert authored cases separately from actual native execution.
 * @aiUsage Suitable for offline explanation; evidence remains authored, even when expected values match native behavior.
 */
export function authoredScenario(mode: CheckoutMode): DomainScenario {
  const quantity = mode === 'refusal' ? 21 : 2;
  const draft: OrderSnapshot = { orderId: 'order-42', sku: 'book', quantity, status: 'draft', intentKey: null, receipt: null };
  const awaiting: OrderSnapshot = { ...draft, status: 'awaiting', intentKey: 'intent-42' };
  const intent = { orderId: 'order-42', sku: 'book', quantity, intentKey: 'intent-42' };
  const realReceipt: Receipt = { ...intent, reservationId: 'reservation:intent-42' };
  const initialStock: StockSnapshot = { sku: 'book', onHand: 20, reservations: [] };
  const committed: StockSnapshot = { ...initialStock, reservations: [realReceipt] };
  const calls: NativeCallWitness[] = [{ stepId: 'begin', input: { intentKey: 'intent-42' }, completion: 'reply', outcomeId: 'ready', payload: intent, ownerOutcomeId: 'ready', before: draft, after: awaiting }];
  let order = awaiting; let stock = committed; let terminal: NativeCheckoutRun['outcome'] = 'completed';
  if (mode === 'lost-response') {
    calls.push({ stepId: 'reserve', input: intent, completion: 'local-outcome', outcomeId: 'unknown', ownerOutcomeId: 'reserved', before: initialStock, after: committed }); terminal = 'unknown';
  } else if (mode === 'refusal') {
    const refusal = { ...intent, reason: 'Insufficient inventory.' };
    stock = initialStock; order = { ...awaiting, status: 'rejected' }; terminal = 'rejected';
    calls.push({ stepId: 'reserve', input: intent, completion: 'reply', outcomeId: 'refused', payload: refusal, ownerOutcomeId: 'refused', before: initialStock, after: initialStock }, { stepId: 'reject', input: refusal, completion: 'reply', outcomeId: 'recorded', ownerOutcomeId: 'recorded', before: awaiting, after: order });
  } else {
    const receipt = { ...realReceipt, ...(mode === 'wrong-receipt' ? { orderId: 'order-43' } : {}), ...(mode === 'stale-receipt' ? { intentKey: 'old-intent-42' } : {}), ...(mode === 'quantity-mismatch' ? { quantity: 3 } : {}) };
    const accepted = mode === 'success'; order = accepted ? { ...awaiting, status: 'confirmed', receipt } : awaiting; terminal = accepted ? 'completed' : 'rejected';
    calls.push({ stepId: 'reserve', input: intent, completion: 'reply', outcomeId: 'reserved', payload: receipt, ownerOutcomeId: 'reserved', before: initialStock, after: committed }, { stepId: 'confirm', input: receipt, completion: 'reply', outcomeId: accepted ? 'confirmed' : 'invalid-receipt', payload: accepted ? receipt : { reason }, ownerOutcomeId: accepted ? 'confirmed' : 'invalid-receipt', before: awaiting, after: order });
  }
  return projectRun({ mode, order, stock, otherOrder: { orderId: 'order-43', sku: 'book', quantity: 1, status: 'draft', intentKey: null, receipt: null }, available: mode === 'refusal' ? 20 : 18, outcome: terminal, calls, recordedAt: '' }, { kind: 'authored', explanation: 'Pure authored fixture; no native handler executed.' });
}
