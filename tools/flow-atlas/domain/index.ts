/** @aiPurpose Expose reusable pure domain compilation/checkpoints with no native example import or startup effects.
 * @aiUsage inspect retains draft diagnostics; compile requires an error-free artifact; snapshots are detached copies.
 */
export type * from './contracts.ts';
export { bindNativeMethod } from './binding.ts';
export { inspect, compile } from './compiler.ts';
export { snapshot, changes, checkpoint } from './checkpoints.ts';
