/** @aiPurpose Own implemented source-first domain semantics and the portable workbench/checkpoint contract.
 * @aiUsage Import public API from index.ts; native schema handles are removed before JSON export.
 */
import type { Diagnostic, Evidence, Id, Identity, Json, OutcomeMeaning, Source, ValueSchema } from '../design/contracts.ts';
import type { SchemaHandle } from '../authoring/contracts.ts';
export type { Checked, Diagnostic, Evidence, Id, Identity, Json, OutcomeMeaning, Source, ValueSchema } from '../design/contracts.ts';
export type { SchemaHandle } from '../authoring/contracts.ts';

export type FieldPath = readonly (string | number)[];
export interface Context extends Identity { readonly responsibility: string; }
export interface DomainModule extends Identity { readonly contextId: Id; readonly responsibility: string; readonly dependencies: readonly Id[]; }
export interface SemanticValue extends Identity { readonly schema: SchemaHandle; readonly meaning: string; readonly unit?: string; }
export interface ValueDefinition extends Omit<SemanticValue, 'schema'> { readonly schemaId: Id; }
/** Paths select native schema properties; derived fields are descriptive, never independently writable. */
export interface DomainField extends Identity { readonly path: FieldPath; readonly valueId?: Id; readonly derived?: { readonly description: string; readonly sourcePaths: readonly FieldPath[]; readonly projection?: { readonly kind: 'subtract-sum'; readonly totalPath: FieldPath; readonly itemsPath: FieldPath; readonly itemPath: FieldPath } }; }
export interface LifecycleTransition extends Identity { readonly from: Json; readonly to: Json; readonly operationId: Id; readonly outcomeId: Id; }
export interface DomainObject extends Identity {
  readonly moduleId: Id; readonly kind: 'entity' | 'value' | 'service' | 'process' | 'actor' | 'technical';
  readonly responsibility: string; readonly state: SchemaHandle; readonly fields: readonly DomainField[];
  readonly businessIdentity?: readonly FieldPath[];
  readonly consistencyRootId?: Id;
  readonly lifecycle?: { readonly path: FieldPath; readonly transitions: readonly LifecycleTransition[] };
}
export interface ObjectDefinition extends Omit<DomainObject, 'state'> { readonly stateSchemaId: Id; }
export type RuleOperand =
  | { readonly kind: 'state'; readonly path: FieldPath }
  | { readonly kind: 'input' | 'result'; readonly path: FieldPath }
  | { readonly kind: 'literal'; readonly value: Json };
export interface Comparison { readonly left: RuleOperand; readonly operator: 'eq' | 'ne' | 'gte' | 'lte'; readonly right: RuleOperand; }
export interface DomainRule extends Identity {
  readonly ownerId: Id; readonly description: string; readonly basis: 'declared' | 'source-bound';
  readonly sourceIds: readonly Id[]; readonly check?: Comparison;
  /** Restrict an operation rule to these results; omitted applies to all result branches. */
  readonly outcomeIds?: readonly Id[];
}
export interface InputField extends Identity { readonly path: FieldPath; readonly valueId: Id; }
export interface DomainOutcome extends Identity { readonly meaning: OutcomeMeaning; readonly payload?: SchemaHandle; readonly fields?: readonly InputField[]; }
export interface OutcomeDefinition extends Omit<DomainOutcome, 'payload'> { readonly payloadSchemaId?: Id; }
export interface OperationEffect { readonly outcomeId: Id; readonly fieldPaths: readonly FieldPath[]; }
/** Typed source correspondence only; the implementation function lives outside this surface and portable JSON. */
export interface NativeMethodHandle<I = unknown, O = unknown> extends Identity {
  readonly ownerId: Id; readonly input: SchemaHandle<I>; readonly output: SchemaHandle<O>;
}
export interface NativeMethodSpec<I, O> extends NativeMethodHandle<I, O> {
  readonly method: (input: NoInfer<I>) => NoInfer<O> | Promise<NoInfer<O>>;
}
export interface DomainOperation extends Identity {
  readonly ownerId: Id; readonly kind: 'command' | 'query'; readonly input: SchemaHandle;
  readonly inputFields?: readonly InputField[]; readonly outcomes: readonly DomainOutcome[];
  readonly effects: readonly OperationEffect[]; readonly ruleIds: readonly Id[];
  readonly binding: { readonly status: 'declared' | 'source-bound'; readonly sourceIds: readonly Id[] };
  readonly native?: NativeMethodHandle;
}
export interface OperationDefinition extends Omit<DomainOperation, 'input' | 'outcomes' | 'native'> {
  readonly inputSchemaId: Id; readonly outcomes: readonly OutcomeDefinition[];
  readonly nativeMethodId?: Id; readonly nativeOutputSchemaId?: Id;
}
export interface DomainRelationship extends Identity { readonly fromObjectId: Id; readonly toObjectId: Id; readonly kind: 'uses' | 'association' | 'member'; readonly cardinality?: string; }
export interface ProcessRole extends Identity { readonly objectId: Id; }
export type ParameterSource =
  | { readonly kind: 'input'; readonly path: FieldPath }
  | { readonly kind: 'identity'; readonly roleId: Id; readonly path: FieldPath }
  | { readonly kind: 'result'; readonly stepId: Id; readonly outcomeId: Id; readonly path: FieldPath }
  | { readonly kind: 'literal'; readonly value: Json; readonly valueId?: Id };
export interface ParameterBinding extends Identity { readonly targetPath: FieldPath; readonly source: ParameterSource; readonly valueId?: Id; }
export type DomainStep = Identity & (
  | { readonly kind: 'start' }
  | { readonly kind: 'end'; readonly outcome: 'completed' | 'rejected' | 'unknown' | 'failed' }
  | { readonly kind: 'operation'; readonly roleId: Id; readonly callerRoleId: Id; readonly operationId: Id; readonly bindings: readonly ParameterBinding[] }
);
export interface ControlFlow extends Identity { readonly from: Id; readonly to: Id; readonly outcomeId?: Id; }
export interface DomainProcess extends Identity {
  readonly input: SchemaHandle; readonly inputFields?: readonly InputField[];
  readonly roles: readonly ProcessRole[]; readonly steps: readonly DomainStep[]; readonly flows: readonly ControlFlow[]; readonly startId: Id;
}
export interface ProcessDefinition extends Omit<DomainProcess, 'input'> { readonly inputSchemaId: Id; }
export interface DomainDefinition extends Identity {
  readonly sources: readonly Source[]; readonly contexts: readonly Context[]; readonly modules: readonly DomainModule[];
  readonly values: readonly SemanticValue[]; readonly objects: readonly DomainObject[]; readonly rules: readonly DomainRule[];
  readonly operations: readonly DomainOperation[]; readonly relationships: readonly DomainRelationship[]; readonly processes: readonly DomainProcess[];
}
export interface WorkbenchDefinition extends Omit<DomainDefinition, 'values' | 'objects' | 'operations' | 'processes'> {
  readonly schemas: readonly ValueSchema[]; readonly values: readonly ValueDefinition[]; readonly objects: readonly ObjectDefinition[];
  readonly operations: readonly OperationDefinition[]; readonly processes: readonly ProcessDefinition[];
}
export interface ScenarioInstance extends Identity { readonly objectId: Id; readonly businessIdentity?: Json; readonly initial: Json; readonly consistencyRootInstanceId?: Id; }
export interface StateWrite { readonly instanceId: Id; readonly path: FieldPath; readonly value: Json; readonly transitionId?: Id; }
/** Effect is the owner's authoritative fact; outcome/payload is what the caller learned. */
export interface ScenarioVisit extends Identity {
  readonly stepId: Id; readonly viaFlowId?: Id; readonly callId: Id; readonly input: Json;
  readonly outcomeId: Id; readonly completion: 'reply' | 'local-outcome'; readonly payload?: Json; readonly evidence: Evidence;
  readonly effect?: { readonly outcomeId: Id; readonly writes: readonly StateWrite[]; readonly evidence: Evidence };
}
export type ScenarioExpectation = Identity & (
  | { readonly kind: 'state'; readonly instanceId: Id; readonly path: FieldPath; readonly equals: Json }
  | { readonly kind: 'call'; readonly visitId: Id; readonly outcomeId: Id }
  | { readonly kind: 'terminal'; readonly outcome: 'completed' | 'rejected' | 'unknown' | 'failed' }
);
export interface DomainScenario extends Identity {
  readonly processId: Id; readonly input: Json; readonly instances: readonly ScenarioInstance[];
  readonly roles: Readonly<Record<Id, Id>>; readonly visits: readonly ScenarioVisit[]; readonly terminalStepId: Id;
  readonly expectations: readonly ScenarioExpectation[];
}
export interface DomainModel { readonly definition: DomainDefinition; readonly scenarios: readonly DomainScenario[]; }
export interface StateDifference { readonly instanceId: Id; readonly path: FieldPath; readonly before?: Json; readonly after: Json; readonly causeVisitId: Id; readonly operationId: Id; readonly transitionId?: Id; readonly evidence: Evidence; }
export interface ParameterProvenance extends Identity {
  readonly targetPath: FieldPath; readonly source: ParameterSource; readonly sourceVisitId?: Id; readonly sourceCallId?: Id;
  readonly status: 'resolved' | 'unavailable' | 'mismatch'; readonly value?: Json; readonly actual?: Json;
}
export interface CallProjection {
  readonly callId: Id; readonly visitId: Id; readonly stepId: Id; readonly operationId: Id;
  readonly fromInstanceId: Id; readonly targetInstanceId: Id; readonly input: Json;
  readonly phase: 'pending' | OutcomeMeaning; readonly outcomeId?: Id; readonly payload?: Json;
  readonly completion?: 'reply' | 'local-outcome'; readonly effectOutcomeId?: Id; readonly evidence: Evidence;
}
export interface AssertionResult extends Identity { readonly status: 'passed' | 'failed' | 'declared' | 'unsupported'; readonly subjectId: Id; readonly basis: 'authored' | 'observed' | 'declared'; readonly message: string; }
/** Authoritative immutable projection; UI never reapplies writes or evaluates rules. */
export interface Checkpoint extends Identity {
  readonly index: number; readonly phase: 'initial' | 'call' | 'effect' | 'reply' | 'outcome' | 'terminal';
  readonly stepId?: Id; readonly visitId?: Id; readonly callId?: Id; readonly outcomeId?: Id;
  readonly states: Readonly<Record<Id, Json>>; readonly calls: readonly CallProjection[];
  readonly derived: Readonly<Record<Id, Readonly<Record<Id, Json>>>>;
  readonly parameters: readonly ParameterProvenance[]; readonly differences: readonly StateDifference[];
  readonly assertions: readonly AssertionResult[]; readonly diagnostics: readonly Diagnostic[];
}
export interface CompiledScenario extends DomainScenario { readonly checkpoints: readonly Checkpoint[]; readonly diagnostics: readonly Diagnostic[]; readonly assertions: readonly AssertionResult[]; }
export interface CoverageItem { readonly subjectId: Id; readonly status: 'declared' | 'source-bound' | 'checked-authored' | 'checked-observed' | 'unsupported'; readonly detail: string; }
export interface WorkbenchArtifact extends Identity {
  readonly format: 'flow-atlas/workbench-1'; readonly definition: WorkbenchDefinition;
  readonly scenarios: readonly CompiledScenario[]; readonly diagnostics: readonly Diagnostic[]; readonly coverage: readonly CoverageItem[];
}
