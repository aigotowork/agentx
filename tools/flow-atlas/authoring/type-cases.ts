/** @aiPurpose Keep schema-derived handler and invocation types honest through compile-only negative cases.
 * These functions are never invoked; controller owns the compiler execution.
 */
import { defineOperation, observeProcess } from './index.ts';
import { createOrderExample } from './examples/order-inventory.ts';

function rejectedHandlerReturn() {
  const example = createOrderExample();
  defineOperation({
    ...example.reserve,
    // @ts-expect-error The imported output schema cannot be widened to admit a number.
    method: () => 123,
    classify: result => result.kind,
  });
}

function rejectedHandlerInput() {
  const example = createOrderExample();
  defineOperation({
    ...example.reserve,
    // @ts-expect-error The native method must accept the imported request schema's output.
    method: (input: { sku: number }) => ({ kind: 'rejected' as const, reason: String(input.sku) }),
    classify: result => result.kind,
  });
}

function rejectedInvocation() {
  const example = createOrderExample();
  const observed = observeProcess(example.process, example.model.bindings);
  // @ts-expect-error The binding's native schema requires quantity.
  void observed.invoke(example.reserveBinding, { orderId: '42', sku: 'book' });
  // @ts-expect-error The binding's native schema requires numeric quantity.
  void observed.invoke(example.reserveBinding, { orderId: '42', sku: 'book', quantity: '1' });
}

void [rejectedHandlerReturn, rejectedHandlerInput, rejectedInvocation];
