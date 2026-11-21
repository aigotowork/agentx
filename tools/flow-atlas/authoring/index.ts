/** @aiPurpose Expose a bounded source authoring library with separate pure inspection and opt-in observation.
 * @aiUsage Import core APIs here and the optional Zod adapter from ./adapters/zod.ts.
 */
export type * from './contracts.ts';
export { schema, defineModule, defineObject, defineOperation, step, flow, defineProcess, bindStep } from './handles.ts';
export { inspect, compile, lint } from './compiler.ts';
export { checkTrace, observeProcess } from './conformance.ts';
