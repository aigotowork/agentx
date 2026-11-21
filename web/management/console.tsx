import { useEffect, useRef, useState } from 'react';
import { useManagement, type Management } from './use-management';
import { AgentResources, AgentToolbar } from './resources';
import { MessageWorkspace, TranscriptWorkspace } from './conversation';
import { OfficeWorkspace, type RoleLaunch } from './offices';
import { LaunchDialog } from './launch';
import { OperationsWorkspace, ReceiptNotice } from './operations';
import { needsAttention, refKey } from './model';
import { ConsoleNavigation, destinationAtLocation, receiptAtLocation, type Destination } from './navigation';
import { EmptyState, Icon } from './presentation';
import type { AgentRef } from './api/native';

type DetailTab = 'messages' | 'transcript';
type ReceiptActions = { onOperation: (id: string) => void; onOpenOperation: (id: string) => void };

function AgentsWorkspace({ model, ...actions }: { model: Management } & ReceiptActions) {
  const [tab, setTab] = useState<DetailTab>('messages');
  const detailHeading = useRef<HTMLElement>(null);
  const workspaceKey = `${model.descriptor?.generation}:${model.selected ? refKey(model.selected) : 'none'}`;
  const pick = (agent: AgentRef) => {
    model.select(agent);
    if (window.matchMedia('(max-width: 700px)').matches) requestAnimationFrame(() => detailHeading.current?.focus());
  };
  return <div className="workspace agents-workspace">
    <AgentResources model={model} onPick={pick} />
    <section className="agent-detail" ref={detailHeading} tabIndex={-1} aria-label="选中的 Agent">
      {!model.selected ? <EmptyState icon="agents" title="选择一个 Agent">从列表进入消息、模型记录和执行详情。</EmptyState> : <>
        <AgentToolbar key={workspaceKey} model={model} {...actions} />
        <nav className="detail-tabs" aria-label="Agent 内容">
          <button aria-label="消息" aria-pressed={tab === 'messages'} onClick={() => setTab('messages')}><Icon name="message" />消息{model.messages.length > 0 && <span className="tab-count" title="已加载消息">{model.messages.length}</span>}</button>
          <button aria-pressed={tab === 'transcript'} onClick={() => setTab('transcript')}><Icon name="transcript" />模型记录</button>
        </nav>
        <div className="detail-content">
          <div className="tab-panel" hidden={tab !== 'messages'}><MessageWorkspace key={workspaceKey} model={model} {...actions} /></div>
          <div className="tab-panel" hidden={tab !== 'transcript'}><TranscriptWorkspace key={workspaceKey} model={model} {...actions} /></div>
        </div>
      </>}
    </section>
  </div>;
}

/**
 * @aiPurpose Compose the real Management entry around Agent, workspace and receipt destinations.
 * @aiUsage Mount once per selected HTTP listener.
 * @aiBehavior Destination changes preserve local work; the operation journal lives above all pages.
 */
export function ManagementConsole() {
  const model = useManagement();
  const [page, setPage] = useState(destinationAtLocation);
  const [returnPage, setReturnPage] = useState<Exclude<Destination, 'operations'>>(() => history.state?.managementReturn === 'workspaces' ? 'workspaces' : 'agents');
  const [launch, setLaunch] = useState<{ role?: RoleLaunch; origin: Destination }>();
  const [notice, setNotice] = useState<Partial<Record<Destination, { id: string; context: string }>>>({});
  const [receiptId, setReceiptId] = useState(receiptAtLocation);
  const destination = useRef<HTMLElement>(null);
  const focusDestination = useRef(false);
  const pending = model.receipts.filter(needsAttention).length;
  const context = (view: Destination) => view === 'agents' ? `agents:${model.selected ? refKey(model.selected) : ''}` : view;
  const currentNotice = notice[page];
  const localReceipt = currentNotice?.context === context(page) ? model.receipts.find(item => item.id === currentNotice.id) : undefined;

  useEffect(() => {
    function fromHistory() {
      focusDestination.current = true;
      setPage(destinationAtLocation()); setReceiptId(receiptAtLocation());
      setReturnPage(history.state?.managementReturn === 'workspaces' ? 'workspaces' : 'agents');
    }
    window.addEventListener('popstate', fromHistory);
    window.addEventListener('hashchange', fromHistory);
    return () => { window.removeEventListener('popstate', fromHistory); window.removeEventListener('hashchange', fromHistory); };
  }, []);
  useEffect(() => {
    if (focusDestination.current) { destination.current?.focus(); focusDestination.current = false; }
  }, [page]);

  function navigate(next: Destination, id?: string) {
    if (next === page) return;
    if (next === 'operations' && page !== 'operations') setReturnPage(page);
    const hash = id && next === 'operations' ? `#operations?receipt=${encodeURIComponent(id)}` : `#${next}`;
    history.pushState(next === 'operations' ? { managementReturn: page } : null, '', hash);
    setReceiptId(next === 'operations' ? id : undefined);
    focusDestination.current = true;
    setPage(next);
  }
  function chooseReceipt(id?: string) {
    setReceiptId(id);
    history.replaceState(history.state, '', id ? `#operations?receipt=${encodeURIComponent(id)}` : '#operations');
  }
  function openOperation(id: string) { if (page === 'operations') chooseReceipt(id); else navigate('operations', id); }
  function showReceipt(id: string, origin: Destination) {
    if (origin === 'operations') chooseReceipt(id);
    else setNotice(previous => ({ ...previous, [origin]: { id, context: context(origin) } }));
  }
  function openMember(agent: AgentRef) { model.select(agent); navigate('agents'); }
  const agentActions = { onOperation: (id: string) => showReceipt(id, 'agents'), onOpenOperation: openOperation };
  const roleLaunch = (role: RoleLaunch) => setLaunch({ role, origin: 'workspaces' });

  return <div className="management-app">
    <ConsoleNavigation model={model} page={page} pending={pending} navigate={navigate} onLaunch={() => setLaunch({ origin: page })} />
    {model.error && <div className="connection-error" role="alert"><span>{model.error}</span><button className="icon-button" aria-label="重试读取" onClick={model.refresh}><Icon name="refresh" />重试读取</button></div>}
    <main className="console-main" ref={destination} tabIndex={-1} aria-label={page === 'agents' ? 'Agents' : page === 'workspaces' ? '工作区' : '操作记录'}>
      {localReceipt && <ReceiptNotice receipt={localReceipt} onOpen={openOperation} onClose={() => setNotice(previous => ({ ...previous, [page]: undefined }))} />}
      <div className="destination" hidden={page !== 'agents'}><AgentsWorkspace model={model} {...agentActions} /></div>
      <div className="destination" hidden={page !== 'workspaces'}><OfficeWorkspace model={model} onLaunch={roleLaunch} onMember={openMember} onOperation={id => showReceipt(id, 'workspaces')} onOpenOperation={openOperation} /></div>
      {page === 'operations' && <div className="destination"><OperationsWorkspace model={model} selectedId={receiptId} onSelect={chooseReceipt} onBack={() => navigate(returnPage)} onAgent={openMember} /></div>}
    </main>
    <LaunchDialog model={model} roleLaunch={launch?.role} open={!!launch} onClose={() => setLaunch(undefined)} onOperation={id => showReceipt(id, launch?.origin ?? page)} />
  </div>;
}
