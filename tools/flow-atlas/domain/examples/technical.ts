/** @aiPurpose Demonstrate reuse for a technical playback lifecycle without product IDs, aggregates or domain-specific compiler branches. */
import * as z from 'zod';
import { schema } from '../../authoring/handles.ts';
import { zodAdapter } from '../../authoring/adapters/zod.ts';
import type { DomainModel } from '../contracts.ts';
const state = schema(z.object({ phase: z.enum(['playing', 'paused']) }), zodAdapter, { id: 'tech.schema.playback', label: 'Playback state' });
const input = schema(z.object({ reason: z.string().min(1) }), zodAdapter, { id: 'tech.schema.pause-input', label: 'Pause input' });
const empty = schema(z.object({}), zodAdapter, { id: 'tech.schema.empty', label: 'Actor state' });
const reason = schema(z.string().min(1), zodAdapter, { id: 'tech.schema.reason', label: 'Pause reason' });
/** @aiPurpose Author a second small consumer of the same public model/checkpoint compiler.
 * @aiUsage Compile independently; it has no entities/business identity, native runtime, UI dependency or aggregate requirement.
 */
export function createTechnicalModel(): DomainModel {
  return {
    definition: {
      id: 'technical-playback-design', label: '技术领域：拖动前暂停', sources: [],
      contexts: [{ id: 'tech.context', label: 'Presentation', responsibility: 'Illustrative playback lifecycle' }],
      modules: [{ id: 'tech.module', label: 'Playback control', contextId: 'tech.context', responsibility: 'Pause illustration before independent layout changes', dependencies: [] }],
      values: [{ id: 'tech.value.reason', label: 'Reason', schema: reason, meaning: 'A local interaction reason' }],
      objects: [
        { id: 'tech.playback', label: 'Playback', moduleId: 'tech.module', kind: 'technical', responsibility: 'Own presentation clock state', state, fields: [{ id: 'tech.field.phase', label: 'Phase', path: ['phase'] }], lifecycle: { path: ['phase'], transitions: [{ id: 'tech.transition.pause', label: 'Pause', from: 'playing', to: 'paused', operationId: 'tech.pause', outcomeId: 'paused' }] } },
        { id: 'tech.pointer', label: 'Pointer interaction', moduleId: 'tech.module', kind: 'actor', responsibility: 'Request pause before moving layout', state: empty, fields: [] },
      ],
      rules: [{ id: 'tech.rule.layout', label: 'Layout does not mutate semantic facts', ownerId: 'tech.playback', description: 'Layout and model have independent ownership; this text is not executable proof.', basis: 'declared', sourceIds: [] }],
      operations: [{ id: 'tech.pause', label: 'pause', ownerId: 'tech.playback', kind: 'command', input, inputFields: [{ id: 'tech.input.reason', label: 'Reason', path: ['reason'], valueId: 'tech.value.reason' }], outcomes: [{ id: 'paused', label: 'Paused', meaning: 'returned' }], effects: [{ outcomeId: 'paused', fieldPaths: [['phase']] }], ruleIds: [], binding: { status: 'declared', sourceIds: [] } }],
      relationships: [{ id: 'tech.relation.pause', label: 'Requests pause', fromObjectId: 'tech.pointer', toObjectId: 'tech.playback', kind: 'uses' }],
      processes: [{ id: 'tech.process', label: 'Pause before drag', input, inputFields: [{ id: 'tech.process.reason', label: 'Reason', path: ['reason'], valueId: 'tech.value.reason' }], roles: [{ id: 'tech.role.pointer', label: 'Interaction', objectId: 'tech.pointer' }, { id: 'tech.role.playback', label: 'Player', objectId: 'tech.playback' }], startId: 'tech.start', steps: [
        { id: 'tech.start', label: 'Drag begins', kind: 'start' },
        { id: 'tech.step.pause', label: 'Pause', kind: 'operation', roleId: 'tech.role.playback', callerRoleId: 'tech.role.pointer', operationId: 'tech.pause', bindings: [{ id: 'tech.binding.reason', label: 'Interaction input', targetPath: [], source: { kind: 'input', path: [] } }] },
        { id: 'tech.end', label: 'Layout may move', kind: 'end', outcome: 'completed' },
      ], flows: [{ id: 'tech.flow.start', label: 'Pause request', from: 'tech.start', to: 'tech.step.pause' }, { id: 'tech.flow.paused', label: 'Paused', from: 'tech.step.pause', to: 'tech.end', outcomeId: 'paused' }] }],
    },
    scenarios: [{ id: 'tech.scenario', label: 'Drag pauses playback', processId: 'tech.process', input: { reason: 'pointer-drag' }, instances: [{ id: 'tech.player-1', label: 'Player one', objectId: 'tech.playback', initial: { phase: 'playing' } }, { id: 'tech.player-2', label: 'Player two', objectId: 'tech.playback', initial: { phase: 'playing' } }, { id: 'tech.pointer-1', label: 'Pointer', objectId: 'tech.pointer', initial: {} }], roles: { 'tech.role.playback': 'tech.player-1', 'tech.role.pointer': 'tech.pointer-1' }, visits: [{ id: 'tech.visit.pause', label: 'Pause for drag', stepId: 'tech.step.pause', callId: 'tech.call.pause', input: { reason: 'pointer-drag' }, completion: 'reply', outcomeId: 'paused', evidence: { kind: 'authored' }, effect: { outcomeId: 'paused', evidence: { kind: 'authored' }, writes: [{ instanceId: 'tech.player-1', path: ['phase'], value: 'paused', transitionId: 'tech.transition.pause' }] } }], terminalStepId: 'tech.end', expectations: [{ id: 'tech.expect.paused', label: 'Target paused', kind: 'state', instanceId: 'tech.player-1', path: ['phase'], equals: 'paused' }, { id: 'tech.expect.other', label: 'Other player unchanged', kind: 'state', instanceId: 'tech.player-2', path: ['phase'], equals: 'playing' }, { id: 'tech.expect.end', label: 'Completed', kind: 'terminal', outcome: 'completed' }] }],
  };
}
