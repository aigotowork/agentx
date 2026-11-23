import type { CloudScopeTypes } from '@agentx/client';
import type { MessageReadPage } from '../live/domain-office';
import type { HistoryRecord, SuppliedRecords } from '../offline/model';
import { cloudProblem, cloudTime, type Session } from './model';
import { exactLabel, sameAgent, type CloudAgent } from './route';

export type CloudMessage = CloudScopeTypes['MessageView'];
type Page = CloudScopeTypes['MessageHistoryPage'];
type Lane = { phase: 'ready' | 'loading' | 'error'; error?: string };
const pageSize = 20;
const olderId = (a: string, b: string) => a.length < b.length || (a.length === b.length && a < b);
const validId = (id: string) => /^[1-9][0-9]{0,19}$/.test(id);

export async function messagePage(session: Session, agent: CloudAgent, signal: AbortSignal, before?: string, limit = pageSize, after?: string): Promise<Page> {
  const { host, name, incarnation } = agent;
  const query = { generation: session.generation, host, name, incarnation, before, after, limit };
  const result = await (session.access === 'admin'
    ? session.client.listMessages(session.scope, query, { signal })
    : session.client.listMessages(query, { signal }));
  signal.throwIfAborted();
  let previous = before ?? after;
  for (const message of result.items) {
    if (!validId(message.id) || previous && !(after ? olderId(previous, message.id) : olderId(message.id, previous))
      || !sameAgent(agent, message.from) && !sameAgent(agent, message.to)) throw new Error('云端消息顺序或实例引用不匹配');
    previous = message.id;
  }
  if (result.next !== null && (!result.items.length || result.next !== previous)) throw new Error('更早消息游标未前进');
  return result;
}
function record(agent: CloudAgent, message: CloudMessage): HistoryRecord {
  return {
    id: message.id, kind: 'message', role: sameAgent(agent, message.from) ? 'assistant' : 'user',
    text: message.body, at: cloudTime(message.created_at)?.toISOString() ?? '',
    author: exactLabel(message.from), recipients: [exactLabel(message.to)],
    thread: message.thread ?? undefined, replyTo: message.reply_to ?? undefined, intent: message.intent, native: true,
  };
}

/** Keep a stable scroll token and separate live watermark and older cursor for this exact subject. */
export class CloudHistory {
  private older: Lane = { phase: 'ready' };
  private live: Lane = { phase: 'ready' };
  private phase: Lane = { phase: 'loading' };
  private latestRead?: { signal: AbortSignal; done: Promise<void> };
  private next: string | null = null;
  private head?: string;
  private seeded = false;
  private arrivals = 0;
  private readonly token = {};
  private readonly messages = new Map<string, HistoryRecord>();
  private sorted: HistoryRecord[] = [];
  constructor(readonly agent: CloudAgent, private session: Session, private changed: () => void, private failed: (error: unknown) => void) {}

  incremental = false;
  get liveFailed() { return this.live.phase === "error"; }
  get page(): MessageReadPage {
    return { ...this.phase, mode: 'latest', more: this.next !== null, token: this.seeded ? this.token : undefined,
      arrivals: this.arrivals, older: this.older, live: this.live };
  }
  get records(): SuppliedRecords {
    if (this.seeded) return { status: 'available', provenance: '云端 Scope 已持久化的 AgentX 消息', items: this.sorted };
    return this.phase.phase === 'error' ? { status: 'error', reason: this.phase.error ?? '读取失败' } : { status: 'loading' };
  }
  private merge(items: readonly CloudMessage[]) {
    let added = 0;
    for (const item of items) {
      if (this.messages.has(item.id)) continue;
      this.messages.set(item.id, record(this.agent, item));
      added++;
    }
    if (added) this.sorted = [...this.messages.values()].sort((a, b) => olderId(a.id, b.id) ? -1 : 1);
    return added;
  }
  accept(message: CloudMessage) {
    this.arrivals += this.merge([message]);
  }
  seed(result: Page) {
    this.next = result.next;
    this.head = result.items[0]?.id;
    this.merge(result.items);
    this.seeded = true;
    this.phase = { phase: 'ready' };
  }
  async update(signal: AbortSignal, mode: 'latest' | 'older') {
    if (signal.aborted) return;
    if (mode === 'older' && this.seeded) return this.read(signal, mode);
    const pending = this.latestRead;
    if (pending) {
      await pending.done;
      if (pending.signal.aborted) await this.update(signal, 'latest');
      return;
    }
    const done = Promise.resolve().then(() => this.read(signal, 'latest'));
    this.latestRead = { signal, done };
    try { await done; }
    finally { this.latestRead = undefined; }
  }
  private async read(signal: AbortSignal, mode: 'latest' | 'older') {
    if (signal.aborted) return;
    if (!this.seeded) {
      this.phase = { phase: 'loading' };
      this.changed();
      try { this.seed(await messagePage(this.session, this.agent, signal)); }
      catch (error) {
        if (!signal.aborted) { this.failed(error); this.phase = { phase: 'error', error: cloudProblem(error, '无法读取这条精确会话。') }; }
      } finally { if (!signal.aborted) this.changed(); }
      return;
    }
    const lane = mode === 'older' ? this.older : this.live;
    if (lane.phase === 'loading' || mode === 'older' && this.next === null) return;
    let changed = mode === 'older' || lane.phase === 'error';
    lane.phase = 'loading';
    if (mode === 'older') this.changed();
    try {
      if (mode === 'older') {
        const result = await messagePage(this.session, this.agent, signal, this.next!);
        this.next = result.next;
        this.merge(result.items);
      } else if (this.incremental) {
        let cursor = this.head ?? '0';
        do {
          const result = await messagePage(this.session, this.agent, signal, undefined, pageSize, cursor);
          const added = this.merge(result.items);
          this.head = result.items.at(-1)?.id ?? this.head;
          this.arrivals += added;
          changed ||= added > 0;
          if (!result.next) break;
          cursor = result.next;
        } while (!signal.aborted);
      } else {
        const head = this.head;
        // The legacy API only pages backwards. Probe one row and read more only to bridge a changed head.
        const first = await messagePage(this.session, this.agent, signal, undefined, head ? 1 : pageSize);
        let result = first;
        const unseen: CloudMessage[] = [];
        // A busy channel can add more than one page between reads. Advance only after the gap is filled.
        while (true) {
          unseen.push(...result.items.filter(item => !head || olderId(head, item.id)));
          if (!head || !result.next || !olderId(head, result.next)) break;
          result = await messagePage(this.session, this.agent, signal, result.next);
        }
        if (!head) this.next = first.next;
        this.head = first.items[0]?.id ?? head;
        const added = this.merge(unseen);
        this.arrivals += added;
        changed ||= added > 0;
      }
      lane.phase = 'ready'; lane.error = undefined;
    } catch (error) {
      if (signal.aborted) { lane.phase = 'ready'; return; }
      this.failed(error);
      lane.phase = 'error'; lane.error = cloudProblem(error, '无法更新消息。');
      changed = true;
    } finally { if (!signal.aborted && changed) this.changed(); }
  }
}
