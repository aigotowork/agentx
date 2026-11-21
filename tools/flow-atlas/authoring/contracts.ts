/** @aiPurpose Own the bounded source-binding, portable inspector and explicit observation contracts.
 * Full Definition/Scenario compilation remains in ../design.
 */
import type { Checked, Diagnostic, Domain, Id, Identity, Module, ModuleDependencyPolicy, ObjectType, OutcomeMeaning, SchemaExportContext, ValueSchema } from '../design/contracts.ts';

export type { Checked, Diagnostic, DesignMetadata, Id, Identity, Json } from '../design/contracts.ts';
export interface RuntimeSchema<T> { parse(value: unknown): T; }
export interface SchemaAdapter {
  convert(schema: unknown, context: SchemaExportContext): Checked<ValueSchema>;
}
export interface SchemaHandle<T = unknown> extends Identity {
  readonly native: RuntimeSchema<T>;
  readonly adapter: SchemaAdapter;
}
export interface ModuleHandle extends Module { readonly dependencies: readonly Id[]; }
export interface ObjectHandle extends Identity {
  readonly module: ModuleHandle;
  readonly kind: ObjectType['kind'];
}
export interface AuthoringOutcome extends Identity { readonly meaning: OutcomeMeaning; }
export interface OperationSpec<I, O> extends Identity {
  readonly owner: ObjectHandle;
  readonly kind: 'command' | 'query';
  readonly input: SchemaHandle<I>;
  readonly output: SchemaHandle<O>;
  readonly outcomes: readonly AuthoringOutcome[];
  readonly method: (input: NoInfer<I>) => NoInfer<O> | Promise<NoInfer<O>>;
  /** Runs only after an explicit runtime invocation, never during inspection. */
  readonly classify: (output: NoInfer<O>) => Id;
  readonly failureOutcomeId?: Id;
  readonly unknownOutcomeId?: Id;
  readonly noResponseOutcomeId?: Id;
}
export interface OperationHandle<I = unknown, O = unknown> extends Identity {
  readonly owner: ObjectHandle;
  readonly kind: 'command' | 'query';
  readonly input: SchemaHandle<I>;
  readonly output: SchemaHandle<O>;
  readonly outcomes: readonly AuthoringOutcome[];
  readonly failureOutcomeId?: Id;
  readonly unknownOutcomeId?: Id;
  readonly noResponseOutcomeId?: Id;
}
export type StepHandle = Identity & (
  | { readonly kind: 'start' }
  | { readonly kind: 'end'; readonly outcome: 'completed' | 'rejected' | 'unknown' | 'failed' }
  | { readonly kind: 'operation'; readonly caller: ObjectHandle; readonly operation: OperationHandle }
);
export interface FlowHandle extends Identity {
  readonly from: StepHandle;
  readonly to: StepHandle;
  readonly outcomeId?: Id;
}
export interface ProcessHandle extends Identity {
  readonly start: StepHandle;
  readonly steps: readonly StepHandle[];
  readonly flows: readonly FlowHandle[];
}
export interface StepBinding<I = unknown, O = unknown> {
  readonly step: StepHandle;
  readonly operation: OperationHandle<I, O>;
}
export interface AuthoringModel extends Identity {
  readonly domains: readonly Domain[];
  readonly modules: readonly ModuleHandle[];
  readonly objects: readonly ObjectHandle[];
  readonly operations: readonly OperationHandle[];
  readonly processes: readonly ProcessHandle[];
  readonly bindings: readonly StepBinding[];
  readonly policy?: ModuleDependencyPolicy;
}

/** Stable plain JSON input for offline inspectors. No functions, native schemas or live resources. */
export interface InspectorArtifact extends Identity {
  readonly format: 'flow-atlas/inspector-1';
  readonly domains: readonly Domain[];
  readonly modules: readonly (Module & { readonly dependencies: readonly Id[] })[];
  readonly objects: readonly (Identity & { readonly moduleId: Id; readonly domainId: Id; readonly kind: ObjectType['kind'] })[];
  readonly schemas: readonly ValueSchema[];
  readonly operations: readonly (Identity & {
    readonly ownerId: Id; readonly moduleId: Id; readonly kind: 'command' | 'query';
    readonly inputSchemaId: Id; readonly outputSchemaId: Id;
    readonly outcomes: readonly AuthoringOutcome[];
  })[];
  readonly processes: readonly (Identity & {
    readonly startId: Id;
    readonly steps: readonly (Identity & (
      | { readonly kind: 'start' }
      | { readonly kind: 'end'; readonly outcome: 'completed' | 'rejected' | 'unknown' | 'failed' }
      | { readonly kind: 'operation'; readonly callerId: Id; readonly operationId: Id; readonly bound: boolean }
    ))[];
    readonly flows: readonly (Identity & { readonly from: Id; readonly to: Id; readonly outcomeId?: Id })[];
  })[];
  readonly diagnostics: readonly Diagnostic[];
  readonly coverage: readonly string[];
}
export interface ObservedStep {
  readonly sequence: number;
  readonly stepId: Id;
  readonly operationId: Id;
  readonly status: 'returned' | 'failed' | 'unknown' | 'no-response';
  readonly outcomeId?: Id;
  readonly reason?: string;
}
export interface TraceReport {
  readonly processId: Id;
  readonly complete: boolean;
  readonly terminalStepId?: Id;
  readonly diagnostics: readonly Diagnostic[];
}
export interface ProcessObservation {
  /** Only this explicit request invokes a method. Calls must be awaited sequentially. */
  invoke<I, O>(binding: StepBinding<I, O>, input: NoInfer<I>): Promise<O>;
  /** Records external uncertainty without invoking or retrying a handler. */
  record(binding: StepBinding, status: 'unknown' | 'no-response', reason: string): void;
  trace(): readonly ObservedStep[];
  check(options?: { readonly complete?: boolean }): TraceReport;
}
