/** @aiPurpose Convert source-first contracts and check references, ownership and bounded dataflow without business execution. */
import type { Checked, Diagnostic, DomainModel, FieldPath, Identity, InputField, Json, ParameterSource, ProcessDefinition, SchemaHandle, ValueSchema, WorkbenchArtifact, WorkbenchDefinition } from './contracts.ts';
import { portableCopy } from '../authoring/portable.ts';
import { compileScenario } from './checkpoints.ts';
import { nativeMethod } from './binding.ts';
import { compatible, pathKey, prefix, requiredPaths, schemaAt, schemaCoverage, valueErrors } from './schema.ts';
import type { Report } from './schema.ts';

/** @aiPurpose Inspect incomplete source-first designs and retain located diagnostics and scenario projections.
 * @aiUsage Never invokes methods, native parse hooks, predicates or source URLs; only explicit schema adapters convert.
 */
export function inspect(model: DomainModel): WorkbenchArtifact {
  const diagnostics: Diagnostic[] = [];
  const report: Report = (code, path, message, severity = 'error') => diagnostics.push({ severity, code, path, message });
  const identities = (items: readonly Identity[], path: FieldPath): void => {
    const seen = new Set<string>();
    items.forEach((item, i) => {
      if (!/^[A-Za-z0-9][A-Za-z0-9_.:-]*$/.test(item.id) || ['__proto__', 'constructor', 'prototype'].includes(item.id)) report('INVALID_ID', [...path, i, 'id'], 'Use a safe nonempty stable ID.');
      if (seen.has(item.id)) report('DUPLICATE_ID', [...path, i, 'id'], `Duplicate ${item.id}.`);
      seen.add(item.id);
      if (!item.label?.trim()) report('INVALID_LABEL', [...path, i, 'label'], 'A visible label is required.');
      for (const field of ['documents', 'sourceLinks'] as const) for (const [j, link] of (item.metadata?.[field] ?? []).entries()) checkUri(link.uri, [...path, i, 'metadata', field, j, 'uri']);
      for (const key of Object.keys(item.metadata?.extensions ?? {})) if (!/^[A-Za-z0-9_.-]+:[A-Za-z0-9_.:-]+$/.test(key)) report('EXTENSION_NAMESPACE', [...path, i, 'metadata', 'extensions', key], 'Extension keys require an owner namespace.');
    });
  };
  function checkUri(uri: string, path: FieldPath): void {
    if (!uri.trim() || /[\u0000-\u0020\u007f\\]/.test(uri) || uri.startsWith('//') || /^[A-Za-z][A-Za-z0-9+.-]*:/.test(uri) && !/^https?:\/\//i.test(uri)) report('UNSAFE_DOCUMENT_URI', path, 'Use an inert relative reference or HTTP(S) URL.');
  }
  const schemas: ValueSchema[] = [];
  const schemaHandles = new Map<string, SchemaHandle>();
  function convert(handle: SchemaHandle, io: 'input' | 'output', path: FieldPath): string {
    const id = `${handle.id}:${io}`;
    const previous = schemaHandles.get(id);
    if (previous) {
      if (previous.native !== handle.native || previous.adapter !== handle.adapter) report('DUPLICATE_SCHEMA_OWNERSHIP', path, `Schema ${id} has conflicting native owners.`);
      return id;
    }
    schemaHandles.set(id, handle);
    identities([handle], path);
    try {
      const result = handle.adapter.convert(handle.native, { id, label: handle.label, io });
      diagnostics.push(...result.diagnostics.map(item => ({ ...item, path: [...path, ...item.path] })));
      if (result.ok) {
        if (result.value.id !== id) report('SCHEMA_ID_MISMATCH', path, 'Converter must preserve requested schema identity.');
        const jsonSchema = portableCopy(result.value.jsonSchema);
        if (!jsonSchema || typeof jsonSchema !== 'object' || Array.isArray(jsonSchema)) report('NONPORTABLE_SCHEMA', path, 'Expected JSON Schema object.');
        else schemas.push({ id, label: handle.label, jsonSchema: jsonSchema as Readonly<Record<string, Json>> });
      } else if (!result.diagnostics.some(item => item.severity === 'error')) report('SCHEMA_CONVERSION_FAILED', path, 'Schema adapter refused conversion.');
    } catch (error) { report('SCHEMA_CONVERSION_FAILED', path, String(error)); }
    return id;
  }
  const source = model.definition;
  const methodOwners = new Map<unknown, string>();
  const methodIds = new Map<string, unknown>();
  const { values, objects, operations, processes, ...plain } = source;
  let definition: WorkbenchDefinition;
  try {
    // Only portable leaves pass this boundary; no native schema/parse method is copied.
    definition = portableCopy({
      ...plain, schemas,
      values: values.map(({ schema, ...value }, i) => ({ ...value, schemaId: convert(schema, 'output', ['values', i, 'schema']) })),
      objects: objects.map(({ state, ...object }, i) => ({ ...object, stateSchemaId: convert(state, 'output', ['objects', i, 'state']) })),
      operations: operations.map(({ input, outcomes, native, ...operation }, i) => {
        const path: FieldPath = ['definition', 'operations', i, 'native'];
        const method = native && nativeMethod(native);
        const validNative = !!(method && native && native.input === input && native.ownerId === operation.ownerId);
        if (native) {
          identities([native], path);
          if (!method) report('FOREIGN_NATIVE_METHOD', path, 'Use the exact handle returned by bindNativeMethod with an actual native method value.');
          if (native.input !== input) report('NATIVE_INPUT_MISMATCH', path, 'The operation must reuse its native method input SchemaHandle.');
          if (native.ownerId !== operation.ownerId) report('NATIVE_METHOD_OWNERSHIP', path, 'Native method handle belongs to another owner.');
          if (method) {
            if (methodOwners.has(method)) report('DUPLICATE_METHOD_OWNERSHIP', path, `Native method is already owned by ${methodOwners.get(method)}.`);
            methodOwners.set(method, operation.id);
            const previous = methodIds.get(native.id);
            if (previous && previous !== native) report('DUPLICATE_NATIVE_METHOD_ID', path, 'A native method ID has conflicting handles.');
            methodIds.set(native.id, native);
          }
        } else if (operation.binding.status === 'source-bound') report('UNBOUND_NATIVE_METHOD', path, 'Source links alone retain declared correspondence; no actual typed method is bound.', 'warning');
        return {
          ...operation,
          binding: { ...operation.binding, status: validNative ? operation.binding.status : 'declared' },
          ...(validNative && native ? { nativeMethodId: native.id, nativeOutputSchemaId: convert(native.output, 'output', [...path, 'output']) } : {}),
          inputSchemaId: convert(input, 'input', ['operations', i, 'input']),
          outcomes: outcomes.map(({ payload, ...outcome }, j) => ({ ...outcome, ...(payload ? { payloadSchemaId: convert(payload, 'output', ['operations', i, 'outcomes', j, 'payload']) } : {}) })),
        };
      }),
      processes: processes.map(({ input, ...process }, i) => ({ ...process, inputSchemaId: convert(input, 'input', ['processes', i, 'input']) })),
    }) as unknown as WorkbenchDefinition;
  } catch (error) {
    report('NONPORTABLE_DEFINITION', ['definition'], String(error));
    definition = { id: source.id, label: source.label, sources: [], schemas: [], contexts: [], modules: [], values: [], objects: [], rules: [], operations: [], relationships: [], processes: [] };
  }
  // The eager array reference above is copied after conversion has completed.
  for (const group of ['contexts', 'modules', 'values', 'objects', 'rules', 'operations', 'relationships', 'processes', 'sources', 'schemas'] as const) identities(definition[group], ['definition', group]);
  identities([definition], ['definition']);
  definition.sources.forEach((source, i) => checkUri(source.uri, ['definition', 'sources', i, 'uri']));
  const schemaMap = new Map(definition.schemas.map(item => [item.id, item]));
  for (const schema of definition.schemas) schemaCoverage(schema, report);
  const contextMap = new Map(definition.contexts.map(item => [item.id, item]));
  const moduleMap = new Map(definition.modules.map(item => [item.id, item]));
  const objectMap = new Map(definition.objects.map(item => [item.id, item]));
  const operationMap = new Map(definition.operations.map(item => [item.id, item]));
  const valueMap = new Map(definition.values.map(item => [item.id, item]));
  const ruleMap = new Map(definition.rules.map(item => [item.id, item]));
  const sourceIds = new Set(definition.sources.map(item => item.id));
  const ref = (found: unknown, id: string, path: FieldPath): void => { if (!found) report('BROKEN_REF', path, `Unknown reference ${id}.`); };
  const validPath = (path: FieldPath): boolean => Array.isArray(path) && path.every(key => typeof key === 'number' ? Number.isInteger(key) && key >= 0 : typeof key === 'string' && !['__proto__', 'constructor', 'prototype'].includes(key));
  function field(schemaId: string, path: FieldPath, at: FieldPath): void {
    if (!validPath(path)) report('INVALID_FIELD_PATH', at, 'Use safe property names or nonnegative array indexes.');
    else if (!schemaAt(schemaMap.get(schemaId), path)) report('MISSING_FIELD', at, `Field ${pathKey(path)} is absent or not supported by the native schema.`);
  }
  function annotations(fields: readonly InputField[], schemaId: string, at: FieldPath): void {
    identities(fields, at);
    for (const [i, item] of fields.entries()) { field(schemaId, item.path, [...at, i, 'path']); ref(valueMap.get(item.valueId), item.valueId, [...at, i, 'valueId']); }
  }
  definition.modules.forEach((module, i) => {
    const at = ['definition', 'modules', i];
    ref(contextMap.get(module.contextId), module.contextId, [...at, 'contextId']);
    if (!module.responsibility.trim()) report('MISSING_RESPONSIBILITY', at, 'Module responsibility is required.');
    module.dependencies.forEach((id, j) => ref(moduleMap.get(id), id, [...at, 'dependencies', j]));
  });
  definition.objects.forEach((object, i) => {
    const at = ['definition', 'objects', i];
    ref(moduleMap.get(object.moduleId), object.moduleId, [...at, 'moduleId']);
    if (!object.responsibility.trim()) report('MISSING_RESPONSIBILITY', at, 'Object responsibility is required.');
    identities(object.fields, [...at, 'fields']);
    for (const [j, item] of object.fields.entries()) {
      if (!item.derived) field(object.stateSchemaId, item.path, [...at, 'fields', j, 'path']);
      if (item.valueId) ref(valueMap.get(item.valueId), item.valueId, [...at, 'fields', j, 'valueId']);
      for (const path of item.derived?.sourcePaths ?? []) field(object.stateSchemaId, path, [...at, 'fields', j, 'derived']);
    }
    if (object.kind === 'entity' && !object.businessIdentity?.length) report('MISSING_BUSINESS_IDENTITY', [...at, 'businessIdentity'], 'An entity needs explicit business identity paths independent from instance/call IDs.');
    for (const path of object.businessIdentity ?? []) field(object.stateSchemaId, path, [...at, 'businessIdentity']);
    if (object.consistencyRootId) { const root = objectMap.get(object.consistencyRootId); ref(root, object.consistencyRootId, [...at, 'consistencyRootId']); if (root?.kind !== 'entity' || root.consistencyRootId && root.consistencyRootId !== root.id) report('INVALID_CONSISTENCY_ROOT', at, 'Consistency root must be a root entity, not a member of another root.'); }
    if (object.lifecycle) {
      field(object.stateSchemaId, object.lifecycle.path, [...at, 'lifecycle', 'path']);
      identities(object.lifecycle.transitions, [...at, 'lifecycle', 'transitions']);
      for (const transition of object.lifecycle.transitions) {
        const operation = operationMap.get(transition.operationId);
        ref(operation, transition.operationId, [...at, 'lifecycle', transition.id]);
        if (operation?.ownerId !== object.id) report('TRANSITION_OWNERSHIP', [...at, 'lifecycle', transition.id], 'A transition must be triggered by this owner’s public operation.');
        ref(operation?.outcomes.find(outcome => outcome.id === transition.outcomeId), transition.outcomeId, [...at, 'lifecycle', transition.id]);
      }
    }
  });
  definition.rules.forEach((rule, i) => {
    const at = ['definition', 'rules', i];
    ref(objectMap.get(rule.ownerId), rule.ownerId, [...at, 'ownerId']);
    for (const id of rule.sourceIds) ref(sourceIds.has(id), id, [...at, 'sourceIds']);
    if (rule.basis === 'source-bound' && !rule.sourceIds.length) report('MISSING_SOURCE_BINDING', at, 'Source-bound rules need provenance; this is not execution proof.');
    if (rule.check) {
      for (const operand of [rule.check.left, rule.check.right]) if (operand.kind === 'state') { const owner = objectMap.get(rule.ownerId); if (owner) field(owner.stateSchemaId, operand.path, [...at, 'check']); }
      if (!['eq', 'ne', 'gte', 'lte'].includes(rule.check.operator)) report('UNSUPPORTED_RULE', [...at, 'check'], 'Only inert eq/ne/gte/lte comparisons are implemented.');
    }
  });
  definition.operations.forEach((operation, i) => {
    const at = ['definition', 'operations', i];
    const owner = objectMap.get(operation.ownerId);
    ref(owner, operation.ownerId, [...at, 'ownerId']);
    annotations(operation.inputFields ?? [], operation.inputSchemaId, [...at, 'inputFields']);
    identities(operation.outcomes, [...at, 'outcomes']);
    if (!operation.outcomes.length) report('MISSING_OUTCOMES', at, 'Declare operation outcomes.');
    for (const outcome of operation.outcomes) if (outcome.payloadSchemaId) annotations(outcome.fields ?? [], outcome.payloadSchemaId, [...at, 'outcomes', outcome.id, 'fields']);
    for (const ruleId of operation.ruleIds) { const rule = ruleMap.get(ruleId); ref(rule, ruleId, [...at, 'ruleIds']); if (rule && rule.ownerId !== operation.ownerId) report('RULE_OWNERSHIP', [...at, 'ruleIds'], `Rule ${ruleId} belongs to another object.`); }
    for (const id of operation.binding.sourceIds) ref(sourceIds.has(id), id, [...at, 'binding']);
    if (operation.binding.status === 'source-bound' && !operation.binding.sourceIds.length) report('MISSING_SOURCE_BINDING', [...at, 'binding'], 'A source-bound operation requires provenance.');
    if (operation.kind === 'query' && operation.effects.some(effect => effect.fieldPaths.length)) report('QUERY_WRITE', [...at, 'effects'], 'Queries cannot declare domain-state writes.');
    for (const effect of operation.effects) {
      ref(operation.outcomes.find(outcome => outcome.id === effect.outcomeId), effect.outcomeId, [...at, 'effects']);
      for (const path of effect.fieldPaths) {
        if (owner) field(owner.stateSchemaId, path, [...at, 'effects']);
        if (owner?.businessIdentity?.some(identity => prefix(path, identity) || prefix(identity, path))) report('IDENTITY_WRITE', [...at, 'effects'], 'Public operations cannot replace business identity.');
        if (owner?.fields.some(item => item.derived && (prefix(item.path, path) || prefix(path, item.path)))) report('DERIVED_STATE_WRITE', [...at, 'effects'], 'Derived projections are not independently writable state.');
      }
    }
  });
  for (const [i, relationship] of definition.relationships.entries()) {
    const at = ['definition', 'relationships', i];
    ref(objectMap.get(relationship.fromObjectId), relationship.fromObjectId, at); ref(objectMap.get(relationship.toObjectId), relationship.toObjectId, at);
    if (relationship.kind === 'member' && objectMap.get(relationship.toObjectId)?.consistencyRootId !== relationship.fromObjectId) report('MEMBERSHIP_OWNERSHIP', at, 'Member relationship must agree with the member’s explicit consistency root.');
  }
  for (const [i, process] of definition.processes.entries()) {
    const at: FieldPath = ['definition', 'processes', i];
    identities(process.roles, [...at, 'roles']); identities(process.steps, [...at, 'steps']); identities(process.flows, [...at, 'flows']);
    annotations(process.inputFields ?? [], process.inputSchemaId, [...at, 'inputFields']);
    const roles = new Map(process.roles.map(role => [role.id, role]));
    const steps = new Map(process.steps.map(step => [step.id, step]));
    for (const role of process.roles) ref(objectMap.get(role.objectId), role.objectId, [...at, 'roles', role.id]);
    if (steps.get(process.startId)?.kind !== 'start') report('INVALID_START', [...at, 'startId'], 'Process start must reference its start step.');
    const outgoing = new Map(process.steps.map(step => [step.id, process.flows.filter(flow => flow.from === step.id)]));
    for (const flow of process.flows) {
      ref(steps.get(flow.from), flow.from, [...at, 'flows', flow.id, 'from']); ref(steps.get(flow.to), flow.to, [...at, 'flows', flow.id, 'to']);
      const from = steps.get(flow.from);
      if (from?.kind === 'operation') ref(operationMap.get(from.operationId)?.outcomes.find(outcome => outcome.id === flow.outcomeId), String(flow.outcomeId), [...at, 'flows', flow.id, 'outcomeId']);
      else if (flow.outcomeId) report('INVALID_BRANCH', [...at, 'flows', flow.id], 'Only operation flows select an outcome.');
      if (from?.kind === 'end') report('TERMINAL_FLOW', [...at, 'flows', flow.id], 'Terminal steps have no outgoing flow.');
    }
    const active = new Set<string>(); const visited = new Set<string>();
    const visitGraph = (id: string): void => {
      if (active.has(id)) { report('UNSUPPORTED_CYCLE', [...at, 'steps', id], 'Loop dataflow is not implemented.'); return; }
      if (visited.has(id)) return;
      active.add(id);
      for (const flow of outgoing.get(id) ?? []) visitGraph(flow.to);
      active.delete(id); visited.add(id);
    };
    visitGraph(process.startId);
    const reachable = new Set(visited);
    for (const step of process.steps) visitGraph(step.id);
    for (const step of process.steps) {
      if (!reachable.has(step.id)) report('UNREACHABLE_STEP', [...at, 'steps', step.id], 'Step is unreachable from process start.', 'warning');
      if (!['start', 'end', 'operation'].includes(step.kind)) report('UNSUPPORTED_PARALLEL', [...at, 'steps', step.id], 'Only sequential start/operation/end steps are implemented.');
      const branches = outgoing.get(step.id) ?? [];
      if (new Set(branches.map(flow => flow.outcomeId ?? '')).size !== branches.length) report('UNSUPPORTED_PARALLEL', [...at, 'steps', step.id], 'One outcome may select only one sequential successor.');
      if (step.kind === 'start' && branches.length !== 1) report('INVALID_START', [...at, 'steps', step.id], 'Start selects exactly one successor.');
      if (step.kind !== 'operation') continue;
      const operation = operationMap.get(step.operationId);
      ref(operation, step.operationId, [...at, 'steps', step.id, 'operationId']);
      const role = roles.get(step.roleId); const caller = roles.get(step.callerRoleId);
      ref(role, step.roleId, [...at, 'steps', step.id, 'roleId']); ref(caller, step.callerRoleId, [...at, 'steps', step.id, 'callerRoleId']);
      if (operation && role?.objectId !== operation.ownerId) report('METHOD_OWNERSHIP', [...at, 'steps', step.id], 'Target role must be an instance of the method owner.');
      const callerModule = objectMap.get(caller?.objectId ?? '')?.moduleId; const targetModule = objectMap.get(role?.objectId ?? '')?.moduleId;
      if (callerModule && targetModule && callerModule !== targetModule && !moduleMap.get(callerModule)?.dependencies.includes(targetModule)) report('MODULE_DEPENDENCY', [...at, 'steps', step.id], 'Cross-module calls require the caller’s direct declared dependency.');
      if (!operation) continue;
      identities(step.bindings, [...at, 'steps', step.id, 'bindings']);
      for (const required of requiredPaths(schemaMap.get(operation.inputSchemaId))) if (!step.bindings.some(binding => prefix(binding.targetPath, required))) report('MISSING_INPUT_SOURCE', [...at, 'steps', step.id, 'bindings'], `Required input ${pathKey(required)} has no source.`);
      for (const [j, binding] of step.bindings.entries()) {
        const here = [...at, 'steps', step.id, 'bindings', j];
        field(operation.inputSchemaId, binding.targetPath, [...here, 'targetPath']);
        if (step.bindings.slice(0, j).some(previous => prefix(previous.targetPath, binding.targetPath) || prefix(binding.targetPath, previous.targetPath))) report('OVERLAPPING_BINDINGS', here, 'An input field has competing sources.');
        const sourceInfo = parameterSchema(binding.source, process, operationMap, objectMap, schemaMap, report, here);
        const targetSchema = schemaAt(schemaMap.get(operation.inputSchemaId), binding.targetPath);
        if (binding.source.kind === 'literal' && targetSchema) {
          for (const error of valueErrors(binding.source.value, { id: 'literal', label: 'Literal', jsonSchema: targetSchema })) report('INCOMPATIBLE_VALUE', here, error);
        } else if (sourceInfo?.shape && targetSchema) {
          const result = compatible(sourceInfo.shape, targetSchema);
          if (result !== 'yes') report(result === 'no' ? 'INCOMPATIBLE_VALUE' : 'SCHEMA_COMPATIBILITY_UNSUPPORTED', here, result === 'no' ? 'Source schema cannot satisfy this input.' : 'Schema compatibility is not proved for this construct.', result === 'no' ? 'error' : 'warning');
        }
        const sourceAnnotations = sourceInfo?.fields ?? [];
        for (const targetField of operation.inputFields ?? []) {
          if (!prefix(binding.targetPath, targetField.path)) continue;
          const sourcePath = [...(binding.source.kind === 'literal' ? [] : binding.source.path), ...targetField.path.slice(binding.targetPath.length)];
          const sourceValueId = binding.source.kind === 'literal' ? binding.source.valueId : sourceAnnotations.find(item => pathKey(item.path) === pathKey(sourcePath))?.valueId;
          const declaredValueId = binding.valueId;
          if (sourceValueId && sourceValueId !== targetField.valueId || declaredValueId && declaredValueId !== targetField.valueId) report('INCOMPATIBLE_VALUE_MEANING', here, 'Field meaning/unit differs despite compatible primitive types.');
          else if (!sourceValueId && binding.source.kind !== 'identity') report('VALUE_MEANING_UNDECLARED', here, 'Source field meaning/unit is undeclared; shape checking is not semantic proof.', 'warning');
        }
        if (binding.source.kind === 'result') {
          const resultSource = binding.source;
          const seen = new Set<string>(); let bad = false; let reached = false;
          const walk = (id: string, available: boolean): void => {
            const key = `${id}:${available}`; if (seen.has(key)) return; seen.add(key);
            if (id === step.id) { reached = true; bad ||= !available; return; }
            for (const edge of outgoing.get(id) ?? []) walk(edge.to, id === resultSource.stepId ? edge.outcomeId === resultSource.outcomeId : available);
          };
          walk(process.startId, false);
          if (bad || !reached || resultSource.stepId === step.id) report('WRONG_BRANCH', here, 'The qualified result is not available on every path to this consumer.');
        }
      }
    }
  }
  identities(model.scenarios, ['scenarios']);
  const compiled = model.scenarios.map((scenario, i) => compileScenario(definition, scenario, i));
  diagnostics.push(...compiled.flatMap(scenario => scenario.diagnostics));
  const coverage: WorkbenchArtifact['coverage'] = [
    ...definition.rules.map(rule => ({ subjectId: rule.id, status: rule.check ? 'declared' as const : rule.basis, detail: rule.check ? 'Inert bounded comparison; per-scenario assertions carry actual results/basis.' : 'Text/source correspondence only; no predicate or handler was executed.' })),
    ...definition.operations.map(operation => ({ subjectId: operation.id, status: operation.binding.status, detail: operation.nativeMethodId ? 'Actual typed native method/input/output reference retained outside JSON. Branch projections, source bodies and implementation conformance are not inferred.' : 'Declared source correspondence only; no actual native method handle is bound.' })),
    ...diagnostics.filter(item => /UNSUPPORTED/.test(item.code)).map(item => ({ subjectId: String(item.path.find(part => typeof part === 'string') ?? definition.id), status: 'unsupported' as const, detail: item.message })),
    { subjectId: definition.id, status: 'declared', detail: 'Sequential acyclic process/dataflow only. No scheduler, arbitrary code inference, callbacks, concurrency, loop values, recovery protocol or universal invariant proof.' },
  ];
  const artifact = portableCopy({ id: definition.id, label: definition.label, format: 'flow-atlas/workbench-1', definition, scenarios: compiled, diagnostics, coverage }) as unknown as WorkbenchArtifact;
  return freeze(artifact);
}

function parameterSchema(source: ParameterSource, process: ProcessDefinition, operations: ReadonlyMap<string, WorkbenchDefinition['operations'][number]>, objects: ReadonlyMap<string, WorkbenchDefinition['objects'][number]>, schemas: ReadonlyMap<string, ValueSchema>, report: Report, path: FieldPath): { shape?: Readonly<Record<string, Json>>; fields: readonly InputField[] } | undefined {
  let schemaId: string | undefined; let fields: readonly InputField[] = [];
  if (source.kind === 'literal') return undefined;
  if (source.kind === 'input') { schemaId = process.inputSchemaId; fields = process.inputFields ?? []; }
  else if (source.kind === 'identity') {
    const object = objects.get(process.roles.find(role => role.id === source.roleId)?.objectId ?? '');
    if (!object?.businessIdentity?.some(identity => pathKey(identity) === pathKey(source.path))) report('PRIVATE_STATE_SOURCE', path, 'Role data sources may read declared business identity only. Other state requires public input/query.');
    schemaId = object?.stateSchemaId;
    fields = (object?.fields ?? []).filter(field => field.valueId).map(field => ({ ...field, valueId: field.valueId! }));
  } else if (source.kind === 'result') {
    const step = process.steps.find(step => step.id === source.stepId);
    const outcome = step?.kind === 'operation' ? operations.get(step.operationId)?.outcomes.find(outcome => outcome.id === source.outcomeId) : undefined;
    if (!outcome) report('BROKEN_REF', path, 'Result source must qualify an existing operation step and outcome.');
    if (!outcome?.payloadSchemaId || ['unknown', 'no-response', 'failed'].includes(outcome.meaning)) report('UNAVAILABLE_RESULT', path, 'This branch does not provide a consumable result.');
    schemaId = outcome?.payloadSchemaId; fields = outcome?.fields ?? [];
  } else report('UNSUPPORTED_INPUT_SOURCE', path, 'Only process input, identity, qualified results and constants are implemented.');
  const shape = schemaAt(schemas.get(schemaId ?? ''), source.path);
  if (!shape) report('MISSING_SOURCE_FIELD', path, 'Source field is absent or unsupported by its native schema.');
  return { shape, fields };
}
function freeze<T>(value: T): T {
  if (value && typeof value === 'object') { for (const child of Object.values(value)) freeze(child); Object.freeze(value); }
  return value;
}
/** @aiPurpose Return an accepted portable artifact only when no semantic errors remain.
 * @aiUsage Use inspect for drafts; unsupported coverage warnings remain visible on accepted bounded artifacts.
 */
export function compile(model: DomainModel): Checked<WorkbenchArtifact> {
  const value = inspect(model);
  return value.diagnostics.some(item => item.severity === 'error') ? { ok: false, diagnostics: value.diagnostics } : { ok: true, value, diagnostics: value.diagnostics };
}
