/** The tool can describe its own drag interaction using the same public model.
 * Authored target behavior only; it is not a log of an implemented drag feature.
 */
import type { AtlasBundle, AtlasDefinition, Scenario } from './contracts.js';

export const selfDefinition: AtlasDefinition = {
  format: 'flow-atlas/design-1', id: 'atlas-self', label: 'Flow Atlas：拖动职责',
  sources: [{ id: 'design', label: '目标交互契约', uri: 'README.md', basis: 'design' }],
  schemas: [
    { id: 'empty', label: '无参数', jsonSchema: { type: 'object', properties: {}, additionalProperties: false } },
    { id: 'point', label: '移动位置', jsonSchema: { type: 'object', properties: { objectId: { type: 'string' }, x: { type: 'number' }, y: { type: 'number' } }, required: ['objectId', 'x', 'y'], additionalProperties: false } },
    { id: 'play-state', label: '播放状态', jsonSchema: { type: 'string', enum: ['playing', 'paused'] } },
    { id: 'coordinate', label: '坐标', jsonSchema: { type: 'number' } },
  ],
  domains: [{ id: 'presentation', label: '可视化工作区' }],
  modules: [{ id: 'player', label: '演示时钟', domainId: 'presentation' }, { id: 'layout', label: '用户布局', domainId: 'presentation' }],
  objectTypes: [
    { id: 'viewer', label: 'Viewer', kind: 'actor', domainId: 'presentation', state: { id: 'viewer-state', label: '观察者', fields: [] }, ports: [] },
    { id: 'player', label: 'Player', kind: 'service', domainId: 'presentation', moduleId: 'player',
      state: { id: 'player-state', label: '时钟状态', fields: [{ id: 'status', label: '播放', schema: { schemaId: 'play-state' } }] },
      ports: [{ id: 'pause', label: '暂停', kind: 'command', input: { schemaId: 'empty' }, outcomes: [{ id: 'paused', label: '已暂停', meaning: 'returned' }] }],
    },
    { id: 'layout', label: 'LayoutSession', kind: 'service', domainId: 'presentation', moduleId: 'layout',
      state: { id: 'layout-state', label: '节点布局', fields: [{ id: 'x', label: '节点X坐标', schema: { schemaId: 'coordinate' } }] },
      ports: [{ id: 'move', label: '移动节点', kind: 'command', input: { schemaId: 'point' }, outcomes: [{ id: 'moved', label: '已提交布局', meaning: 'returned' }] }],
    },
  ],
  relationships: [
    { id: 'viewer-pauses', label: '拖动先暂停', kind: 'uses', fromTypeId: 'viewer', target: { typeId: 'player', portId: 'pause' } },
    { id: 'viewer-moves', label: '移动只改布局', kind: 'uses', fromTypeId: 'viewer', target: { typeId: 'layout', portId: 'move' } },
  ], processes: [],
};

export const selfScenario: Scenario = {
  id: 'drag', label: '播放中移动节点', definitionId: 'atlas-self',
  objects: [{ id: 'workspace', label: '画板', typeId: 'viewer' }, { id: 'clock', label: '唯一时钟', typeId: 'player' }, { id: 'positions', label: '布局会话', typeId: 'layout' }], channels: [],
  initial: { objects: { workspace: {}, clock: { status: { kind: 'known', value: 'playing' } }, positions: { x: { kind: 'known', value: 10 } } } },
  events: [
    { id: 'pause-call', label: '开始拖动', kind: 'call', callId: 'pause-1', from: { objectId: 'workspace' }, target: { objectId: 'clock', portId: 'pause' }, input: { kind: 'known', value: {} }, after: [], evidence: { kind: 'authored' } },
    { id: 'pause-reply', label: '时钟已停', kind: 'reply', callId: 'pause-1', outcomeId: 'paused', after: ['pause-call'], evidence: { kind: 'authored' } },
    { id: 'pause-state', label: '保留暂停状态', kind: 'state', causeEventId: 'pause-reply', writes: [{ objectId: 'clock', fieldId: 'status', value: { kind: 'known', value: 'paused' }, nature: 'change' }], after: ['pause-reply'], evidence: { kind: 'authored' } },
    { id: 'move-call', label: '提交位置', kind: 'call', callId: 'move-1', from: { objectId: 'workspace' }, target: { objectId: 'positions', portId: 'move' }, input: { kind: 'known', value: { objectId: 'order-42', x: 100, y: 40 } }, after: ['pause-state'], evidence: { kind: 'authored' } },
    { id: 'move-reply', label: '布局已提交', kind: 'reply', callId: 'move-1', outcomeId: 'moved', after: ['move-call'], evidence: { kind: 'authored' } },
    { id: 'move-state', label: '只更新画面坐标', kind: 'state', causeEventId: 'move-reply', writes: [{ objectId: 'positions', fieldId: 'x', value: { kind: 'known', value: 100 }, nature: 'change' }], after: ['move-reply'], evidence: { kind: 'authored' } },
  ],
};

export const selfBundle: AtlasBundle = { definition: selfDefinition, scenarios: [selfScenario], layouts: [] };
