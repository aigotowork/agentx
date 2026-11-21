/** @aiPurpose Coordinate fenced Agent reads and durable native operation admission. */
import { useCallback, useEffect, useRef, useState } from 'react';
import type { AgentPage, AgentRef, AgentView, Descriptor, MessageInput, MessagePage, MessageView, Operation, LaunchInput, TranscriptPage, SubmitDirectoryMessage, SubmitLabels } from './api/native';
import { useOffices } from './use-offices';
import { foregroundPoll } from './foreground-poll';
import { ApiError, management, storageSuffix, submitManagement } from './api/client';
import { activeOperation, loadReceipts, messageOf, readStored, refKey, requestId, saveReceipts, type Receipt, type WriteBody, type WriteKind } from './model';

const selectionKey = 'agentx.management.selection.v1' + storageSuffix;
type StoredSelection = { agent: AgentRef; generation: string } | AgentRef | null;

/** Own scoped read polling and durable operation admission independently of visible panels. */
export function useManagement() {
  const [descriptor, setDescriptor] = useState<Descriptor>();
  const descriptorRef = useRef<Descriptor | undefined>(undefined);
  const [agents, setAgents] = useState<AgentView[]>([]);
  const [pages, setPages] = useState(1);
  const [labelSelectors, setLabelSelectors] = useState<string[]>([]);
  const applyLabelSelectors = useCallback((values: string[]) => { setLabelSelectors(values); setPages(1); setAgents([]); setHasMore(false); setLoading(true); }, []);
  const [hasMore, setHasMore] = useState(false);
  const [savedSelection] = useState(() => readStored<StoredSelection>(selectionKey, null));
  const [selected, setSelectedState] = useState<AgentRef | null>(null);
  const selectedRef = useRef<AgentRef | null>(null);
  const [detail, setDetail] = useState<AgentView>();
  const [messages, setMessages] = useState<MessageView[]>([]);
  const [messageMore, setMessageMore] = useState(false);
  const [messagesReady, setMessagesReady] = useState(false);
  const cursor = useRef<string | undefined>(undefined);
  const [receipts, setReceipts] = useState<Receipt[]>(() => loadReceipts(storageSuffix));
  const receiptsRef = useRef(receipts);
  const [error, setError] = useState('');
  const [detailError, setDetailError] = useState('');
  const [messageError, setMessageError] = useState('');
  const [operationError, setOperationError] = useState('');
  const [loading, setLoading] = useState(true);
  const [refreshVersion, setRefreshVersion] = useState(0);
  const offices = useOffices(descriptor?.generation, refreshVersion);
  const inFlight = useRef(new Set<string>());
  const refresh = useCallback(() => setRefreshVersion(value => value + 1), []);
  const selectedKey = selected ? refKey(selected) : '';

  function updateReceipts(update: (current: Receipt[]) => Receipt[], required = false) {
    const next = update(receiptsRef.current);
    try { saveReceipts(next, storageSuffix); } catch (error) {
      if (required) throw new Error(`无法保存恢复凭据，操作尚未发送：${messageOf(error)}`);
      setOperationError('浏览器无法保存最新回执；请保留此页面。');
    }
    receiptsRef.current = next;
    setReceipts(next);
  }

  const select = useCallback((agent: AgentRef | null) => {
    if ((agent && refKey(agent)) !== (selectedRef.current && refKey(selectedRef.current))) {
      selectedRef.current = agent;
      setDetail(undefined); setDetailError(''); setMessages([]); setMessageError(''); setMessageMore(false); setMessagesReady(false);
    }
    setSelectedState(agent);
    try { localStorage.setItem(selectionKey, JSON.stringify(agent && { agent, generation: descriptorRef.current?.generation })); } catch { /* Selection persistence is optional. */ }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    async function poll() {
      try {
        const next = await management.describe({ signal: controller.signal });
        if (controller.signal.aborted) return;
        const first = !descriptorRef.current;
        const changed = !first && descriptorRef.current?.generation !== next.generation;
        descriptorRef.current = next;
        setDescriptor(next);
        if (changed) { select(null); setAgents([]); setError('数据范围已重置；请选择当前 Agent。旧操作保留原代次。'); }
        if (first && savedSelection && 'agent' in savedSelection && savedSelection.generation === next.generation) select(savedSelection.agent);
        if (labelSelectors.length && !next.supports_labels) throw new Error('当前服务不支持标签筛选。请清除条件。');
        let after: string | undefined;
        const rows: AgentView[] = [];
        for (let index = 0; index < pages; index++) {
          const page = await management.agents.list({ generation: next.generation, limit: next.max_page, after, current_only: labelSelectors.length ? true : undefined, label: labelSelectors }, { signal: controller.signal });
          if (page.generation !== next.generation) throw new Error('数据代次已变化，请刷新。');
          rows.push(...page.agents);
          after = page.next ?? undefined;
          if (!after) break;
        }
        if (!controller.signal.aborted) {
          // A legacy selection had no dataset metadata. Restore only if the
          // first read confirms its full reference within this listener.
          if (first && savedSelection && !('agent' in savedSelection)) {
            const match = rows.find(row => refKey(row.agent) === refKey(savedSelection));
            if (match) select(match.agent);
          }
          setAgents(rows); setHasMore(!!after); if (!changed) setError('');
        }
      } catch (error) { if (!controller.signal.aborted) setError(messageOf(error)); }
      finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }
    foregroundPoll(poll, 2000, controller.signal);
    return () => controller.abort();
  }, [pages, refreshVersion, select, labelSelectors, savedSelection]);

  useEffect(() => {
    cursor.current = undefined;
    if (!selected || !descriptor) return;
    const controller = new AbortController();
    const generation = descriptor.generation;
    const currentTarget = () => !controller.signal.aborted && descriptorRef.current?.generation === generation && !!selectedRef.current && refKey(selectedRef.current) === selectedKey;
    async function poll() {
      try {
        const current = await management.agents.get({ generation, agent: selected! }, { signal: controller.signal });
        if (refKey(current.agent) !== selectedKey) throw new Error('服务返回的执行实例已变化；请重新选择。');
        if (currentTarget()) { setDetail(current); setDetailError(''); }
      } catch (error) { if (currentTarget()) { setDetail(undefined); setDetailError(messageOf(error)); } }
      try {
        const page = await management.agents.messages.list({ generation, agent: selected!, limit: descriptor!.max_page, after: cursor.current }, { signal: controller.signal });
        if (page.generation !== generation) throw new Error('消息代次已变化，请刷新。');
        if (currentTarget()) {
          cursor.current = page.next_after;
          setMessageMore(page.has_more);
          setMessagesReady(true);
          setMessages(previous => { const merged = new Map(previous.map(item => [item.id, item])); page.messages.forEach(item => merged.set(item.id, item)); return [...merged.values()]; });
          setMessageError('');
        }
      } catch (error) { if (currentTarget()) setMessageError(messageOf(error)); }
    }
    foregroundPoll(poll, 2000, controller.signal);
    return () => controller.abort();
  }, [selectedKey, descriptor?.generation]);

  async function lookup(receipt: Receipt, signal?: AbortSignal) {
    if (receipt.rejected || receipt.body.generation !== descriptorRef.current?.generation || inFlight.current.has(receipt.id)) return;
    inFlight.current.add(receipt.id);
    try {
      const operation = await management.getOperation({ generation: receipt.body.generation, request_id: receipt.id }, { signal });
      if (signal?.aborted) return;
      updateReceipts(all => all.map(item => item.id === receipt.id ? { ...item, operation, issue: undefined, missing: false } : item));
    } catch (error) {
      if (!signal?.aborted) updateReceipts(all => all.map(item => item.id === receipt.id ? { ...item, issue: messageOf(error), missing: error instanceof ApiError && error.status === 404 } : item));
    } finally { inFlight.current.delete(receipt.id); }
  }

  useEffect(() => {
    const controller = new AbortController();
    async function poll() {
      const pending = receiptsRef.current.filter(activeOperation);
      // Bound concurrent read pressure without overlapping ticks.
      for (let offset = 0; offset < pending.length && !controller.signal.aborted; offset += 4) {
        await Promise.allSettled(pending.slice(offset, offset + 4).map(receipt => lookup(receipt, controller.signal)));
      }
    }
    foregroundPoll(poll, 2000, controller.signal);
    return () => controller.abort();
  }, []);

  async function post(receipt: Receipt) {
    if (inFlight.current.has(receipt.id)) return;
    inFlight.current.add(receipt.id);
    try {
      const operation = await submitManagement(receipt.kind, receipt.body);
      updateReceipts(all => all.map(item => item.id === receipt.id ? { ...item, operation, issue: undefined, missing: false } : item));
      refresh();
    } catch (error) {
      // Native label admission rejects these inputs before committing the patch.
      const rejected = receipt.kind === 'labels' && error instanceof ApiError && ['InvalidInput', 'Forbidden', 'NotFound', 'Conflict', 'StaleReference', 'Unsupported'].includes(error.code);
      updateReceipts(all => all.map(item => item.id === receipt.id ? { ...item, rejected, missing: false, issue: rejected ? `请求已拒绝：${messageOf(error)}` : `回执未确认：${messageOf(error)}。先查询原操作。` } : item));
    } finally { inFlight.current.delete(receipt.id); }
  }

  async function submit(kind: WriteKind, input?: LaunchInput | MessageInput | Pick<SubmitDirectoryMessage, 'directory' | 'message'> | Pick<SubmitLabels, 'set' | 'remove' | 'if_revision'>, onJournaled?: (id: string) => void): Promise<string> {
    if (!descriptorRef.current) throw new Error('尚未连接管理服务。');
    const generation = descriptorRef.current.generation;
    const id = requestId();
    let body: WriteBody;
    let path: string;
    if (kind === 'launch') { body = { request_id: id, generation, launch: input as LaunchInput }; path = '/api/v1/agents'; }
    else if (kind === 'directory-send') {
      const directoryInput = input as Pick<SubmitDirectoryMessage, 'directory' | 'message'>;
      body = { request_id: id, generation, directory: directoryInput.directory, message: directoryInput.message }; path = '/api/v1/directory-messages';
    }
    else {
      if (!selected || !selectedRef.current || refKey(selectedRef.current) !== refKey(selected)) throw new Error('执行实例已变化；请重新选择。');
      const base = { request_id: id, generation, agent: selected };
      body = kind === 'send' ? { ...base, message: input as MessageInput } : kind === 'labels' ? { ...base, ...input as Pick<SubmitLabels, 'set' | 'remove' | 'if_revision'> } : base;
      path = `/api/v1/agents/${encodeURIComponent(selected.name)}/${kind === 'send' ? 'messages' : kind}`;
    }
    const receipt: Receipt = { id, path, body, kind, label: kind === 'launch' ? '新 Agent' : kind === 'directory-send' && 'directory' in body ? body.directory : selected!.name, at: new Date().toISOString() };
    updateReceipts(all => [receipt, ...all], true);
    onJournaled?.(id);
    await post(receipt);
    return id;
  }

  async function replay(receipt: Receipt) {
    if (receipt.rejected || !receipt.missing || receipt.operation || receipt.body.generation !== descriptorRef.current?.generation) return;
    // Replay is explicit, after a missing lookup; keep the exact original body and ID.
    updateReceipts(all => all.map(item => item.id === receipt.id ? { ...item, missing: false, issue: undefined } : item), true);
    await post(receipt);
  }

  function dismiss(id: string) { updateReceipts(all => all.filter(item => item.id !== id || activeOperation(item) || item.operation?.status === 'Unknown')); }
  const transcriptReceipt = receipts.find(receipt => receipt.kind === 'transcript' && 'agent' in receipt.body && refKey(receipt.body.agent) === selectedKey && receipt.body.generation === descriptor?.generation);
  const transcript: TranscriptPage | undefined = transcriptReceipt?.operation?.result && 'Transcript' in transcriptReceipt.operation.result ? transcriptReceipt.operation.result.Transcript : undefined;
  const targetPending = receipts.some(receipt => activeOperation(receipt) && 'agent' in receipt.body && refKey(receipt.body.agent) === selectedKey && receipt.body.generation === descriptor?.generation);
  return { descriptor, storageSuffix, labelSelectors, applyLabelSelectors, offices, agents, detail, selected, select, messages, messageMore, messageError, messagesReady, receipts, error, detailError, operationError, loading, hasMore, loadMore: () => setPages(value => value + 1), refresh, submit, lookup, replay, dismiss, transcript, transcriptReceipt, targetPending };
}
export type Management = ReturnType<typeof useManagement>;
