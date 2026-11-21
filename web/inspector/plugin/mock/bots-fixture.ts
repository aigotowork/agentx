import { emptyBotsSnapshot, emptyObservation, type BotsSnapshot, type AutomationSnapshot } from '../shared/domain/bots-observation';
const at = '2026-11-13T15:30:00.000Z';
const complete = <T,>(items: T[]) => ({ ...emptyObservation<T>('available'), items, observedAt: at });
/** Synthetic objects deliberately include archived, failed, unknown and unavailable relationships. */
export function botsFixture(): BotsSnapshot {
  const scenario = new URLSearchParams(globalThis.location?.search ?? '').get('botsCase');
  if (scenario === 'unconfigured') return emptyBotsSnapshot(at);
  if (scenario === 'denied') return emptyBotsSnapshot(at, 'forbidden', 'denied');
  const result: BotsSnapshot = { observedAt: at, authorityId: 'fixture-bots-authority', connection: 'available', issue: null,
    bots: complete([
      { id: 'atlas', name: 'Atlas', title: '交付负责人', provider: 'claude', model: 'sonnet', archived: false },
      { id: 'forge', name: 'Forge', title: '实现与测试', provider: 'codex', model: 'gpt-5', archived: false },
      { id: 'iris', name: 'Iris', title: '质量与风险审查', provider: 'claude', model: 'sonnet', archived: false },
      { id: 'scout', name: 'Scout', title: '研究与资料整理', provider: 'codex', model: null, archived: false },
      { id: 'scribe', name: 'Scribe', title: '文档维护', provider: 'claude', model: null, archived: true },
    ]),
    teams: complete([{ id: 'delivery', name: '产品交付', leadId: 'atlas', memberIds: ['atlas', 'forge', 'iris'] }, { id: 'research', name: '研究工作室', leadId: null, memberIds: ['scout'] }]),
    conversations: complete([
      { id: 'c1', botId: 'atlas', teamId: 'delivery', title: '发布计划与依赖梳理', status: 'running', updatedAt: at, attention: false },
      { id: 'c2', botId: 'forge', teamId: 'delivery', title: '实现团队资源目录 Adapter', status: 'running', updatedAt: at, attention: false },
      { id: 'c3', botId: 'forge', teamId: 'delivery', title: '检查契约兼容性', status: 'failed', updatedAt: at, attention: true },
      { id: 'c4', botId: 'iris', teamId: 'delivery', title: '等待发布方案审阅', status: 'idle', updatedAt: at, attention: true },
      { id: 'c5', botId: 'scout', teamId: 'research', title: '调研运行数据的覆盖范围', status: 'unknown', updatedAt: at, attention: null },
      { id: 'c6', botId: 'atlas', teamId: null, title: '独立需求讨论', status: 'succeeded', updatedAt: at, attention: false },
      { id: 'c7', botId: 'missing-bot', teamId: 'former-team', title: '保留的历史归属', status: 'cancelled', updatedAt: at, attention: null },
    ]),
    runtimes: complete([{ id: 'local', serverId: 'fixture-agentx', generation: 'g1', title: '本机 AgentX', mode: 'managed', health: 'ready', partial: false, observedAt: at }, { id: 'remote', serverId: 'fixture-remote', generation: 'g2', title: '远程研究环境', mode: 'attached', health: 'ready', partial: false, observedAt: at }]),
    agents: complete([{ id: 'ordinary-1', runtimeId: 'local', serverId: 'fixture-agentx', generation: 'g1', title: '独立的 Agent 运行', provider: 'codex', model: null, ownership: 'ordinary', state: 'running', attention: false, updatedAt: at }, { id: 'ordinary-1', runtimeId: 'remote', serverId: 'fixture-remote', generation: 'g2', title: '远程执行需要检查', provider: 'claude', model: null, ownership: 'foreign', state: 'failed', attention: true, updatedAt: at }]),
  };
  if (scenario === 'partial') { result.conversations.status = 'partial'; result.conversations.truncated = true; result.teams = emptyObservation('forbidden', 'denied'); result.agents.status = 'partial'; result.agents.gap = true; }
  if (scenario === 'empty') { result.bots.items = []; result.teams.items = []; result.conversations.items = []; result.runtimes.items = []; result.agents.items = []; }
  return result;
}
