import { useEffect, useId, useRef, useState } from 'react';
import type { LaunchRelationCursor, LaunchRelationsPage, RelatedConversation } from '@agentx/client';
import { Button, useOverlay } from '../../uiux/systems/space/space.web';
import type { AgentResource } from '../offline/model';
import type { SpaceController } from '../offline/controller';
import type { LiveSpace } from '../live/controller';

type RelationProps = { agent: AgentResource; live: LiveSpace; controller: SpaceController; onOpenRelated?: (id: string) => void };
type OpenMode = 'preview' | 'explicit' | false;
const openDelay = 180;
const closeDelay = 160;

/** @aiPurpose Offer a keyboard, pointer and touch preview of native launch relations.
 * @aiUsage Mount in an exact reader header. Only an open panel leases the relation read.
 */
export function RelationDisclosure(props: RelationProps) {
  const id = useId();
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const suppressFocus = useRef(false), hover = useRef(false);
  const [mode, setMode] = useState<OpenMode>(false);
  function cancelTimer() { clearTimeout(timer.current); }
  function dismiss(restoreFocus = false) {
    cancelTimer(); setMode(false);
    if (restoreFocus && root.current?.contains(document.activeElement) && document.activeElement !== trigger.current) {
      suppressFocus.current = true;
      trigger.current?.focus({ preventScroll: true });
      suppressFocus.current = false;
    }
  }
  useOverlay(!!mode, () => dismiss(true), 30, root, false, false);
  useEffect(() => cancelTimer, []);
  useEffect(() => {
    if (!mode) return;
    function outside(event: PointerEvent) {
      if (event.target instanceof Node && !root.current?.contains(event.target)) dismiss();
    }
    document.addEventListener('pointerdown', outside, true);
    return () => document.removeEventListener('pointerdown', outside, true);
  }, [mode]);
  function enter(pointerType: string) {
    if (pointerType !== 'mouse') return;
    hover.current = true; cancelTimer();
    if (!mode) timer.current = setTimeout(() => setMode(current => current || 'preview'), openDelay);
  }
  function leave(pointerType: string) {
    if (pointerType !== 'mouse') return;
    hover.current = false; cancelTimer();
    if (mode === 'preview' && (!root.current?.contains(document.activeElement) || document.activeElement === trigger.current && !trigger.current?.matches(':focus-visible'))) {
      timer.current = setTimeout(() => setMode(current => current === 'preview' ? false : current), closeDelay);
    }
  }
  function openedRelated(target: string) {
    dismiss(true);
    if (props.onOpenRelated) props.onOpenRelated(target);
    else props.controller.openExact(target);
  }
  return <div className="conversation-relation" ref={root}
    onPointerEnter={event => enter(event.pointerType)} onPointerLeave={event => leave(event.pointerType)}
    onBlur={event => { if (!hover.current && !event.currentTarget.contains(event.relatedTarget)) dismiss(); }}>
    <Button ref={trigger} variant="icon" className="conversation-icon conversation-relation-trigger" aria-label="启动关系"
      aria-expanded={!!mode} aria-controls={mode ? id : undefined}
      onFocus={event => { if (!suppressFocus.current && event.currentTarget.matches(':focus-visible')) { cancelTimer(); setMode(current => current || 'preview'); } }}
      onClick={() => { cancelTimer(); setMode(current => current === 'explicit' ? false : 'explicit'); }}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" aria-hidden="true"><path d="M12 7v4M5 17v-4h14v4" /><circle cx="12" cy="5" r="2" /><circle cx="5" cy="19" r="2" /><circle cx="19" cy="19" r="2" /></svg>
      <span>启动关系</span>
    </Button>
    {mode && <div className="workspace-reader-panel conversation-relation-panel" id={id}><LaunchRelations {...props} onOpenRelated={openedRelated} compact /></div>}
  </div>;
}

/** @aiPurpose Show direct launch provenance and fresh native status for one conversation.
 * @aiUsage Mount for one exact live-supply subject; navigation opens native exact references in their actual office.
 * @aiBehavior The optional navigation callback receives only the SDK-resolved exact ID. Compact callers mount only during disclosure.
 */
export function LaunchRelations(props: RelationProps & { compact?: boolean }) {
  return <SubjectRelations key={props.agent.id} {...props} />;
}

function SubjectRelations({ agent, live, controller, onOpenRelated, compact }: Parameters<typeof LaunchRelations>[0]) {
  const [page, setPage] = useState<LaunchRelationsPage>(), [readError, setReadError] = useState(''), [openError, setOpenError] = useState('');
  const [busy, setBusy] = useState(false), [revision, setRevision] = useState(0);
  const [after, setAfter] = useState<LaunchRelationCursor>(), [previous, setPrevious] = useState<(LaunchRelationCursor | undefined)[]>([]);
  const navigation = useRef<AbortController | undefined>(undefined);
  useEffect(() => () => navigation.current?.abort(), []);
  useEffect(() => {
    const request = new AbortController();
    let active = true, timer: ReturnType<typeof setTimeout>;
    async function read() {
      try {
        const result = await live.launchRelations(agent.id, after, request.signal);
        if (active) { setPage(result); setReadError(''); }
      } catch (failure) { if (active) { setPage(undefined); setReadError(failure instanceof Error ? failure.message : String(failure)); } }
      finally { if (active) timer = setTimeout(() => void read(), 3000); }
    }
    void read();
    return () => { active = false; request.abort(); clearTimeout(timer); };
  }, [agent.id, live, after, revision]);
  async function open(target: RelatedConversation) {
    if (navigation.current || target.availability !== 'available') return;
    const request = navigation.current = new AbortController();
    setBusy(true); setOpenError('');
    try {
      const resource = await live.openRelated(target, agent.ref, request.signal);
      if (request.signal.aborted) return;
      if (onOpenRelated) onOpenRelated(resource.id);
      else controller.openExact(resource.id);
    } catch (failure) {
      if (!request.signal.aborted) setOpenError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      if (navigation.current === request) {
        navigation.current = undefined;
        if (!request.signal.aborted) setBusy(false);
      }
    }
  }
  function item(target: RelatedConversation) {
    const { name, incarnation } = target.creation;
    const facts = target.agent?.facts;
    const hint = target.launchFailure ? '原生启动失败 · 精确会话可能未保留' : facts?.detail ?? '精确会话未保留';
    const status = target.launchFailure ? '启动失败' : facts ? relationState(facts.status) : '不可用';
    return <li key={`${name}:${incarnation}`}><Button disabled={target.availability !== 'available'} aria-disabled={busy || undefined} onClick={() => void open(target)} aria-label={`打开 ${name} 的精确会话`} title={hint}><span>@{name}</span><small>{status}</small></Button></li>;
  }
  const origin = page?.origin === 'external' ? '外部启动' : page?.origin === 'unknown' ? '来源未知' : '来源未保留';
  const upstream = page?.upstream ? <ul>{item(page.upstream)}</ul> : <small>{origin}</small>;
  const downstream = page?.downstream.length ? <ul>{page.downstream.map(item)}</ul> : <small>暂无启动记录</small>;
  const source = page?.forkSource;
  const forkSource = source && <div className="launch-fork-source"><strong>Fork 来源</strong><small>@{source.name} · {source.kind === 'stopped' ? `停止历史 #${source.stopped_event}` : `原实例 ${source.incarnation}`}</small></div>;
  function nextPage() {
    if (!page?.next) return;
    setPrevious(value => [...value, after]); setAfter(page.next); setPage(undefined); setOpenError('');
  }
  function previousPage() {
    setAfter(previous.at(-1)); setPrevious(value => value.slice(0, -1)); setPage(undefined); setOpenError('');
  }
  const readStatus = readError ? <div className="launch-relations-status"><span role="alert">{readError}</span><button type="button" onClick={() => { setReadError(''); setRevision(value => value + 1); }}>重试</button></div> : !page ? <small role="status">关系读取中…</small> : null;
  const paging = page && (previous.length > 0 || page.next) && <div className="launch-relations-paging">
    {previous.length > 0 && <button type="button" onClick={previousPage}>上一页</button>}
    {page.next && <button type="button" onClick={nextPage}>更多启动关系</button>}
  </div>;
  return <section className="launch-relations" data-compact={compact || undefined} aria-label="Agent 启动关系">
    {readStatus}{openError && <div className="launch-relations-status" role="alert">{openError}</div>}
    {page && <><div><strong>{compact ? '谁启动了我' : '启动来源'}</strong>{upstream}</div><div><strong>我启动的 Agent</strong>{downstream}{paging}</div>{forkSource}</>}
  </section>;
}

function relationState(value: string) {
  const names: Record<string, string> = { active: '运行中', working: '工作中', listening: '等待', idle: '空闲', ready: '就绪', inactive: '未活跃', failed: '失败', blocked: '阻塞', stopped: '已停止' };
  return names[value] ?? value;
}
