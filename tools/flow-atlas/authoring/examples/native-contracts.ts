/** @aiPurpose Own the example application's real schemas and methods independently of its visualization.
 * Importing this contract-only module creates no requests, services or handler invocations.
 */
import * as z from 'zod';

/** @aiPurpose Define the application's native order request once.
 * @aiUsage Both business code and authoring import this exact runtime schema.
 */
export const OrderRequest = z.object({ orderId: z.string().min(1), sku: z.string().min(1), quantity: z.number().int().min(1) });
/** @aiPurpose Reuse the native request as the inventory boundary rather than duplicate its fields.
 * @aiUsage This example's prepared reservation request has exactly the order request shape.
 */
export const ReserveRequest = OrderRequest;
/** @aiPurpose Define the native inventory result discriminant and payloads.
 * @aiUsage Infer business return types and inspector JSON Schema from this same source.
 */
export const ReserveResult = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('reserved'), reservationId: z.string(), remaining: z.number().int().min(0) }),
  z.object({ kind: z.literal('rejected'), reason: z.string() }),
  z.object({ kind: z.literal('unknown'), reason: z.string() }),
]);
export type OrderInput = z.infer<typeof OrderRequest>;
export type ReservationResult = z.infer<typeof ReserveResult>;

const stock = new Map([['book', 20]]);
let calls = 0;

/** @aiPurpose Prepare an order using its native runtime contract.
 * @aiUsage This real method is passed unchanged to defineOperation; calls are observable for side-effect checks.
 */
export function prepareOrder(input: OrderInput): z.infer<typeof ReserveRequest> {
  calls++;
  return ReserveRequest.parse(input);
}

/** @aiPurpose Reserve native in-memory example inventory only when explicitly invoked.
 * @aiUsage book demonstrates a real decrement, missing rejects, lost reports uncertainty and throw raises a transport error.
 */
export async function reserveInventory(input: z.infer<typeof ReserveRequest>): Promise<ReservationResult> {
  calls++;
  if (input.sku === 'throw') throw new Error('Inventory transport failed.');
  if (input.sku === 'lost') return { kind: 'unknown', reason: 'Connection lost; the reservation result is unknown.' };
  const available = stock.get(input.sku) ?? 0;
  if (available < input.quantity) return { kind: 'rejected', reason: 'Insufficient inventory.' };
  const remaining = available - input.quantity;
  stock.set(input.sku, remaining);
  return { kind: 'reserved', reservationId: `reservation:${input.orderId}`, remaining };
}

/** @aiPurpose Expose a counter to verify inspection never invokes business methods.
 * @aiUsage Read before and after inspect/compile; observation calls intentionally increment it.
 */
export function nativeCallCount(): number { return calls; }
