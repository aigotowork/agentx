/** @aiPurpose Retain typed native method references as inert source correspondence without creating runtime objects or exporting functions to JSON. */
import type { NativeMethodHandle, NativeMethodSpec } from './contracts.ts';

type Implementation = (input: unknown) => unknown;
const methods = new WeakMap<NativeMethodHandle, Implementation>();

/** @aiPurpose Bind an actual native method value to its original input/output schemas and owning type.
 * @aiUsage Pass a real function or prototype method with imported SchemaHandles; NoInfer checks its signature against those schemas.
 * @aiBehavior The method is retained by reference without invocation; this is correspondence, not arbitrary source or business-rule proof.
 */
export function bindNativeMethod<I, O>(spec: NativeMethodSpec<I, O>): NativeMethodHandle<I, O> {
  const { method, ...surface } = spec;
  if (typeof method !== 'function') throw new TypeError('A native method binding requires an actual callable method reference.');
  const handle = Object.freeze({ ...surface });
  methods.set(handle, method as Implementation);
  return handle;
}

/** @aiPurpose Resolve registered method identity for compiler ownership checks without executing it.
 * @aiUsage Internal compiler bridge; undefined identifies a copied, forged or foreign handle. Never serialize or invoke during inspection.
 */
export function nativeMethod(handle: NativeMethodHandle): Implementation | undefined { return methods.get(handle); }
