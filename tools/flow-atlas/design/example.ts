/** Typed design example only: an order reservation timeout never proves no stock change.
 * There is no network, handler invocation, business computation or rendering here.
 */
import type { AtlasBundle, AtlasDefinition, Scenario, TraceEvent } from './contracts.js';

export const definition = {
  format: 'flow-atlas/design-1', id: 'shop-design', label: '订单与库存协作',
  sources: [{ id: 'design-source', label: '设计示例', uri: 'example.ts', basis: 'design' }],
  schemas: [
    { id: 'status', label: '状态', jsonSchema: { type: 'string', enum: ['draft', 'waiting', 'confirmed', 'rejected', 'unknown'] } },
    { id: 'count', label: '库存数量', jsonSchema: { type: 'integer', minimum: 0 } },
    { id: 'reserve-input', label: '预留输入', jsonSchema: { type: 'object', properties: { orderId: { type: 'string' }, quantity: { type: 'integer', minimum: 1 } }, required: ['orderId', 'quantity'], additionalProperties: false } },
    { id: 'reservation', label: '预留结果', jsonSchema: { type: 'object', properties: { reservationId: { type: 'string' } }, required: ['reservationId'], additionalProperties: false } },
  ],
  domains: [{ id: 'sales', label: '销售域' }, { id: 'stock', label: '库存域' }],
  modules: [{ id: 'checkout', label: '结账服务', domainId: 'sales' }, { id: 'inventory-api', label: '库存接口', domainId: 'stock' }],
  objectTypes: [
    {
      id: 'order', label: '订单', domainId: 'sales', moduleId: 'checkout', kind: 'entity',
      state: { id: 'order-state', label: '订单状态', fields: [{ id: 'status', label: '状态', schema: { schemaId: 'status' } }] }, ports: [],
    },
    {
      id: 'inventory', label: '库存服务', domainId: 'stock', moduleId: 'inventory-api', kind: 'service',
      state: { id: 'stock-state', label: '库存状态', fields: [{ id: 'available', label: '可用库存', schema: { schemaId: 'count' } }] },
      ports: [{ id: 'reserve', label: '预留库存', kind: 'command', input: { schemaId: 'reserve-input' }, outcomes: [
        { id: 'reserved', label: '预留成功', meaning: 'returned', payload: { schemaId: 'reservation' } },
        { id: 'out-of-stock', label: '库存不足', meaning: 'rejected' },
        { id: 'unconfirmed', label: '结果未确认', meaning: 'unknown' },
      ] }],
    },
  ],
  relationships: [{ id: 'order-uses-stock', label: '订单请求预留', kind: 'uses', fromTypeId: 'order', target: { typeId: 'inventory', portId: 'reserve' } }],
  processes: [{
    id: 'reserve-process', label: '预留库存流程',
    roles: [{ id: 'order-role', label: '当前订单', typeId: 'order' }, { id: 'stock-role', label: '库存服务', typeId: 'inventory' }],
    startId: 'start',
    nodes: [
      { id: 'start', label: '提交订单', kind: 'start' },
      { id: 'reserve-step', label: '请求预留', kind: 'operation', callerRoleId: 'order-role', target: { roleId: 'stock-role', portId: 'reserve' } },
      { id: 'decide', label: '结果分支', kind: 'decision', question: '是否已确认预留？' },
      { id: 'done', label: '已确认', kind: 'end', outcome: 'completed' },
      { id: 'denied', label: '库存不足', kind: 'end', outcome: 'rejected' },
      { id: 'pending', label: '等待查询确认', kind: 'end', outcome: 'unknown' },
    ],
    flows: [
      { id: 'begin', label: '开始预留', from: 'start', to: 'reserve-step' },
      { id: 'evaluate', label: '观察结果', from: 'reserve-step', to: 'decide' },
      { id: 'yes', label: '成功', from: 'decide', to: 'done', condition: 'result == reserved' },
      { id: 'no', label: '拒绝', from: 'decide', to: 'denied', condition: 'result == out-of-stock' },
      { id: 'uncertain', label: '未确认', from: 'decide', to: 'pending', condition: 'result == unconfirmed' },
    ],
  }],
} as const satisfies AtlasDefinition;

const objects = [{ id: 'order-42', label: '订单42', typeId: 'order' }, { id: 'warehouse', label: '库存服务', typeId: 'inventory' }] as const;
const channels = [{ id: 'stock-api', label: '库存RPC', fromObjectId: 'order-42', toObjectId: 'warehouse', transport: 'RPC' }] as const;
const bindings = { 'order-role': 'order-42', 'stock-role': 'warehouse' } as const;
const start = { id: 's0', label: '开始', kind: 'checkpoint', after: [], processNodeId: 'start', note: '新订单', evidence: { kind: 'authored' } } as const satisfies TraceEvent;
const request = {
  id: 'request', label: '发起预留', kind: 'call', callId: 'call-reserve', after: ['s0'], processNodeId: 'reserve-step',
  from: { objectId: 'order-42' }, target: { objectId: 'warehouse', portId: 'reserve' }, channelId: 'stock-api',
  input: { kind: 'known', value: { orderId: '42', quantity: 2 } }, evidence: { kind: 'authored' },
} as const satisfies TraceEvent;
const initial = { objects: { 'order-42': { status: { kind: 'known', value: 'draft' } }, warehouse: { available: { kind: 'known', value: 10 } } } } as const;

export const accepted: Scenario = {
  id: 'reserved', label: '预留成功', definitionId: definition.id, objects, channels, initial,
  process: { processId: 'reserve-process', bindings, visits: [
    { id: 'v0', label: '开始', nodeId: 'start', occurrence: 0, eventIds: ['s0'] },
    { id: 'v1', label: '调用', nodeId: 'reserve-step', occurrence: 0, viaFlowId: 'begin', eventIds: ['request', 'reply', 'stock-state'] },
    { id: 'v2', label: '分支', nodeId: 'decide', occurrence: 0, viaFlowId: 'evaluate', eventIds: ['decision'] },
    { id: 'v3', label: '完成', nodeId: 'done', occurrence: 0, viaFlowId: 'yes', eventIds: ['order-state'] },
  ] },
  events: [start, request,
    { id: 'reply', label: '预留返回', kind: 'reply', callId: 'call-reserve', outcomeId: 'reserved', payload: { kind: 'known', value: { reservationId: 'r-42' } }, after: ['request'], processNodeId: 'reserve-step', evidence: { kind: 'authored' } },
    { id: 'stock-state', label: '更新库存观察', kind: 'state', causeEventId: 'reply', after: ['reply'], processNodeId: 'reserve-step', writes: [{ objectId: 'warehouse', fieldId: 'available', value: { kind: 'known', value: 8 }, nature: 'observation' }], evidence: { kind: 'authored' } },
    { id: 'decision', label: '选择成功分支', kind: 'checkpoint', after: ['stock-state'], processNodeId: 'decide', note: '已有成功返回', evidence: { kind: 'authored' } },
    { id: 'order-state', label: '确认订单', kind: 'state', causeEventId: 'reply', after: ['decision'], processNodeId: 'done', writes: [{ objectId: 'order-42', fieldId: 'status', value: { kind: 'known', value: 'confirmed' }, nature: 'change' }], evidence: { kind: 'authored' } },
  ],
};

export const unknown: Scenario = {
  id: 'timeout', label: '连接断开：结果未知', definitionId: definition.id, objects, channels, initial,
  process: { processId: 'reserve-process', bindings, visits: [
    { id: 'v0', label: '开始', nodeId: 'start', occurrence: 0, eventIds: ['s0'] },
    { id: 'v1', label: '调用', nodeId: 'reserve-step', occurrence: 0, viaFlowId: 'begin', eventIds: ['request', 'disconnect', 'unknown-result'] },
    { id: 'v2', label: '分支', nodeId: 'decide', occurrence: 0, viaFlowId: 'evaluate', eventIds: ['decision'] },
    { id: 'v3', label: '待确认', nodeId: 'pending', occurrence: 0, viaFlowId: 'uncertain', eventIds: ['state-unknown'] },
  ] },
  events: [start, request,
    { id: 'disconnect', label: '连接断开', kind: 'channel', channelId: 'stock-api', state: 'disconnected', reason: '请求已发出但没有收到结果', after: ['request'], processNodeId: 'reserve-step', evidence: { kind: 'authored' } },
    { id: 'unknown-result', label: '标记结果待确认', kind: 'outcome', callId: 'call-reserve', outcomeId: 'unconfirmed', reason: '断线不证明远端未执行', after: ['disconnect'], processNodeId: 'reserve-step', evidence: { kind: 'authored' } },
    { id: 'decision', label: '选择未知分支', kind: 'checkpoint', note: '不能当作库存不足', after: ['unknown-result'], processNodeId: 'decide', evidence: { kind: 'authored' } },
    { id: 'state-unknown', label: '保留不确定性', kind: 'state', causeEventId: 'unknown-result', after: ['decision'], processNodeId: 'pending', writes: [
      { objectId: 'order-42', fieldId: 'status', value: { kind: 'known', value: 'unknown' }, nature: 'observation' },
      { objectId: 'warehouse', fieldId: 'available', value: { kind: 'unknown', reason: '尚无权威返回；不能继续假定为10' }, nature: 'observation' },
    ], evidence: { kind: 'authored' } },
  ],
};

export const bundle: AtlasBundle = { definition, scenarios: [accepted, unknown], layouts: [] };
