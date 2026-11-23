import { CloudAdminError, type CloudScopeTypes } from '@agentx/client';
import type { MessageReadPage } from '../live/domain-office';
import type { HistoryRecord, SuppliedRecords } from '../offline/model';
import { cloudProblem, type CloudAgentFacts, type Session } from './model';
import { exactLabel, sameAgent, type CloudAgent } from './route';

type Page = CloudScopeTypes['TranscriptHistoryPage'];
type Lane = { phase: 'ready' | 'loading' | 'error'; error?: string };
const pageSize = 20;
const retained = 256;
const maxPages = Math.ceil(retained / pageSize);
const validCursor = (value: string) => /^[1-9][0-9]{0,18}$/.test(value);
const before = (a: string, b: string) => a.length < b.length || (a.length === b.length && a < b);
const provenance = '云端 Codex 可见记录';

async function page(session: Session, agent: CloudAgent, signal: AbortSignal, cursors?: { before?: string; after?: string }): Promise<Page> {
  const { host, name, incarnation } = agent;
  const query = { generation: session.generation, host, name, incarnation, limit: pageSize, ...cursors };
  const result = await (session.access === 'admin' ? session.client.listTranscript(session.scope, query, { signal })
    : session.client.listTranscript(query, { signal }));
  signal.throwIfAborted();
  if (!sameAgent(result.agent, agent) || result.latest !== null && !validCursor(result.latest))
    throw new Error('模型记录实例或游标不匹配');
  let previous = cursors?.before ?? cursors?.after;
  for (const item of result.items) {
    if (!validCursor(item.id) || previous !== undefined && !(cursors?.after !== undefined ? before(previous, item.id) : before(item.id, previous))
      || result.latest === null || before(result.latest, item.id)) throw new Error('模型记录顺序不匹配');
    previous = item.id;
  }
  if (result.next !== null && (!result.items.length || result.next !== previous)) throw new Error('模型记录游标未前进');
  return result;
}

function record(agent: CloudAgent, row: Page['items'][number]): HistoryRecord {
  return { id: row.id, kind: 'message', role: row.role, text: row.text, at: '', author: row.role === 'assistant' ? exactLabel(agent) : '用户' };
}

/** One exact, bounded model reader: the initial tail, older pages and notification-driven additions share a scroll anchor. */
export class CloudTranscriptReader {
  private phase: Lane = { phase: 'loading' };
  private older: Lane = { phase: 'ready' };
  private live: Lane = { phase: 'ready' };
  private availability?: boolean;
  private legacy = false;
  private next: string | null = null;
  private head = '0';
  private arrivals = 0;
  private readonly token = {};
  private items: readonly HistoryRecord[] = [];
  private latestRead?: Promise<void>;
  private queued?: AbortSignal;
  facts: CloudAgentFacts | null = null;

  constructor(private agent: CloudAgent, private session: Session, private changed: () => void, private failed: (error: unknown) => void) {}

  get records(): SuppliedRecords {
    if (this.availability) return { status: 'available', provenance, items: this.items };
    if (this.legacy) return { status: 'unavailable', reason: '此云端服务尚未接入模型记录。' };
    if (this.phase.phase === 'error') return { status: 'error', reason: this.phase.error ?? '无法读取模型记录。' };
    if (this.availability === false) return { status: 'unavailable', reason: '此实例尚未上报模型记录。' };
    return { status: 'loading' };
  }

  get page(): MessageReadPage {
    return { ...this.phase, mode: 'latest', more: this.next !== null, token: this.availability ? this.token : undefined,
      arrivals: this.arrivals, older: this.older, live: this.live };
  }

  refresh(signal: AbortSignal): Promise<void> {
    if (signal.aborted || this.legacy) return Promise.resolve();
    if (this.latestRead) {
      this.queued = signal;
      return this.latestRead;
    }
    const run = async () => {
      let request: AbortSignal | undefined = signal;
      while (request) {
        if (!request.aborted) await this.readLatest(request);
        request = this.queued;
        this.queued = undefined;
      }
    };
    this.latestRead = run().finally(() => { this.latestRead = undefined; });
    return this.latestRead;
  }

  private async readLatest(signal: AbortSignal) {
    const initial = this.availability === undefined;
    if (initial && this.phase.phase !== 'loading') {
      this.phase = { phase: 'loading' }; this.changed();
    }
    try {
      let result = await page(this.session, this.agent, signal, initial ? undefined : { after: this.head });
      this.acceptFacts(result);
      if (initial) {
        this.next = result.next;
        this.head = result.latest ?? '0';
        this.items = result.items.slice().reverse().map(row => record(this.agent, row));
      } else {
        // Use the last row consumed as the watermark. The snapshot head may be ahead of a partial page.
        for (let reads = 0; reads < maxPages; reads++) {
          this.acceptFacts(result);
          const append = result.items.map(row => record(this.agent, row));
          if (append.length) {
            const combined = [...this.items, ...append];
            if (combined.length > retained) this.next = null;
            this.items = combined.slice(-retained);
            this.arrivals += append.length;
            this.head = append.at(-1)!.id;
          }
          if (!result.next || reads + 1 === maxPages) break;
          result = await page(this.session, this.agent, signal, { after: this.head });
        }
      }
      this.phase = this.live = { phase: 'ready' };
    } catch (error) {
      if (signal.aborted) return;
      this.failed(error);
      if (error instanceof CloudAdminError && error.status === 404) {
        this.legacy = true;
        this.phase = { phase: 'ready' };
      } else {
        const failure: Lane = { phase: 'error', error: cloudProblem(error, '无法读取模型记录。') };
        if (this.availability === undefined) this.phase = failure;
        else this.live = failure;
      }
    } finally {
      if (!signal.aborted) this.changed();
    }
  }

  async readOlder(signal: AbortSignal) {
    if (signal.aborted) return;
    if (this.phase.phase === 'error') return this.refresh(signal);
    const cursor = this.next;
    if (!cursor || this.older.phase === 'loading') return;
    this.older = { phase: 'loading' }; this.changed();
    try {
      const result = await page(this.session, this.agent, signal, { before: cursor });
      // A concurrent live read may already have reached the local retention ceiling.
      if (this.next !== cursor) { this.older = { phase: 'ready' }; return; }
      const room = Math.max(0, retained - this.items.length);
      this.items = [...result.items.slice(0, room).reverse().map(row => record(this.agent, row)), ...this.items];
      this.next = this.items.length >= retained ? null : result.next;
      this.older = { phase: 'ready' };
    } catch (error) {
      if (signal.aborted) { this.older = { phase: 'ready' }; return; }
      this.failed(error);
      this.older = { phase: 'error', error: cloudProblem(error, '无法读取更早的模型记录。') };
    } finally {
      if (!signal.aborted) this.changed();
    }
  }

  private acceptFacts(result: Page) {
    this.availability = result.available;
    this.facts = result.facts;
  }
}
