/** @aiPurpose Compile and inspect a bounded source authoring graph while keeping native methods opaque. */
import type { ValueSchema } from '../design/contracts.ts';
import type {
  AuthoringModel, Checked, Diagnostic, Identity, InspectorArtifact, OperationHandle,
  ProcessHandle, SchemaHandle, StepHandle,
} from './contracts.ts';
import { nativeOperation } from './handles.ts';
import { portableCopy } from './portable.ts';

type Path = readonly (string | number)[];
const coverage = [
  'Explicit authoring handles and direct declared module calls only; arbitrary source is not inspected.',
  'Sequential observed operation paths only; no workflow scheduler, concurrent traces or automatic retries.',
  'Native schema conversion exports structure; business refinements and arbitrary implementation correctness are not proved.',
  'Full Definition/Scenario/state/event/layout compilation and oRPC import are not implemented by this slice.',
] as const;

/** @aiPurpose Produce a plain JSON inspector document, retaining precise errors for incomplete drafts.
 * @aiUsage Never treats errors as accepted compilation; use compile when an artifact must be valid.
 * @aiBehavior Calls explicit schema exporters only; never methods, classifiers or schema parse hooks.
 */
export function inspect(model: AuthoringModel): InspectorArtifact {
  const diagnostics: Diagnostic[] = [];
  const report = (code: string, path: Path, message: string, severity: Diagnostic['severity'] = 'error') => {
    diagnostics.push({ code, path, message, severity });
  };
  function identity(value: Identity, path: Path): Identity {
    const result: { id: string; label: string; metadata?: Identity['metadata']; explanation?: Identity['explanation'] } = { id: value.id, label: value.label };
    if (!/^[A-Za-z0-9][A-Za-z0-9_.:-]*$/.test(value.id) || ['__proto__', 'prototype', 'constructor'].includes(value.id)) report('INVALID_ID', [...path, 'id'], 'Use a stable nonempty safe identity.');
    if (!value.label.trim()) report('INVALID_LABEL', [...path, 'label'], 'A visible label is required.');
    for (const field of ['metadata', 'explanation'] as const) {
      if (value[field] === undefined) continue;
      try {
        const copied = portableCopy(value[field]);
        if (field === 'metadata') result.metadata = copied as unknown as Identity['metadata'];
        else result.explanation = copied as unknown as Identity['explanation'];
      } catch (error) { report('NONPORTABLE_METADATA', [...path, field], String(error)); }
    }
    if (result.metadata?.extensions) {
      for (const key of Object.keys(result.metadata.extensions)) {
        if (!/^[A-Za-z0-9_.-]+:[A-Za-z0-9_.:-]+$/.test(key)) report('EXTENSION_NAMESPACE', [...path, 'metadata', 'extensions', key], 'Extension keys require an owner namespace.');
      }
    }
    const documents = result.metadata?.documents ?? [];
    const documentIds = new Set<string>();
    for (const [i, doc] of documents.entries()) {
      if (!doc.id.trim() || documentIds.has(doc.id)) report('INVALID_DOCUMENT_REF', [...path, 'metadata', 'documents', i], 'Document references require a unique nonempty identity.');
      documentIds.add(doc.id);
    }
    for (const field of ['documents', 'sourceLinks'] as const) {
      for (const [i, link] of (result.metadata?.[field] ?? []).entries()) {
        const uri = link.uri.trim();
        if (!uri || /[\u0000-\u0020\u007f\\]/.test(uri) || uri.startsWith('//') || (/^[A-Za-z][A-Za-z0-9+.-]*:/.test(uri) && !/^https?:\/\//i.test(uri))) report('UNSAFE_DOCUMENT_URI', [...path, 'metadata', field, i, 'uri'], 'Use a nonempty relative path or HTTP(S) document reference.');
      }
    }
    return result;
  }
  function unique<T extends Identity>(items: readonly T[], path: Path): Map<string, T> {
    const values = new Map<string, T>();
    items.forEach((item, index) => {
      if (values.has(item.id)) report('DUPLICATE_ID', [...path, index, 'id'], `Duplicate identity ${item.id}.`);
      else values.set(item.id, item);
    });
    return values;
  }
  const domains = unique(model.domains, ['domains']);
  const modules = unique(model.modules, ['modules']);
  const objects = unique(model.objects, ['objects']);
  const operations = unique(model.operations, ['operations']);
  unique(model.processes, ['processes']);

  function checkCycles(edges: ReadonlyMap<string, readonly string[]>, path: Path, severity: Diagnostic['severity'] = 'error'): void {
    const active = new Set<string>();
    const done = new Set<string>();
    function visit(id: string, chain: readonly string[]): void {
      if (active.has(id)) { report('DEPENDENCY_CYCLE', path, [...chain, id].join(' -> '), severity); return; }
      if (done.has(id)) return;
      active.add(id);
      for (const target of edges.get(id) ?? []) if (edges.has(target)) visit(target, [...chain, id]);
      active.delete(id);
      done.add(id);
    }
    for (const id of edges.keys()) visit(id, []);
  }
  const dependencies = new Map(model.modules.map(module => [module.id, module.dependencies]));
  const cyclePolicy = model.policy?.dependencyCycles ?? 'warning';
  if (cyclePolicy !== 'allow') checkCycles(dependencies, ['modules', 'dependencies'], cyclePolicy);
  model.domains.forEach((domain, i) => {
    if (domain.parentId && !domains.has(domain.parentId)) report('BROKEN_REF', ['domains', i, 'parentId'], `Unknown domain ${domain.parentId}.`);
  });
  checkCycles(new Map(model.domains.map(domain => [domain.id, domain.parentId ? [domain.parentId] : []])), ['domains', 'parentId']);
  model.modules.forEach((module, i) => {
    if (!domains.has(module.domainId)) report('BROKEN_REF', ['modules', i, 'domainId'], `Unknown domain ${module.domainId}.`);
    if (module.parentId && !modules.has(module.parentId)) report('BROKEN_REF', ['modules', i, 'parentId'], `Unknown module ${module.parentId}.`);
    const seen = new Set<string>();
    module.dependencies.forEach((id, j) => {
      if (!modules.has(id)) report('BROKEN_REF', ['modules', i, 'dependencies', j], `Unknown module ${id}.`);
      if (seen.has(id)) report('DUPLICATE_REF', ['modules', i, 'dependencies', j], `Dependency ${id} was declared twice.`);
      seen.add(id);
    });
  });
  checkCycles(new Map(model.modules.map(module => [module.id, module.parentId ? [module.parentId] : []])), ['modules', 'parentId']);
  model.objects.forEach((object, i) => {
    if (modules.get(object.module.id) !== object.module) report('BROKEN_REF', ['objects', i, 'module'], 'Object must reuse its declared module handle.');
  });

  const schemas: ValueSchema[] = [];
  const schemaOwners = new Map<string, SchemaHandle>();
  function convert(handle: SchemaHandle, io: 'input' | 'output', path: Path): string {
    const id = `${handle.id}:${io}`;
    const previous = schemaOwners.get(id);
    if (previous) {
      if (previous.native !== handle.native || previous.adapter !== handle.adapter) report('DUPLICATE_SCHEMA_OWNERSHIP', path, `Schema identity ${id} has different native sources or adapters.`);
      return id;
    }
    schemaOwners.set(id, handle);
    const meta = identity(handle, path);
    try {
      const converted = handle.adapter.convert(handle.native, { id, label: handle.label, io });
      for (const item of converted.diagnostics) diagnostics.push({ ...item, path: [...path, ...item.path] });
      if (converted.ok) {
        if (converted.value.id !== id) report('SCHEMA_ID_MISMATCH', path, 'Schema adapter must preserve the requested identity.');
        const data = portableCopy(converted.value.jsonSchema);
        if (data === null || typeof data !== 'object' || Array.isArray(data)) report('NONPORTABLE_SCHEMA', path, 'Adapter must return a JSON Schema object.');
        else schemas.push({ ...meta, id, jsonSchema: data as Readonly<Record<string, import('./contracts.ts').Json>> });
      } else if (!converted.diagnostics.some(item => item.severity === 'error')) report('SCHEMA_CONVERSION_FAILED', path, 'Schema adapter refused conversion.');
    } catch (error) { report('SCHEMA_CONVERSION_FAILED', path, String(error)); }
    return id;
  }
  const methodOwners = new Map<unknown, OperationHandle>();
  const exportedOperations = model.operations.map((operation, i) => {
    const path: Path = ['operations', i];
    if (objects.get(operation.owner.id) !== operation.owner) report('BROKEN_REF', [...path, 'owner'], 'Operation must reuse its declared object handle.');
    const native = nativeOperation(operation);
    if (!native) report('FOREIGN_OPERATION', path, 'Use defineOperation to bind an actual native method.');
    else {
      const owner = methodOwners.get(native.method);
      if (owner && owner !== operation) report('DUPLICATE_OPERATION_OWNERSHIP', path, `The native method is already owned by ${owner.id}.`);
      methodOwners.set(native.method, operation);
    }
    const outcomes = unique(operation.outcomes, [...path, 'outcomes']);
    if (!outcomes.size) report('MISSING_OUTCOMES', [...path, 'outcomes'], 'At least one explicit outcome is required.');
    for (const [field, meaning] of [['failureOutcomeId', 'failed'], ['unknownOutcomeId', 'unknown'], ['noResponseOutcomeId', 'no-response']] as const) {
      const id = operation[field];
      if (id !== undefined && outcomes.get(id)?.meaning !== meaning) report('INVALID_OUTCOME_BINDING', [...path, field], `Bind a declared ${meaning} outcome.`);
    }
    return {
      ...identity(operation, path), ownerId: operation.owner.id, moduleId: operation.owner.module.id, kind: operation.kind,
      inputSchemaId: convert(operation.input, 'input', [...path, 'input']),
      outputSchemaId: convert(operation.output, 'output', [...path, 'output']),
      outcomes: operation.outcomes.map((outcome, j) => ({ ...identity(outcome, [...path, 'outcomes', j]), meaning: outcome.meaning })),
    };
  });

  const allSteps = new Set(model.processes.flatMap(process => process.steps));
  const bound = new Map<StepHandle, OperationHandle>();
  model.bindings.forEach((binding, i) => {
    if (!allSteps.has(binding.step)) report('ORPHAN_BINDING', ['bindings', i, 'step'], 'Binding references a removed or foreign process step.');
    if (bound.has(binding.step)) report('DUPLICATE_BINDING', ['bindings', i], 'A process step has more than one implementation binding.');
    bound.set(binding.step, binding.operation);
    if (binding.step.kind !== 'operation' || binding.step.operation !== binding.operation) report('WRONG_OPERATION_BINDING', ['bindings', i, 'operation'], 'Implementation must reuse the operation handle owned by this step.');
    if (operations.get(binding.operation.id) !== binding.operation) report('BROKEN_REF', ['bindings', i, 'operation'], 'Binding operation is not registered in this model.');
  });

  function projectProcess(process: ProcessHandle, index: number): InspectorArtifact['processes'][number] {
    const path: Path = ['processes', index];
    const steps = unique(process.steps, [...path, 'steps']);
    unique(process.flows, [...path, 'flows']);
    if (steps.get(process.start.id) !== process.start || process.start.kind !== 'start') report('INVALID_START', [...path, 'start'], 'Start must reference this process start handle.');
    if (process.steps.filter(item => item.kind === 'start').length !== 1) report('INVALID_START', [...path, 'steps'], 'Exactly one start step is supported.');
    const outgoing = new Map<StepHandle, typeof process.flows[number][]>();
    const incoming = new Map<StepHandle, StepHandle[]>();
    const flows = process.flows.map((edge, i) => {
      const edgePath = [...path, 'flows', i];
      if (steps.get(edge.from.id) !== edge.from) report('BROKEN_REF', [...edgePath, 'from'], 'Flow source must reuse a step handle in this process.');
      if (steps.get(edge.to.id) !== edge.to) report('BROKEN_REF', [...edgePath, 'to'], 'Flow target must reuse a step handle in this process.');
      outgoing.set(edge.from, [...(outgoing.get(edge.from) ?? []), edge]);
      incoming.set(edge.to, [...(incoming.get(edge.to) ?? []), edge.from]);
      if (edge.to.kind === 'start') report('INVALID_START_FLOW', [...edgePath, 'to'], 'The start boundary cannot be entered again.');
      if (edge.from.kind === 'operation') {
        if (!edge.from.operation.outcomes.some(outcome => outcome.id === edge.outcomeId)) report('UNDECLARED_OUTCOME', [...edgePath, 'outcomeId'], 'Operation edges must select a declared outcome.');
      } else if (edge.outcomeId !== undefined) report('UNEXPECTED_OUTCOME', [...edgePath, 'outcomeId'], 'Only operation edges select outcomes.');
      return { ...identity(edge, edgePath), from: edge.from.id, to: edge.to.id, ...(edge.outcomeId === undefined ? {} : { outcomeId: edge.outcomeId }) };
    });
    function reachable(seeds: readonly StepHandle[], next: (item: StepHandle) => readonly StepHandle[]): Set<StepHandle> {
      const visited = new Set<StepHandle>();
      const pending = [...seeds];
      while (pending.length) {
        const item = pending.pop()!;
        if (visited.has(item)) continue;
        visited.add(item);
        pending.push(...next(item).filter(target => !visited.has(target)));
      }
      return visited;
    }
    const reached = reachable([process.start], item => (outgoing.get(item) ?? []).map(edge => edge.to));
    const terminates = reachable(process.steps.filter(item => item.kind === 'end'), item => incoming.get(item) ?? []);
    const exportedSteps: InspectorArtifact['processes'][number]['steps'][number][] = process.steps.map((item, i) => {
      const itemPath = [...path, 'steps', i];
      const edges = outgoing.get(item) ?? [];
      if (!reached.has(item)) report('UNREACHABLE_STEP', itemPath, 'Step is unreachable from the process start.');
      if (!terminates.has(item)) report('DEAD_END_STEP', itemPath, 'Step has no path to an explicit terminal.');
      if (item.kind === 'end' && edges.length) report('TERMINAL_HAS_FLOW', itemPath, 'Terminal steps cannot have outgoing flows.');
      if (item.kind === 'start' && edges.length !== 1) report('AMBIGUOUS_FLOW', itemPath, 'Start must have exactly one outgoing flow.');
      if (item.kind === 'operation') {
        if (objects.get(item.caller.id) !== item.caller) report('BROKEN_REF', [...itemPath, 'caller'], 'Caller must reuse a declared object handle.');
        if (operations.get(item.operation.id) !== item.operation) report('BROKEN_REF', [...itemPath, 'operation'], 'Step must reuse a declared operation handle.');
        const sourceModule = item.caller.module;
        const targetModule = item.operation.owner.module;
        if (sourceModule !== targetModule && !sourceModule.dependencies.includes(targetModule.id)) report('FORBIDDEN_MODULE_CALL', itemPath, `Module ${sourceModule.id} does not allow calls to ${targetModule.id}.`);
        for (const outcome of item.operation.outcomes) {
          const matches = edges.filter(edge => edge.outcomeId === outcome.id);
          if (!matches.length) report('UNHANDLED_OUTCOME', itemPath, `No flow handles outcome ${outcome.id}.`);
          if (matches.length > 1) report('AMBIGUOUS_FLOW', itemPath, `More than one flow handles outcome ${outcome.id}.`);
        }
        const isBound = bound.get(item) === item.operation;
        if (!isBound) report('UNBOUND_STEP', itemPath, 'No implementation binding uses this exact step/operation pair.');
        return { ...identity(item, itemPath), kind: 'operation', callerId: item.caller.id, operationId: item.operation.id, bound: isBound };
      }
      return item.kind === 'end'
        ? { ...identity(item, itemPath), kind: 'end', outcome: item.outcome }
        : { ...identity(item, itemPath), kind: 'start' };
    });
    return { ...identity(process, path), startId: process.start.id, steps: exportedSteps, flows };
  }
  const artifact: InspectorArtifact = {
    ...identity(model, []), format: 'flow-atlas/inspector-1',
    domains: model.domains.map((item, i) => ({ ...identity(item, ['domains', i]), ...(item.parentId === undefined ? {} : { parentId: item.parentId }) })),
    modules: model.modules.map((item, i) => ({ ...identity(item, ['modules', i]), domainId: item.domainId, dependencies: [...item.dependencies], ...(item.parentId === undefined ? {} : { parentId: item.parentId }) })),
    objects: model.objects.map((item, i) => ({ ...identity(item, ['objects', i]), kind: item.kind, moduleId: item.module.id, domainId: item.module.domainId })),
    schemas, operations: exportedOperations, processes: model.processes.map(projectProcess), diagnostics, coverage,
  };
  // Retain no aliases into the consumer's mutable metadata, schemas or dependency arrays.
  return portableCopy(artifact) as unknown as InspectorArtifact;
}

/** @aiPurpose Reject structurally broken drafts before exporting an accepted design artifact.
 * @aiUsage Use inspect to view invalid drafts; compile returns no value when any error exists.
 */
export function compile(model: AuthoringModel): Checked<InspectorArtifact> {
  const artifact = inspect(model);
  return artifact.diagnostics.some(item => item.severity === 'error')
    ? { ok: false, diagnostics: artifact.diagnostics }
    : { ok: true, value: artifact, diagnostics: artifact.diagnostics };
}

/** @aiPurpose Report authoring errors without invoking business code.
 * @aiUsage Share the same diagnostic codes and paths as inspect/compile.
 */
export function lint(model: AuthoringModel): readonly Diagnostic[] { return inspect(model).diagnostics; }
