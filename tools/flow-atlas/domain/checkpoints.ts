/** @aiPurpose Own one deterministic causal scenario projection and detached access shared by all workbench views. */
import type { AssertionResult, CallProjection, Checkpoint, CompiledScenario, Diagnostic, DomainScenario, FieldPath, Id, Json, ParameterProvenance, RuleOperand, ScenarioVisit, StateDifference, WorkbenchArtifact, WorkbenchDefinition } from './contracts.ts';
import { portableCopy } from '../authoring/portable.ts';
import { pathKey, prefix, read, same, valueErrors } from './schema.ts';

/** @aiPurpose Compile inert authored/observed witnesses into the only authoritative checkpoint sequence.
 * @aiUsage Internal compiler seam; effects and caller knowledge have separate checkpoints and evidence.
 */
export function compileScenario(definition: WorkbenchDefinition, input: DomainScenario, scenarioIndex: number): CompiledScenario {
  const diagnostics: Diagnostic[] = [];
  const at: FieldPath = ['scenarios', scenarioIndex];
  const report = (code: string, path: FieldPath, message: string): void => { diagnostics.push({ severity: 'error', code, path: [...at, ...path], message }); };
  let scenario: DomainScenario;
  try { scenario = portableCopy(input) as unknown as DomainScenario; }
  catch (error) {
    report('NONPORTABLE_SCENARIO', [], String(error));
    scenario = { id: input.id, label: input.label, processId: '', input: null, instances: [], roles: {}, visits: [], terminalStepId: '', expectations: [] };
  }
  const process = definition.processes.find(item => item.id === scenario.processId);
  const objects = new Map(definition.objects.map(item => [item.id, item]));
  const operations = new Map(definition.operations.map(item => [item.id, item]));
  const schemas = new Map(definition.schemas.map(item => [item.id, item]));
  const instances = new Map(scenario.instances.map(item => [item.id, item]));
  const states: Record<string, Json> = Object.create(null);
  const calls: CallProjection[] = [];
  const checkpoints: Checkpoint[] = [];
  const assertions: AssertionResult[] = [];
  const visits = new Map<string, ScenarioVisit>();
  const seenCallIds = new Set<string>();
  const seenStepIds = new Set<string>();
  const knownIds = new Set<string>();
  const validIdentity = (id: string): boolean => /^[A-Za-z0-9][A-Za-z0-9_.:-]*$/.test(id) && !['__proto__', 'constructor', 'prototype'].includes(id);
  const validate = (value: unknown, schemaId: string, path: FieldPath, code = 'INCOMPATIBLE_VALUE'): void => { for (const error of valueErrors(value, schemas.get(schemaId))) report(code, path, error); };
  for (const [i, instance] of scenario.instances.entries()) {
    if (!validIdentity(instance.id) || knownIds.has(instance.id)) report('INVALID_INSTANCE_ID', ['instances', i, 'id'], 'Instance IDs must be safe and unique within a scenario.');
    knownIds.add(instance.id);
    const object = objects.get(instance.objectId);
    if (!object) report('BROKEN_REF', ['instances', i, 'objectId'], `Unknown object ${instance.objectId}.`);
    else {
      validate(instance.initial, object.stateSchemaId, ['instances', i, 'initial'], 'INVALID_INITIAL_STATE');
      if (object.businessIdentity?.length) {
        const values = object.businessIdentity.map(path => read(instance.initial, path));
        const projected: Record<string | number, Json> = Object.create(null);
        object.businessIdentity.forEach((path, index) => {
          let cursor = projected;
          path.forEach((key, depth) => { if (depth === path.length - 1) cursor[key] = values[index] as Json; else { cursor[key] ??= Object.create(null); cursor = cursor[key] as Record<string | number, Json>; } });
        });
        const matchesScalar = values.length === 1 && same(values[0], instance.businessIdentity);
        if (values.some(value => value === undefined) || !matchesScalar && !same(projected, instance.businessIdentity)) report('BUSINESS_IDENTITY_MISMATCH', ['instances', i, 'businessIdentity'], 'Business identity must equal the native identity projection (or its scalar value for one field), independently from instance/call IDs.');
        for (const previous of scenario.instances.slice(0, i)) if (previous.objectId === instance.objectId && same(object.businessIdentity.map(path => read(previous.initial, path)), values)) report('DUPLICATE_BUSINESS_IDENTITY', ['instances', i], 'Two instances of this entity cannot duplicate one business identity in the same scenario.');
      }
      if (object.consistencyRootId && object.consistencyRootId !== object.id) {
        const root = instances.get(instance.consistencyRootInstanceId ?? '');
        if (!root || root.objectId !== object.consistencyRootId) report('MEMBERSHIP_OWNERSHIP', ['instances', i, 'consistencyRootInstanceId'], 'The instance needs a root instance matching its declared consistency boundary.');
      } else if (instance.consistencyRootInstanceId && instances.get(instance.consistencyRootInstanceId)?.objectId !== (object.consistencyRootId ?? object.id)) report('MEMBERSHIP_OWNERSHIP', ['instances', i], 'Root instance does not match this object’s consistency boundary.');
    }
    states[instance.id] = portableCopy(instance.initial);
  }
  if (!process) report('BROKEN_REF', ['processId'], `Unknown process ${scenario.processId}.`);
  if (process) {
    validate(scenario.input, process.inputSchemaId, ['input'], 'INVALID_PROCESS_INPUT');
    for (const role of process.roles) if (instances.get(scenario.roles[role.id])?.objectId !== role.objectId) report('ROLE_INSTANCE_MISMATCH', ['roles', role.id], 'Bind this role to a distinct scenario instance of the declared object type.');
    for (const roleId of Object.keys(scenario.roles)) if (!process.roles.some(role => role.id === roleId)) report('BROKEN_REF', ['roles', roleId], 'Scenario binds an unknown role.');
  }
  function emit(phase: Checkpoint['phase'], visit?: ScenarioVisit, parameters: readonly ParameterProvenance[] = [], differences: readonly StateDifference[] = [], checks: readonly AssertionResult[] = []): void {
    const index = checkpoints.length;
    const meta = visit ? { stepId: visit.stepId, visitId: visit.id, callId: visit.callId, ...(phase === 'reply' || phase === 'outcome' ? { outcomeId: visit.outcomeId } : {}) } : {};
    const derived: Record<string, Record<string, Json>> = Object.create(null);
    for (const instance of scenario.instances) {
      for (const field of objects.get(instance.objectId)?.fields ?? []) {
        const projection = field.derived?.projection;
        if (!projection) continue;
        const total = read(states[instance.id], projection.totalPath); const items = read(states[instance.id], projection.itemsPath);
        if (projection.kind !== 'subtract-sum' || typeof total !== 'number' || !Array.isArray(items) || items.some(item => typeof read(item, projection.itemPath) !== 'number')) { report('UNSUPPORTED_DERIVATION', ['instances', instance.id, 'fields', field.id], 'Derived projection needs a numeric total and numeric collection members.'); continue; }
        derived[instance.id] ??= Object.create(null);
        derived[instance.id][field.id] = total - items.reduce((sum: number, item: unknown) => sum + (read(item, projection.itemPath) as number), 0);
      }
    }
    checkpoints.push(portableCopy({
      id: `${scenario.id}:checkpoint:${index}`, label: phase === 'initial' ? 'Initial state' : phase === 'terminal' ? 'Expected outcome' : `${visit?.label ?? ''} · ${phase}`,
      index, phase, ...meta, states, derived, calls, parameters, differences, assertions: checks, diagnostics: [...diagnostics],
    }) as unknown as Checkpoint);
  }
  function operand(operand: RuleOperand, state: Json, visit?: ScenarioVisit): unknown {
    if (operand.kind === 'literal') return operand.value;
    if (operand.kind === 'state') return read(state, operand.path);
    if (operand.kind === 'input') return visit ? read(visit.input, operand.path) : undefined;
    return visit && visit.completion === 'reply' ? read(visit.payload, operand.path) : undefined;
  }
  function checkRules(instanceId: string, visit?: ScenarioVisit, ownerOutcomeId?: string): AssertionResult[] {
    const instance = instances.get(instanceId);
    const step = process?.steps.find(step => step.id === visit?.stepId);
    const operation = step?.kind === 'operation' ? operations.get(step.operationId) : undefined;
    const basis = visit?.effect?.evidence.kind ?? visit?.evidence.kind ?? 'authored';
    const results: AssertionResult[] = [];
    for (const rule of definition.rules.filter(rule => rule.ownerId === instance?.objectId)) {
      const isOperationRule = rule.check && [rule.check.left, rule.check.right].some(item => item.kind === 'input' || item.kind === 'result');
      if (isOperationRule && (!visit || !operation?.ruleIds.includes(rule.id))) continue;
      if (rule.outcomeIds && (!ownerOutcomeId || !rule.outcomeIds.includes(ownerOutcomeId))) continue;
      let status: AssertionResult['status'] = 'declared'; let message = 'Declared/source-linked rule; no business predicate executed.';
      if (rule.check) {
        const left = operand(rule.check.left, states[instanceId], visit); const right = operand(rule.check.right, states[instanceId], visit);
        if (left === undefined || right === undefined) { status = 'unsupported'; message = 'This witness does not provide both operands; no rule proof is claimed.'; }
        else {
          const operator = rule.check.operator;
          const numeric = operator === 'gte' || operator === 'lte';
          if (numeric && (typeof left !== 'number' || typeof right !== 'number')) { status = 'unsupported'; message = 'Ordered comparisons require two numeric values.'; }
          else {
            const passed = operator === 'eq' ? same(left, right) : operator === 'ne' ? !same(left, right) : operator === 'gte' ? (left as number) >= (right as number) : operator === 'lte' ? (left as number) <= (right as number) : false;
            status = passed ? 'passed' : 'failed'; message = passed ? 'Bounded comparison holds for this witness.' : 'Witness violates the declared comparison.';
            if (!passed) report('RULE_VIOLATION', ['visits', visit?.id ?? 'initial', 'rules', rule.id], message);
          }
        }
      }
      results.push({ id: `${visit?.id ?? 'initial'}:${instanceId}:${rule.id}`, label: rule.label, subjectId: rule.id, status, basis: status === 'declared' ? 'declared' : basis, message });
    }
    assertions.push(...results);
    return results;
  }
  const initialChecks = scenario.instances.flatMap(instance => checkRules(instance.id));
  emit('initial', undefined, [], [], initialChecks);
  let priorStepId = process?.startId ?? ''; let priorOutcomeId: string | undefined;
  for (const [visitIndex, visit] of scenario.visits.entries()) {
    const here: FieldPath = ['visits', visitIndex];
    if (!validIdentity(visit.id) || visits.has(visit.id)) report('DUPLICATE_VISIT', [...here, 'id'], 'Visits need safe unique identities.');
    if (!validIdentity(visit.callId) || seenCallIds.has(visit.callId)) report('DUPLICATE_CALL', [...here, 'callId'], 'Call identities are unique and separate from instance/business identity.');
    seenCallIds.add(visit.callId);
    if (seenStepIds.has(visit.stepId)) report('UNSUPPORTED_CYCLE', [...here, 'stepId'], 'Repeated step visits/loop dataflow are unsupported.');
    seenStepIds.add(visit.stepId);
    const step = process?.steps.find(step => step.id === visit.stepId);
    if (step?.kind !== 'operation') { report('BROKEN_REF', [...here, 'stepId'], 'A visit must reference an operation step.'); continue; }
    const operation = operations.get(step.operationId);
    if (!operation) { report('BROKEN_REF', here, 'Visited operation does not exist.'); continue; }
    const incoming = process!.flows.filter(flow => flow.from === priorStepId && flow.to === step.id && flow.outcomeId === priorOutcomeId);
    if (incoming.length !== 1 || visit.viaFlowId && incoming[0]?.id !== visit.viaFlowId) report('INVALID_PROCESS_PATH', [...here, 'viaFlowId'], 'Visit is not the unique sequential successor of the prior observed branch.');
    const instanceId = scenario.roles[step.roleId]; const callerId = scenario.roles[step.callerRoleId];
    const instance = instances.get(instanceId); const object = objects.get(instance?.objectId ?? '');
    if (instance?.objectId !== operation.ownerId) report('METHOD_OWNERSHIP', here, 'The call target is not an instance of the operation owner.');
    const outcome = operation.outcomes.find(outcome => outcome.id === visit.outcomeId);
    if (!outcome) report('BROKEN_REF', [...here, 'outcomeId'], 'Visit outcome is not declared by its operation.');
    if (!['reply', 'local-outcome'].includes(visit.completion)) report('INVALID_COMPLETION', [...here, 'completion'], 'Declare received reply or local uncertainty explicitly.');
    if (visit.completion === 'local-outcome') {
      if (!outcome || !['unknown', 'no-response', 'failed'].includes(outcome.meaning)) report('UNKNOWN_AS_SUCCESS', here, 'A local transport outcome cannot establish successful caller knowledge.');
      if (Object.hasOwn(visit, 'payload')) report('UNAVAILABLE_RESULT', [...here, 'payload'], 'Local outcomes cannot fabricate a returned payload.');
    }
    validate(visit.input, operation.inputSchemaId, [...here, 'input'], 'INVALID_OPERATION_INPUT');
    if (visit.completion === 'reply' && outcome?.payloadSchemaId) {
      if (!Object.hasOwn(visit, 'payload')) report('MISSING_RESULT', [...here, 'payload'], 'This received outcome requires a payload.');
      else validate(visit.payload, outcome.payloadSchemaId, [...here, 'payload'], 'INVALID_RESULT');
    } else if (visit.payload !== undefined && !outcome?.payloadSchemaId) report('UNEXPECTED_RESULT', [...here, 'payload'], 'Outcome has no declared payload contract.');
    const parameters: ParameterProvenance[] = step.bindings.map(binding => {
      const source = binding.source; let value: unknown; let sourceVisit: ScenarioVisit | undefined;
      if (source.kind === 'input') value = read(scenario.input, source.path);
      else if (source.kind === 'literal') value = source.value;
      else if (source.kind === 'identity') { const targetId = scenario.roles[source.roleId]; value = read(instances.get(targetId)?.initial, source.path); }
      else if (source.kind === 'result') {
        sourceVisit = [...visits.values()].find(item => item.stepId === source.stepId);
        if (sourceVisit?.completion === 'reply' && sourceVisit.outcomeId === source.outcomeId) value = read(sourceVisit.payload, source.path);
      }
      const actual = read(visit.input, binding.targetPath);
      const status = value === undefined ? 'unavailable' : same(value, actual) ? 'resolved' : 'mismatch';
      if (status !== 'resolved') report(status === 'unavailable' ? 'UNAVAILABLE_RESULT' : 'PARAMETER_MISMATCH', [...here, 'bindings', binding.id], status === 'unavailable' ? 'Source result/value has not been received on this qualified branch.' : 'Actual input differs from its declared parameter source.');
      return { id: binding.id, label: binding.label, targetPath: binding.targetPath, source, status, ...(value === undefined ? {} : { value: value as Json }), ...(actual === undefined ? {} : { actual: actual as Json }), ...(sourceVisit ? { sourceVisitId: sourceVisit.id, sourceCallId: sourceVisit.callId } : {}) };
    });
    const callIndex = calls.length;
    calls.push({ callId: visit.callId, visitId: visit.id, stepId: step.id, operationId: operation.id, fromInstanceId: callerId ?? '', targetInstanceId: instanceId ?? '', input: visit.input, phase: 'pending', evidence: visit.evidence });
    emit('call', visit, parameters);
    if (visit.effect) {
      const effect = visit.effect;
      const ownerOutcome = operation.outcomes.find(outcome => outcome.id === effect.outcomeId);
      if (!ownerOutcome) report('BROKEN_REF', [...here, 'effect', 'outcomeId'], 'Owner effect refers to an undeclared result.');
      if (visit.completion === 'reply' && outcome && !['unknown', 'no-response', 'failed'].includes(outcome.meaning) && visit.outcomeId !== effect.outcomeId) report('EFFECT_OUTCOME_MISMATCH', [...here, 'effect'], 'A definite received result disagrees with the authoritative owner result.');
      if (operation.kind === 'query' && effect.writes.length) report('QUERY_WRITE', [...here, 'effect'], 'A query cannot mutate domain state.');
      const allowed = operation.effects.filter(item => item.outcomeId === effect.outcomeId).flatMap(item => item.fieldPaths);
      const differences: StateDifference[] = [];
      const beforeState = states[instanceId];
      let nextState = beforeState;
      for (const [writeIndex, write] of effect.writes.entries()) {
        const writePath = [...here, 'effect', 'writes', writeIndex];
        if (write.instanceId !== instanceId || instances.get(write.instanceId)?.objectId !== operation.ownerId) { report('STATE_OWNERSHIP', writePath, 'Only the exact called owner instance may change its state.'); continue; }
        if (operation.kind === 'query') continue;
        if (!allowed.some(path => prefix(path, write.path))) { report('UNDECLARED_EFFECT', writePath, 'Operation/outcome does not own this state effect.'); continue; }
        if (object?.businessIdentity?.some(path => prefix(path, write.path) || prefix(write.path, path))) { report('IDENTITY_WRITE', writePath, 'Business identity cannot be changed by scenario patches.'); continue; }
        if (object?.fields.some(field => field.derived && (prefix(field.path, write.path) || prefix(write.path, field.path)))) { report('DERIVED_STATE_WRITE', writePath, 'Derived state must not have independent patches.'); continue; }
        const before = read(nextState, write.path);
        try {
          const candidate = writeValue(nextState, write.path, write.value);
          if (object?.lifecycle) {
            const from = read(nextState, object.lifecycle.path); const to = read(candidate, object.lifecycle.path);
            if (!same(from, to)) {
              const transition = object.lifecycle.transitions.find(transition => transition.id === write.transitionId && transition.operationId === operation.id && transition.outcomeId === effect.outcomeId && same(transition.from, from) && same(transition.to, to));
              if (!transition) { report('INVALID_TRANSITION', writePath, 'Lifecycle change needs the owner’s matching legal transition witness.'); continue; }
            }
          }
          if (object) {
            const errors = valueErrors(candidate, schemas.get(object.stateSchemaId));
            if (errors.length) { for (const message of errors) report('INVALID_STATE', writePath, message); continue; }
          }
          nextState = candidate;
          if (!same(before, write.value)) differences.push({ instanceId: write.instanceId, path: write.path, ...(before === undefined ? {} : { before: before as Json }), after: write.value, causeVisitId: visit.id, operationId: operation.id, ...(write.transitionId ? { transitionId: write.transitionId } : {}), evidence: effect.evidence });
        } catch (error) { report('INVALID_STATE_WRITE', writePath, String(error)); }
      }
      if (instanceId && nextState !== undefined) states[instanceId] = nextState;
      calls[callIndex] = { ...calls[callIndex], effectOutcomeId: effect.outcomeId };
      const checks = instanceId ? checkRules(instanceId, visit, effect.outcomeId) : [];
      emit('effect', visit, parameters, differences, checks);
    }
    calls[callIndex] = { ...calls[callIndex], phase: outcome?.meaning ?? 'failed', outcomeId: visit.outcomeId, completion: visit.completion, ...(visit.payload !== undefined ? { payload: visit.payload } : {}) };
    const checks = !visit.effect && instanceId ? checkRules(instanceId, visit, visit.outcomeId) : [];
    emit(visit.completion === 'local-outcome' ? 'outcome' : 'reply', visit, parameters, [], checks);
    visits.set(visit.id, visit); priorStepId = step.id; priorOutcomeId = visit.outcomeId;
  }
  const terminal = process?.steps.find(step => step.id === scenario.terminalStepId);
  if (terminal?.kind !== 'end') report('INVALID_TERMINAL', ['terminalStepId'], 'Scenario must name its actual terminal step.');
  if (process && !process.flows.some(flow => flow.from === priorStepId && flow.to === scenario.terminalStepId && flow.outcomeId === priorOutcomeId)) report('INVALID_PROCESS_PATH', ['terminalStepId'], 'Terminal is not reachable from the final observed outcome.');
  if (terminal?.kind === 'end' && terminal.outcome === 'completed' && calls.some(call => call.phase === 'unknown' || call.phase === 'no-response' || call.phase === 'failed')) report('UNKNOWN_AS_SUCCESS', ['terminalStepId'], 'A completed terminal cannot erase unresolved/failed caller knowledge.');
  const expectedIds = new Set<string>();
  const expected = scenario.expectations.map(expectation => {
    if (!validIdentity(expectation.id) || expectedIds.has(expectation.id)) report('DUPLICATE_EXPECTATION', ['expectations', expectation.id], 'Expectations require unique safe IDs.');
    expectedIds.add(expectation.id);
    const actual = expectation.kind === 'state' ? read(states[expectation.instanceId], expectation.path) : expectation.kind === 'call' ? calls.find(call => call.visitId === expectation.visitId)?.outcomeId : terminal?.kind === 'end' ? terminal.outcome : undefined;
    const desired = expectation.kind === 'state' ? expectation.equals : expectation.kind === 'call' ? expectation.outcomeId : expectation.outcome;
    const passed = actual !== undefined && same(actual, desired);
    if (!passed) report('SCENARIO_EXPECTATION', ['expectations', expectation.id], 'Actual witness projection does not match the separately authored expectation.');
    return { id: expectation.id, label: expectation.label, subjectId: expectation.kind === 'state' ? expectation.instanceId : expectation.kind === 'call' ? expectation.visitId : scenario.terminalStepId, status: passed ? 'passed' as const : 'failed' as const, basis: scenario.visits.some(visit => visit.evidence.kind === 'authored' || visit.effect?.evidence.kind === 'authored') || !scenario.visits.length ? 'authored' as const : 'observed' as const, message: passed ? 'Expected value matches this witness.' : 'Expected value differs from this witness.' };
  });
  assertions.push(...expected);
  emit('terminal', undefined, [], [], expected);
  return { ...scenario, checkpoints, diagnostics, assertions };
}
function writeValue(state: Json, path: FieldPath, value: Json): Json {
  if (!path.length) return portableCopy(value);
  const result = portableCopy(state);
  let cursor: unknown = result;
  for (const [i, key] of path.entries()) {
    if (['__proto__', 'prototype', 'constructor'].includes(String(key)) || typeof key === 'number' && (!Number.isInteger(key) || key < 0)) throw new Error('Unsafe field path.');
    if (!cursor || typeof cursor !== 'object' || !Object.hasOwn(cursor, key)) throw new Error('State writes must address existing owned fields.');
    if (i === path.length - 1) (cursor as Record<string | number, Json>)[key] = portableCopy(value);
    else cursor = (cursor as Record<string | number, Json>)[key];
  }
  return result;
}
/** @aiPurpose Select a detached authoritative scenario checkpoint.
 * @aiUsage Index zero is initial; unknown scenario/index throws instead of selecting another scenario.
 */
export function checkpoint(artifact: WorkbenchArtifact, scenarioId: Id, index = 0): Checkpoint {
  const value = Number.isInteger(index) ? artifact.scenarios.find(item => item.id === scenarioId)?.checkpoints[index] : undefined;
  if (!value) throw new RangeError(`Unknown checkpoint ${scenarioId}:${index}.`);
  return portableCopy(value) as unknown as Checkpoint;
}
/** @aiPurpose Return detached instance states at one checkpoint.
 * @aiUsage Copies remain independent from the artifact, earlier and later snapshots.
 */
export function snapshot(artifact: WorkbenchArtifact, scenarioId: Id, index = 0): Readonly<Record<Id, Json>> { return checkpoint(artifact, scenarioId, index).states; }
/** @aiPurpose Return only state effects applied at one checkpoint.
 * @aiUsage Call/reply/outcome checkpoints never synthesize an owner state change.
 */
export function changes(artifact: WorkbenchArtifact, scenarioId: Id, index: number): readonly StateDifference[] { return checkpoint(artifact, scenarioId, index).differences; }
