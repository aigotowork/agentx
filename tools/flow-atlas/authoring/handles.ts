/** @aiPurpose Construct shared authoring identities and keep native method storage outside portable JSON. */
import type {
  FlowHandle, Identity, ModuleHandle, ObjectHandle, OperationHandle, OperationSpec,
  ProcessHandle, RuntimeSchema, SchemaAdapter, SchemaHandle, StepBinding, StepHandle,
} from './contracts.ts';

interface NativeOperation {
  readonly method: (input: unknown) => unknown;
  readonly classify: (output: unknown) => string;
}
const nativeOperations = new WeakMap<OperationHandle, NativeOperation>();

/** @aiPurpose Preserve a native schema by reference without parsing or converting it.
 * @aiUsage Pass an imported runtime schema and explicit adapter; reuse the handle across operations.
 */
export function schema<T>(native: RuntimeSchema<T>, adapter: SchemaAdapter, identity: Identity): SchemaHandle<T> {
  return Object.freeze({ ...identity, native, adapter });
}

/** @aiPurpose Declare a module and its permitted direct dependency identities.
 * @aiUsage Dependencies reference module IDs; compilation resolves cycles under the model's explicit policy.
 */
export function defineModule(spec: Omit<ModuleHandle, 'dependencies'> & { readonly dependencies?: readonly string[] }): ModuleHandle {
  return Object.freeze({ ...spec, dependencies: Object.freeze([...(spec.dependencies ?? [])]) });
}

/** @aiPurpose Declare one domain object type owned by one exact module handle.
 * @aiUsage Reuse this handle for operation ownership and caller identity.
 */
export function defineObject(spec: ObjectHandle): ObjectHandle { return Object.freeze({ ...spec }); }

/** @aiPurpose Bind an existing method and schema handles to one inspectable operation identity.
 * @aiUsage Pass the native method unchanged; classify actual results only during explicit runtime invocation.
 */
export function defineOperation<I, O>(spec: OperationSpec<I, O>): OperationHandle<I, O> {
  const { method, classify, ...surface } = spec;
  const handle = Object.freeze({ ...surface, outcomes: Object.freeze([...surface.outcomes]) });
  nativeOperations.set(handle, {
    method: method as (input: unknown) => unknown,
    classify: classify as (output: unknown) => string,
  });
  return handle;
}

/** @aiPurpose Declare an inert start, terminal or exact operation step.
 * @aiUsage Operation steps reuse the operation handle; flows and runtime bindings reuse this returned step.
 */
export function step(spec: StepHandle): StepHandle { return Object.freeze({ ...spec }); }

/** @aiPurpose Declare an inert process edge between exact step handles.
 * @aiUsage Operation edges must select one declared outcome; start edges have no outcome.
 */
export function flow(spec: FlowHandle): FlowHandle { return Object.freeze({ ...spec }); }

/** @aiPurpose Declare a sequential process graph without running or scheduling its operations.
 * @aiUsage Share the exact step handles with flows and implementation bindings; loops are allowed.
 */
export function defineProcess(spec: ProcessHandle): ProcessHandle {
  return Object.freeze({ ...spec, steps: Object.freeze([...spec.steps]), flows: Object.freeze([...spec.flows]) });
}

/** @aiPurpose Declare that an implementation wrapper uses the intended operation at a stable process step.
 * @aiUsage Compilation rejects mismatched handles; creation never invokes the method.
 */
export function bindStep<I, O>(target: StepHandle, operation: OperationHandle<I, O>): StepBinding<I, O> {
  return Object.freeze({ step: target, operation });
}

/** @aiPurpose Internal bridge for the compiler and explicit observation wrapper.
 * @aiUsage Undefined means a forged or foreign operation handle; never serialize the returned functions.
 */
export function nativeOperation(handle: OperationHandle): NativeOperation | undefined { return nativeOperations.get(handle); }
