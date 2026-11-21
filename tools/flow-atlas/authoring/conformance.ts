/** @aiPurpose Record explicitly invoked boundaries and check sequential path conformance without scheduling. */
import type {
  Diagnostic, ObservedStep, ProcessHandle, ProcessObservation, StepBinding, StepHandle, TraceReport,
} from './contracts.ts';
import { nativeOperation } from './handles.ts';

/** @aiPurpose Check whether an explicitly observed sequential trace follows exact authored operation/outcome edges.
 * @aiUsage Pass recorded observations and request complete:true when the trace must reach a terminal.
 * @aiBehavior Uninstrumented calls, hidden side effects, concurrency and business correctness are outside coverage.
 */
export function checkTrace(process: ProcessHandle, trace: readonly ObservedStep[], options: { readonly complete?: boolean } = {}): TraceReport {
  const diagnostics: Diagnostic[] = [];
  const report = (code: string, index: number, message: string) => diagnostics.push({ severity: 'error', code, path: ['trace', index], message });
  let cursor: StepHandle | undefined = process.start;
  const startEdges = process.flows.filter(edge => edge.from === process.start && edge.outcomeId === undefined);
  if (process.start.kind !== 'start' || !process.steps.includes(process.start) || startEdges.length !== 1) {
    report('TRACE_INVALID_PROCESS', 0, 'A unique declared start edge is required.');
    cursor = undefined;
  } else cursor = startEdges[0]!.to;
  for (let i = 0; i < trace.length; i++) {
    const item = trace[i]!;
    if (item.sequence !== i + 1) report('TRACE_SEQUENCE', i, 'Observations must have contiguous sequence numbers beginning at one.');
    if (!cursor || cursor.kind !== 'operation' || cursor.id !== item.stepId) {
      report('TRACE_ORDER', i, `Expected ${cursor?.id ?? 'a resolvable next step'}, observed ${item.stepId}.`);
      cursor = undefined;
      continue;
    }
    if (!process.steps.includes(cursor)) report('TRACE_INVALID_PROCESS', i, 'The path enters an undeclared step handle.');
    if (cursor.operation.id !== item.operationId) report('TRACE_OPERATION', i, 'Observed operation does not match this process step.');
    const outcome = cursor.operation.outcomes.find(candidate => candidate.id === item.outcomeId);
    if (!outcome) report('TRACE_OUTCOME', i, 'No declared outcome resolves this observation; absence is never success.');
    else {
      const meaning = outcome.meaning;
      const expectedStatus = meaning === 'failed' || meaning === 'unknown' || meaning === 'no-response' ? meaning : 'returned';
      if (item.status !== expectedStatus) report('TRACE_STATUS', i, `Outcome ${outcome.id} requires ${expectedStatus}, observed ${item.status}.`);
      if ((item.status === 'unknown' || item.status === 'no-response') && !item.reason?.trim()) report('TRACE_REASON', i, 'Uncertainty requires an explicit reason.');
    }
    const edges = process.flows.filter(edge => edge.from === cursor && edge.outcomeId === item.outcomeId);
    if (!outcome || edges.length !== 1) {
      report('TRACE_BRANCH', i, 'The observed outcome does not select exactly one authored edge.');
      cursor = undefined;
    } else cursor = edges[0]!.to;
  }
  const terminal = cursor?.kind === 'end' && process.steps.includes(cursor) ? cursor : undefined;
  const last = trace[trace.length - 1];
  if (terminal?.outcome === 'completed' && last && ['failed', 'unknown', 'no-response'].includes(last.status)) report('TRACE_TERMINAL_MEANING', trace.length, 'Failed or uncertain observation cannot immediately establish successful completion.');
  if (terminal?.outcome === 'completed' && last) {
    const lastStep = process.steps.find(item => item.id === last.stepId);
    if (lastStep?.kind === 'operation' && lastStep.operation.outcomes.find(item => item.id === last.outcomeId)?.meaning === 'rejected') report('TRACE_TERMINAL_MEANING', trace.length, 'A rejected result cannot immediately establish successful completion.');
  }
  if (options.complete && !terminal) report('TRACE_INCOMPLETE', trace.length, 'The observed prefix has not reached a declared terminal.');
  return {
    processId: process.id, complete: Boolean(terminal) && !diagnostics.length,
    ...(terminal ? { terminalStepId: terminal.id } : {}), diagnostics,
  };
}

/** @aiPurpose Create an explicit instrumentation session without running the process or any business method.
 * @aiUsage The consumer chooses and awaits invoke calls; record marks external unknown/no-response observations.
 * @aiBehavior One invocation at a time; native errors are recorded then rethrown, with no retry or invented success.
 */
export function observeProcess(process: ProcessHandle, bindings: readonly StepBinding[]): ProcessObservation {
  const registered = new Set(bindings);
  const boundSteps = new Set<StepHandle>();
  for (const binding of bindings) {
    if (!process.steps.includes(binding.step) || binding.step.kind !== 'operation' || binding.step.operation !== binding.operation || !nativeOperation(binding.operation)) throw new Error('INVALID_STEP_BINDING: exact declared step and native operation handles are required.');
    if (boundSteps.has(binding.step)) throw new Error('DUPLICATE_BINDING: a step has more than one implementation binding.');
    boundSteps.add(binding.step);
  }
  for (const target of process.steps) if (target.kind === 'operation' && !boundSteps.has(target)) throw new Error(`UNBOUND_STEP: ${target.id}`);
  const observations: ObservedStep[] = [];
  let active = false;
  function requireBinding(binding: StepBinding): void {
    if (!registered.has(binding)) throw new Error('FOREIGN_BINDING: reuse a registered binding object.');
    if (active) throw new Error('CONCURRENT_OBSERVATION: await the active invocation before recording another step.');
  }
  function append(binding: StepBinding, status: ObservedStep['status'], outcomeId?: string, reason?: string): void {
    observations.push(Object.freeze({
      sequence: observations.length + 1, stepId: binding.step.id, operationId: binding.operation.id, status,
      ...(outcomeId === undefined ? {} : { outcomeId }), ...(reason === undefined ? {} : { reason }),
    }));
  }
  return {
    async invoke<I, O>(binding: StepBinding<I, O>, input: NoInfer<I>): Promise<O> {
      requireBinding(binding);
      active = true;
      const native = nativeOperation(binding.operation)!;
      try {
        const parsedInput = binding.operation.input.native.parse(input);
        const rawOutput = await native.method(parsedInput);
        const output = binding.operation.output.native.parse(rawOutput);
        const outcomeId = native.classify(output);
        const meaning = binding.operation.outcomes.find(outcome => outcome.id === outcomeId)?.meaning;
        const status = meaning === 'failed' || meaning === 'unknown' || meaning === 'no-response' ? meaning : 'returned';
        append(binding, status, outcomeId, status === 'unknown' || status === 'no-response' ? `Native result classified as ${status}.` : undefined);
        return output;
      } catch (error) {
        append(binding, 'failed', binding.operation.failureOutcomeId, error instanceof Error ? error.message : String(error));
        throw error;
      } finally { active = false; }
    },
    record(binding, status, reason) {
      requireBinding(binding);
      if (!reason.trim()) throw new Error('TRACE_REASON: uncertainty requires a reason.');
      if (status !== 'unknown' && status !== 'no-response') throw new Error('TRACE_STATUS: record supports only unknown/no-response.');
      const outcomeId = status === 'unknown' ? binding.operation.unknownOutcomeId : binding.operation.noResponseOutcomeId;
      append(binding, status, outcomeId, reason);
    },
    trace() { return Object.freeze(observations.map(item => Object.freeze({ ...item }))); },
    check(options) {
      const result = checkTrace(process, observations, options);
      if (!active) return result;
      return { ...result, complete: false, diagnostics: [...result.diagnostics, { severity: 'error', code: 'TRACE_PENDING', path: ['trace', observations.length], message: 'An invoked method has not settled; no response is inferred.' }] };
    },
  };
}
