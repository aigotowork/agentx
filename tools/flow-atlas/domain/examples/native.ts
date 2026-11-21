/** @aiPurpose Own isolated real in-memory Order, StockPosition and Checkout behavior; imports do not create stock or run methods. */
import * as z from 'zod';
import { OrderRequest } from '../../authoring/examples/native-contracts.ts';

/** @aiPurpose Extend the existing native order contract with an explicit reservation intent.
 * @aiUsage Reuse this exact schema for the inventory command and successful begin payload.
 */
export const ReservationIntent = OrderRequest.extend({ intentKey: z.string().min(1) });
/** @aiPurpose Define an explicit matching receipt, including order, item, quantity and intent identities.
 * @aiUsage Order.confirm validates this entire native contract and its business correspondence.
 */
export const ReservationReceipt = ReservationIntent.extend({ reservationId: z.string().min(1) });
/** @aiPurpose Define a refusal carrying the exact intent it rejects.
 * @aiUsage Order.reject only consumes an authoritative refusal matching its pending intent.
 */
export const ReservationRefusal = ReservationIntent.extend({ reason: z.string().min(1) });
/** @aiPurpose Define begin input separately from the order-owned quantity/item data.
 * @aiUsage Callers supply an intent key; Order builds the reservation request from its own line.
 */
export const BeginInput = z.object({ intentKey: z.string().min(1) });
/** @aiPurpose Define a portable native reason for owner refusal.
 * @aiUsage Rejected begin/confirm/reject outcomes reuse this explicit payload schema.
 */
export const Rejection = z.object({ reason: z.string().min(1) });
/** @aiPurpose Define persisted Order state once for native behavior and source-first inspection.
 * @aiUsage available inventory and transport uncertainty are not Order state fields.
 */
export const OrderState = OrderRequest.extend({ status: z.enum(['draft', 'awaiting', 'confirmed', 'rejected']), intentKey: z.string().nullable(), receipt: ReservationReceipt.nullable() });
/** @aiPurpose Define stock authority as onHand plus reservation records.
 * @aiUsage available is computed from this state, never persisted or independently patched.
 */
export const StockState = z.object({ sku: OrderRequest.shape.sku, onHand: z.number().int().min(0), reservations: z.array(ReservationReceipt) });
/** @aiPurpose Define the real begin result and its refusal branch.
 * @aiUsage Branch projections in the design point to these exact underlying payload schemas.
 */
export const BeginResult = z.discriminatedUnion('kind', [z.object({ kind: z.literal('ready'), intent: ReservationIntent }), z.object({ kind: z.literal('rejected'), reason: z.string() })]);
/** @aiPurpose Define the real inventory result; local transport uncertainty is recorded by Checkout separately.
 * @aiUsage A reserved return is a validated receipt; a refused return includes its intent.
 */
export const ReserveResult = z.discriminatedUnion('kind', [z.object({ kind: z.literal('reserved'), receipt: ReservationReceipt }), z.object({ kind: z.literal('rejected'), refusal: ReservationRefusal })]);
/** @aiPurpose Define native confirmation acceptance versus business refusal.
 * @aiUsage Invalid correspondence returns rejected without changing Order state.
 */
export const ConfirmResult = z.discriminatedUnion('kind', [z.object({ kind: z.literal('confirmed'), receipt: ReservationReceipt }), z.object({ kind: z.literal('rejected'), reason: z.string() })]);
/** @aiPurpose Define native rejection acknowledgement separately from refusing an invalid refusal.
 * @aiUsage Checkout retains either explicit outcome; it cannot directly assign Order.status.
 */
export const RejectResult = z.discriminatedUnion('kind', [z.object({ kind: z.literal('recorded') }), z.object({ kind: z.literal('rejected'), reason: z.string() })]);
export type Intent = z.infer<typeof ReservationIntent>;
export type Receipt = z.infer<typeof ReservationReceipt>;
export type Refusal = z.infer<typeof ReservationRefusal>;
export type OrderSnapshot = z.infer<typeof OrderState>;
export type StockSnapshot = z.infer<typeof StockState>;
export type CheckoutMode = 'success' | 'refusal' | 'lost-response' | 'wrong-receipt' | 'stale-receipt' | 'quantity-mismatch';
let calls = 0;
const copy = <T>(value: T): T => structuredClone(value);

/** @aiPurpose Own one order identity, lifecycle and matching-reservation rules.
 * @aiUsage Construct explicitly in memory; only public commands can change its private state.
 */
export class Order {
  #state: OrderSnapshot;
  constructor(line: z.infer<typeof OrderRequest>) { this.#state = OrderState.parse({ ...line, status: 'draft', intentKey: null, receipt: null }); }
  snapshot(): OrderSnapshot { return copy(this.#state); }
  beginReservation(input: z.infer<typeof BeginInput>): z.infer<typeof BeginResult> {
    calls++;
    const { intentKey } = BeginInput.parse(input);
    if (this.#state.status !== 'draft') return BeginResult.parse({ kind: 'rejected', reason: 'Order is not draft.' });
    const intent = ReservationIntent.parse({ orderId: this.#state.orderId, sku: this.#state.sku, quantity: this.#state.quantity, intentKey });
    this.#state = OrderState.parse({ ...this.#state, status: 'awaiting', intentKey });
    return BeginResult.parse({ kind: 'ready', intent });
  }
  confirm(input: Receipt): z.infer<typeof ConfirmResult> {
    calls++;
    const receipt = ReservationReceipt.parse(input);
    if (this.#state.status !== 'awaiting' || !this.matches(receipt)) return ConfirmResult.parse({ kind: 'rejected', reason: 'Receipt does not match the awaiting order, item, quantity and intent.' });
    this.#state = OrderState.parse({ ...this.#state, status: 'confirmed', receipt });
    return ConfirmResult.parse({ kind: 'confirmed', receipt });
  }
  reject(input: Refusal): z.infer<typeof RejectResult> {
    calls++;
    const refusal = ReservationRefusal.parse(input);
    if (this.#state.status !== 'awaiting' || !this.matches(refusal)) return RejectResult.parse({ kind: 'rejected', reason: 'Refusal does not match the awaiting reservation intent.' });
    this.#state = OrderState.parse({ ...this.#state, status: 'rejected' });
    return RejectResult.parse({ kind: 'recorded' });
  }
  private matches(value: Intent): boolean { return value.orderId === this.#state.orderId && value.sku === this.#state.sku && value.quantity === this.#state.quantity && value.intentKey === this.#state.intentKey; }
}
/** @aiPurpose Own inventory availability and reservation commits within one SKU boundary.
 * @aiUsage Explicit reserve validates native input/receipt and never oversells; no recovery/idempotent retry protocol is promised.
 */
export class StockPosition {
  #state: StockSnapshot;
  constructor(sku: string, onHand: number) { this.#state = StockState.parse({ sku, onHand, reservations: [] }); }
  get available(): number { return this.#state.onHand - this.#state.reservations.reduce((sum, receipt) => sum + receipt.quantity, 0); }
  snapshot(): StockSnapshot { return copy(this.#state); }
  reserve(input: Intent): z.infer<typeof ReserveResult> {
    calls++;
    const intent = ReservationIntent.parse(input);
    const reason = intent.sku !== this.#state.sku ? 'Wrong stock position.' : this.#state.reservations.some(receipt => receipt.intentKey === intent.intentKey) ? 'Intent already used; explicit recovery is required.' : this.available < intent.quantity ? 'Insufficient inventory.' : undefined;
    if (reason) return ReserveResult.parse({ kind: 'rejected', refusal: { ...intent, reason } });
    const receipt = ReservationReceipt.parse({ ...intent, reservationId: `reservation:${intent.intentKey}` });
    // The authoritative commit precedes delivery; a later dropped response never rolls it back.
    this.#state = StockState.parse({ ...this.#state, reservations: [...this.#state.reservations, receipt] });
    return ReserveResult.parse({ kind: 'reserved', receipt });
  }
}
export interface NativeCallWitness {
  readonly stepId: 'begin' | 'reserve' | 'confirm' | 'reject'; readonly input: unknown;
  readonly completion: 'reply' | 'local-outcome'; readonly outcomeId: string; readonly payload?: unknown;
  readonly ownerOutcomeId?: string; readonly before: OrderSnapshot | StockSnapshot; readonly after: OrderSnapshot | StockSnapshot;
}
export interface NativeCheckoutRun {
  readonly mode: CheckoutMode; readonly order: OrderSnapshot; readonly otherOrder: OrderSnapshot; readonly stock: StockSnapshot;
  readonly available: number; readonly outcome: 'completed' | 'rejected' | 'unknown'; readonly calls: readonly NativeCallWitness[];
  readonly recordedAt: string;
}
/** @aiPurpose Run a real isolated checkout only on explicit request and retain bounded input/result/state witnesses.
 * @aiUsage lost-response calls reserve and commits first, then drops delivery; receipt-corruption modes exercise Order refusal.
 */
export function runCheckout(mode: CheckoutMode = 'success'): NativeCheckoutRun {
  const quantity = mode === 'refusal' ? 21 : 2;
  const order = new Order({ orderId: 'order-42', sku: 'book', quantity });
  const other = new Order({ orderId: 'order-43', sku: 'book', quantity: 1 });
  const stock = new StockPosition('book', 20);
  const witnesses: NativeCallWitness[] = [];
  let before = order.snapshot();
  const beginInput = { intentKey: 'intent-42' };
  const begun = order.beginReservation(beginInput);
  if (begun.kind !== 'ready') throw new Error('Fresh isolated order must begin.');
  witnesses.push({ stepId: 'begin', input: beginInput, completion: 'reply', outcomeId: 'ready', payload: begun.intent, before, after: order.snapshot(), ownerOutcomeId: 'ready' });
  const stockBefore = stock.snapshot();
  const reserved = stock.reserve(begun.intent);
  let outcome: NativeCheckoutRun['outcome'];
  if (mode === 'lost-response') {
    if (reserved.kind !== 'reserved' || stock.available !== 18) throw new Error('Lost-response fixture must commit before dropping delivery.');
    witnesses.push({ stepId: 'reserve', input: begun.intent, completion: 'local-outcome', outcomeId: 'unknown', ownerOutcomeId: 'reserved', before: stockBefore, after: stock.snapshot() });
    outcome = 'unknown';
  } else if (reserved.kind === 'rejected') {
    witnesses.push({ stepId: 'reserve', input: begun.intent, completion: 'reply', outcomeId: 'refused', payload: reserved.refusal, ownerOutcomeId: 'refused', before: stockBefore, after: stock.snapshot() });
    before = order.snapshot();
    const rejected = order.reject(reserved.refusal);
    witnesses.push({ stepId: 'reject', input: reserved.refusal, completion: 'reply', outcomeId: rejected.kind === 'recorded' ? 'recorded' : 'invalid-refusal', ...(rejected.kind === 'rejected' ? { payload: { reason: rejected.reason } } : {}), ownerOutcomeId: rejected.kind === 'recorded' ? 'recorded' : 'invalid-refusal', before, after: order.snapshot() });
    outcome = 'rejected';
  } else {
    // Explicit transport fault injection; it is not a claim that StockPosition generated an invalid receipt.
    const receipt: Receipt = { ...reserved.receipt, ...(mode === 'wrong-receipt' ? { orderId: 'order-43' } : {}), ...(mode === 'stale-receipt' ? { intentKey: 'old-intent-42' } : {}), ...(mode === 'quantity-mismatch' ? { quantity: reserved.receipt.quantity + 1 } : {}) };
    witnesses.push({ stepId: 'reserve', input: begun.intent, completion: 'reply', outcomeId: 'reserved', payload: receipt, ownerOutcomeId: 'reserved', before: stockBefore, after: stock.snapshot() });
    before = order.snapshot();
    const confirmed = order.confirm(receipt);
    const outcomeId = confirmed.kind === 'confirmed' ? 'confirmed' : 'invalid-receipt';
    witnesses.push({ stepId: 'confirm', input: receipt, completion: 'reply', outcomeId, payload: confirmed.kind === 'confirmed' ? confirmed.receipt : { reason: confirmed.reason }, ownerOutcomeId: outcomeId, before, after: order.snapshot() });
    outcome = confirmed.kind === 'confirmed' ? 'completed' : 'rejected';
  }
  return { mode, order: order.snapshot(), otherOrder: other.snapshot(), stock: stock.snapshot(), available: stock.available, outcome, calls: witnesses, recordedAt: new Date().toISOString() };
}
/** @aiPurpose Expose explicit native command invocation counts for zero-call inspection checks.
 * @aiUsage Read before/after import/inspect/export; only real owner commands increment it.
 */
export function nativeCallCount(): number { return calls; }
