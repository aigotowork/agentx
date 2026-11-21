import { findConversation, conversationSubjects } from "../offline/model";
/** @aiPurpose Exact conversation navigation and ready/correlated parent context bridge; never sends. */
import type { AgentRef } from '../../../management/api/native';
import type { AgentResource } from '../offline/model';
import type { SpaceController } from '../offline/controller';
import type { LiveSpace } from './controller';
export interface ContextTarget { scope: string; generation: string; agent: AgentRef; }
export interface ContextInput { title?: string; text?: string; url?: string; files?: Blob[]; }
export interface ContextRequest { protocol: 'agentx.context.v1'; type: 'addContext'; requestId: string; target: ContextTarget; selectionRevision?: string; context: ContextInput; }
const protocol = 'agentx.context.v1';
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const failure = (error: unknown) => error instanceof Error ? error.message : String(error);
function targetValue(value: unknown): ContextTarget {
  if (!object(value) || typeof value.scope !== 'string' || !value.scope || value.scope.length > 4096 || typeof value.generation !== 'string' || !value.generation || value.generation.length > 128 || !object(value.agent)) throw new Error('需要完整 scope / generation / Agent 精确引用');
  const agent = value.agent;
  if (typeof agent.name !== 'string' || !agent.name || agent.name.length > 128 || ![agent.incarnation, agent.stopped_event].every(field => field === null || typeof field === 'string' && /^\d{1,24}$/.test(field)) || (agent.incarnation === null) === (agent.stopped_event === null)) throw new Error('Agent 精确引用无效');
  return value as unknown as ContextTarget;
}
function contextValue(value: unknown): ContextInput {
  if (!object(value) || Object.keys(value).some(key => !['title', 'text', 'url', 'files'].includes(key))) throw new Error('上下文格式无效');
  for (const key of ['title', 'text', 'url']) if (value[key] !== undefined && (typeof value[key] !== 'string' || (value[key] as string).length > (key === 'text' ? 65536 : key === 'title' ? 512 : 4096))) throw new Error('上下文字段过长或类型无效');
  if (value.url && !['http:', 'https:'].includes(new URL(value.url as string).protocol)) throw new Error('来源 URL 需要 http / https');
  if (value.files !== undefined && (!Array.isArray(value.files) || value.files.length > 8 || value.files.some(file => !(file instanceof Blob) || file.size > 10 * 1024 * 1024))) throw new Error('附件需要 File / Blob，最多 8 个，每个 10 MiB');
  const context = value as ContextInput;
  if (![context.title, context.text, context.url].some(text => text?.trim()) && !context.files?.length) throw new Error('上下文为空');
  return context;
}
export function exactTarget(controller: SpaceController, target: ContextTarget): AgentResource {
  if (controller.supply.scope.id !== target.scope || controller.supply.scope.generation !== target.generation) throw new Error('目标 scope / generation 已失效');
  const matches = conversationSubjects(controller.supply).filter(agent => agent.name === target.agent.name && agent.ref.incarnation === target.agent.incarnation && (agent.ref.stopped_event ?? null) === target.agent.stopped_event);
  if (matches.length !== 1) throw new Error('精确目标未加载、已失效或不唯一；不会转向同名实例');
  return matches[0];
}
/** @aiPurpose Install the parent context bridge independently of browser navigation.
 * @aiUsage App installs navigation first; cleanup retains exact selection/upload revision fencing.
 */
export function installContext(controller: SpaceController, live: LiveSpace | undefined, report: (issue: string) => void) {
  // getRandomValues also works on the user's plain-HTTP LAN origin.
  const page = Array.from(crypto.getRandomValues(new Uint32Array(4)), value => value.toString(16)).join('-');
  let sequence = 0, previousSelection = '';
  function selected() {
    const state = controller.getSnapshot();
    const agent = findConversation(controller.supply, state.selectedId);
    const channel = agent ? state.channels[agent.id] ?? 'messages' : '';
    const eligible = !!agent && state.reading && ['members', 'history'].includes(state.officeView) && channel === 'messages' && !!live?.capabilities(agent.id)?.can_send && !live.blocked(agent.id);
    const fingerprint = JSON.stringify([controller.supply.scope.id, controller.supply.scope.generation, state.selectedId, state.reading, state.workspaceId, state.officeView, channel, eligible]);
    if (fingerprint !== previousSelection) { previousSelection = fingerprint; sequence++; }
    return { agent, eligible, revision: `${page}:${sequence}` };
  }
  selected();
  const unsubscribe = controller.subscribe(() => { selected(); });
  function selectionTarget() {
    const selection = selected();
    if (!selection.agent || !selection.eligible) throw new Error('请在 Space 中打开可接收消息的当前 Agent 会话');
    const agent = selection.agent;
    const target: ContextTarget = { scope: controller.supply.scope.id, generation: controller.supply.scope.generation, agent: { name: agent.name, incarnation: agent.ref.incarnation, stopped_event: agent.ref.stopped_event ?? null } };
    exactTarget(controller, target);
    return { target, selectionRevision: selection.revision, title: agent.title };
  }
  function selectedTarget() {
    const state = controller.getSnapshot();
    const agent = state.selectedId ? findConversation(controller.supply, state.selectedId) : undefined;
    if (!agent || !state.reading) throw new Error('当前精确会话不可用');
    const target: ContextTarget = { scope: controller.supply.scope.id, generation: controller.supply.scope.generation, agent: { name: agent.name, incarnation: agent.ref.incarnation, stopped_event: agent.ref.stopped_event ?? null } };
    exactTarget(controller, target);
    return { target, selectionRevision: selected().revision, title: agent.title };
  }
  const cache = new Map<string, Promise<Record<string, unknown>>>();
  async function add(value: Record<string, unknown>) {
    const target = targetValue(value.target), context = contextValue(value.context);
    const agent = exactTarget(controller, target);
    const initial = selectionTarget();
    const revision = value.selectionRevision ?? initial.selectionRevision;
    if (typeof revision !== 'string') throw new Error('selectionRevision 无效');
    function guard() {
      const current = selectionTarget();
      if (exactTarget(controller, target).id !== agent.id || selected().agent?.id !== agent.id || current.selectionRevision !== revision) throw new Error('上下文目标选择已改变，请重新确认当前会话');
    }
    guard();
    const text = [context.title, context.text, context.url ? `来源：${context.url}` : ''].filter(Boolean).join('\n\n');
    await controller.addContext(agent.id, text, context.files ?? [], files => live!.upload(agent.id, files), live!.descriptor!.max_text, guard);
    return { target, selectionRevision: revision };
  }
  function receive(event: MessageEvent) {
    if (window.parent === window || event.source !== window.parent || !object(event.data) || event.data.protocol !== protocol) return;
    const value = event.data, requestId = value.requestId;
    if (typeof requestId !== 'string' || !requestId || requestId.length > 128) return;
    const reply = (result: Record<string, unknown>) => window.parent.postMessage({ protocol, requestId, ...result }, event.origin === 'null' ? '*' : event.origin);
    if (value.type === 'hello') { reply({ type: 'ready' }); return; }
    if (value.type === 'getTarget') {
      try { reply({ type: 'target', status: 'selected', ...selectedTarget() }); }
      catch (error) { reply({ type: 'target', status: 'unavailable', error: failure(error) }); }
      return;
    }
    if (value.type !== 'addContext') { reply({ type: 'result', status: 'error', error: '不支持的上下文操作' }); return; }
    let result = cache.get(requestId);
    if (!result) {
      if (cache.size >= 128) { reply({ type: 'result', status: 'error', error: '上下文请求缓存已满；请重新打开页面' }); return; }
      result = add(value).then(selection => ({ type: 'result', status: 'added', ...selection }), error => ({ type: 'result', status: 'error', error: failure(error) }));
      cache.set(requestId, result);
    }
    void result.then(reply);
  }
  window.addEventListener('message', receive);
  if (window.parent !== window) window.parent.postMessage({ protocol, type: 'ready' }, '*');
  return () => { unsubscribe(); window.removeEventListener('message', receive); };
}
