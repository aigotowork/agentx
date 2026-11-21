/** @aiPurpose Supply portable authoring mistakes as reusable case inputs, separately from independent acceptance tests. */
import type { DomainModel, DomainScenario, Json } from '../contracts.ts';
import { createOrderModel } from './order-stock.ts';
export interface SemanticCase { readonly id: string; readonly expectedCodes: readonly string[]; readonly model: DomainModel; }
function alterScenario(model: DomainModel, scenarioId: string, change: (scenario: DomainScenario) => DomainScenario): DomainModel {
  return { ...model, scenarios: model.scenarios.map(scenario => scenario.id === scenarioId ? change(scenario) : scenario) };
}
/** @aiPurpose Create decisive negative source/witness inputs without running tests or native handlers.
 * @aiUsage Inspect each model and locate the expected diagnostic groups; independent acceptance owns executed expectations.
 */
export function createNegativeModels(): readonly SemanticCase[] {
  const receiptMismatch = (field: string, value: Json): DomainModel => alterScenario(createOrderModel(), 'scenario.success', scenario => ({ ...scenario, visits: scenario.visits.map(visit => visit.stepId === 'confirm' ? { ...visit, input: { ...(visit.input as Readonly<Record<string, Json>>), [field]: value } } : visit) }));
  const unknown = alterScenario(createOrderModel(), 'scenario.lost-response', scenario => ({ ...scenario, terminalStepId: 'completed' }));
  const branch = createOrderModel();
  return [
    { id: 'wrong-quantity-as-success', expectedCodes: ['PARAMETER_MISMATCH', 'RULE_VIOLATION'], model: receiptMismatch('quantity', 3) },
    { id: 'wrong-order-receipt-as-success', expectedCodes: ['PARAMETER_MISMATCH', 'RULE_VIOLATION'], model: receiptMismatch('orderId', 'order-43') },
    { id: 'unknown-as-success', expectedCodes: ['UNKNOWN_AS_SUCCESS'], model: unknown },
    { id: 'wrong-result-branch', expectedCodes: ['WRONG_BRANCH', 'UNAVAILABLE_RESULT'], model: { ...branch, definition: { ...branch.definition, processes: branch.definition.processes.map(process => ({ ...process, steps: process.steps.map(step => step.kind === 'operation' && step.id === 'confirm' ? { ...step, bindings: step.bindings.map(binding => ({ ...binding, source: { kind: 'result' as const, stepId: 'reserve', outcomeId: 'refused', path: [] } })) } : step) })) } } },
  ];
}
