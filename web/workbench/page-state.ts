/** @aiPurpose Coordinate native reads and captured commands while preserving Page-owned drafts. */
import { useCallback, useEffect, useRef, useState } from "react";
import { AdapterError, OutcomeUnknown, WorkbenchApi, newCommandId } from "./api/client";
import type { ApplyCall, Change, OperatorCommand, Read, ReadValue, WorkDetail, WorkRef } from "./api/native";
import type { BriefDraft, CommandState, ConversationDraft, DetailTab, ResourceData, ResourceView, Selection, ThreadState, WorkFilter } from "./contracts";
import { assignment, failureText, makeDraft, parsePending, participantKey, readAddress } from "./state";

const pendingKey = "agentx-workbench-pending-v1";
const initialAddress = readAddress(window.location.hash);
const emptyData: ResourceData = { descriptor: null, details: [], candidates: [], loading: false, error: null, observedAt: null, next: null, candidateNext: null };
function errorText(error: unknown): string { return error instanceof AdapterError ? `连接拒绝：${error.code}` : error instanceof Error ? error.message : "读取失败"; }
function restoredCommand(): CommandState { try { const pending = parsePending(sessionStorage.getItem(pendingKey)); return pending ? { status: "restored", ...pending } : { status: "idle" }; } catch { return { status: "idle" }; } }

/**
 * @aiPurpose Own one fixed workspace's authoritative snapshots and finite interaction state.
 * @aiUsage The Page passes child intents into command; Blocks never receive a transport client.
 */
export function useWorkbenchPage() {
  const [data, setData] = useState<ResourceData>(emptyData); const dataRef = useRef(data); dataRef.current = data;
  const [selection, setSelection] = useState<Selection>(initialAddress.selection); const [view, setView] = useState<ResourceView>(initialAddress.view); const [source, setSource] = useState<string | null>(null); const [queries, setQueries] = useState<Record<ResourceView, string>>({ work: "", agents: "", results: "", checks: "" }); const [filter, setFilter] = useState<WorkFilter>("all"); const [drafts, setDrafts] = useState<Record<string, BriefDraft>>({}); const [talkDrafts, setTalkDrafts] = useState<Record<string, ConversationDraft>>({}); const [command, setCommand] = useState<CommandState>(restoredCommand); const [notice, setNotice] = useState<string | null>(null); const [discard, setDiscard] = useState<string | null>(null); const [collapsed, setCollapsed] = useState(false); const selectionRef = useRef(selection); selectionRef.current = selection;
  const api = useRef<WorkbenchApi | null>(null); const readAbort = useRef<AbortController | null>(null); const readVersion = useRef(0); const mutationVersion = useRef(0); const commandBusy = useRef(false); const mounted = useRef(true);
  const [thread, setThread] = useState<ThreadState>({ workId: null, generation: null, items: [], next: null, loading: false, error: null }); const threadRef = useRef(thread); threadRef.current = thread; const threadAbort = useRef<AbortController | null>(null);

  const nativeRead = async (connection: WorkbenchApi, read: Read, signal: AbortSignal): Promise<ReadValue> => { const generation = connection.descriptor.generation; if (!generation) throw new Error("数据代不可用"); const response = await connection.read({ generation, read }, signal); if ("Rejected" in response) throw new Error(failureText(response.Rejected)); if (!("Read" in response)) throw new Error("读取返回不匹配"); return response.Read; };
  const refresh = useCallback(async (connection = api.current, more = false, candidatesOnly = false, candidateMore = false) => {
    if (!connection || !connection.descriptor.generation) return;
    readAbort.current?.abort(); const abort = new AbortController(); readAbort.current = abort; const version = ++readVersion.current; const mutation = mutationVersion.current; const previous = dataRef.current;
    setData(old => ({ ...old, loading: true, error: null }));
    try {
      const limit = connection.descriptor.limits.page;
      const candidateValue = await nativeRead(connection, { Candidates: { limit, after: candidateMore ? previous.candidateNext : null } }, abort.signal);
      if (!("Candidates" in candidateValue)) throw new Error("候选读取返回不匹配");
      const candidateItems = [...candidateValue.Candidates.items]; let candidateNext = candidateValue.Candidates.next ?? null;
      while (!candidateMore && candidateNext && candidateItems.length < previous.candidates.length) { const value = await nativeRead(connection, { Candidates: { limit, after: candidateNext } }, abort.signal); if (!("Candidates" in value)) throw new Error("候选页返回不匹配"); candidateItems.push(...value.Candidates.items); candidateNext = value.Candidates.next ?? null; }
      let details = previous.details; let next = previous.next;
      if (!candidatesOnly) {
        const listValue = await nativeRead(connection, { List: { limit, after: more ? previous.next : null } }, abort.signal);
        if (!("List" in listValue)) throw new Error("工作列表返回不匹配");
        next = listValue.List.next ?? null;
        const refs = new Map<string, WorkRef>();
        if (more || previous.descriptor?.generation === connection.descriptor.generation) previous.details.filter(detail => detail.work.reference.generation === connection.descriptor.generation && detail.work.reference.scope === connection.descriptor.scope && detail.work.reference.project === connection.descriptor.project).forEach(detail => refs.set(detail.work.reference.id, detail.work.reference));
        listValue.List.items.forEach(item => refs.set(item.reference.id, item.reference));
        const direct = readAddress(window.location.hash);
        const directMissing = direct.selection.workId && !refs.has(direct.selection.workId) && (!direct.generation || direct.generation === connection.descriptor.generation);
        const values: WorkDetail[] = []; const references = [...refs.values()];
        for (let index = 0; index < references.length; index += 3) {
          const batch = await Promise.all(references.slice(index, index + 3).map(async reference => { const value = await nativeRead(connection, { Detail: reference }, abort.signal); if (!("Detail" in value)) throw new Error("工作详情返回不匹配"); return value.Detail; })); values.push(...batch);
        }
        if (directMissing && direct.selection.workId) { try { const value = await nativeRead(connection, { Detail: { scope: connection.descriptor.scope, project: connection.descriptor.project, generation: connection.descriptor.generation, id: direct.selection.workId } }, abort.signal); if ("Detail" in value) values.push(value.Detail); } catch (error) { if (!abort.signal.aborted) setNotice(`链接工作未找到：${errorText(error)}`); } }
        details = values;
      }
      if (!mounted.current || abort.signal.aborted || version !== readVersion.current || mutation !== mutationVersion.current) return;
      const candidates = candidateMore && previous.candidateNext ? [...previous.candidates, ...candidateItems].filter((item, index, all) => all.findIndex(value => participantKey(value.participant) === participantKey(item.participant)) === index) : candidateItems;
      setData({ descriptor: connection.descriptor, details, candidates, loading: false, error: null, observedAt: new Date().toLocaleTimeString(), next, candidateNext });
    } catch (error) { if (!mounted.current || abort.signal.aborted || version !== readVersion.current) return; setData(old => ({ ...old, loading: false, error: errorText(error) })); }
  }, []);

  const connect = useCallback(async () => {
    const version = ++readVersion.current; readAbort.current?.abort(); api.current?.close(); setData(old => ({ ...old, loading: true, error: null }));
    try { const connection = await WorkbenchApi.connect(); if (!mounted.current || version !== readVersion.current) { connection.close(); return; } api.current = connection; setData(old => ({ ...old, descriptor: connection.descriptor, loading: false })); const address = readAddress(window.location.hash); if (address.generation && address.generation !== connection.descriptor.generation) { setSelection(old => ({ ...old, workId: null, resultId: null, participant: null })); setNotice("链接的数据代已更换，请从当前工作区选择工作。"); } await refresh(connection); }
    catch (error) { if (mounted.current && version === readVersion.current) setData(old => ({ ...old, loading: false, error: errorText(error) })); }
  }, [refresh]);
  useEffect(() => { mounted.current = true; void connect(); return () => { mounted.current = false; readAbort.current?.abort(); api.current?.close(); }; }, [connect]);
  useEffect(() => { setDrafts(old => { let changed = false; const next = { ...old }; for (const detail of data.details) { const id = detail.work.reference.id; const draft = next[id]; if (draft && !draft.dirty && draft.revision !== detail.work.revision) { next[id] = makeDraft(detail); changed = true; } } const requested = selection.tab === "edit" && selection.workId !== "new" ? data.details.find(item => item.work.reference.id === selection.workId) : null; if (requested && !next[requested.work.reference.id] && Object.keys(next).length < 8) { next[requested.work.reference.id] = makeDraft(requested); changed = true; } return changed ? next : old; }); }, [data.details, selection.workId, selection.tab]);
  const loadThread = useCallback(async (reference: WorkRef, more = false) => {
    const connection = api.current; if (!connection || reference.generation !== connection.descriptor.generation) return;
    threadAbort.current?.abort(); const abort = new AbortController(); threadAbort.current = abort; const previous = threadRef.current; const same = previous.workId === reference.id && previous.generation === reference.generation;
    setThread({ workId: reference.id, generation: reference.generation, items: same ? previous.items : [], next: same ? previous.next : null, loading: true, error: null });
    try { const value = await nativeRead(connection, { Thread: { work: reference, page: { limit: connection.descriptor.limits.page, after: more && same ? previous.next : null } } }, abort.signal); if (!("Thread" in value)) throw new Error("工作线程返回不匹配"); if (!mounted.current || abort.signal.aborted || connection !== api.current) return; const items = more && same ? [...previous.items, ...value.Thread.items].filter((item, index, all) => all.findIndex(value => value.event === item.event) === index) : value.Thread.items; setThread({ workId: reference.id, generation: reference.generation, items, next: value.Thread.next ?? null, loading: false, error: null }); }
    catch (error) { if (mounted.current && !abort.signal.aborted) setThread(old => ({ ...old, loading: false, error: errorText(error) })); }
  }, []);
  useEffect(() => { if (selection.tab !== "messages") { threadAbort.current?.abort(); return; } const detail = data.details.find(item => item.work.reference.id === selection.workId); if (detail) void loadThread(detail.work.reference); return () => threadAbort.current?.abort(); }, [selection.workId, selection.tab, data.details, loadThread]);
  useEffect(() => { const onBack = (event: PopStateEvent) => { const address = readAddress(window.location.hash); setSelection(address.selection); setView(address.view); const context = event.state as { source?: string | null; queries?: Record<ResourceView, string>; filter?: WorkFilter } | null; if (context) { setSource(context.source ?? null); if (context.queries) setQueries(context.queries); if (context.filter) setFilter(context.filter); } setCollapsed(false); }; window.addEventListener("popstate", onBack); return () => window.removeEventListener("popstate", onBack); }, []);
  const dirty = Object.values(drafts).some(draft => draft.dirty) || Object.values(talkDrafts).some(draft => !!draft.answer || !!draft.note); const hasPending = command.status === "pending" || command.status === "unknown" || command.status === "restored";
  useEffect(() => { if (!dirty && !hasPending) return; const beforeUnload = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = ""; }; window.addEventListener("beforeunload", beforeUnload); return () => window.removeEventListener("beforeunload", beforeUnload); }, [dirty, hasPending]);

  const address = (next: Selection, nextView = view) => { const params = new URLSearchParams(); params.set("view", nextView); if (next.workId && next.workId !== "new") params.set("work", next.workId); if (next.resultId) params.set("result", next.resultId); if (next.tab !== "brief") params.set("tab", next.tab); if (data.descriptor?.generation) params.set("generation", data.descriptor.generation); const context = { source, queries, filter }; history.replaceState(context, "", location.href); history.pushState(context, "", `#${params}`); };
  const select = (id: string, tab: DetailTab = "brief", resourceId?: string) => { const next = { workId: id, tab, resultId: resourceId ?? null, questionId: null, participant: null }; setSelection(next); setCollapsed(false); address(next); if (tab === "edit") openEditor(id); };
  const changeView = (next: ResourceView) => { setView(next); address(selection, next); };
  const openEditor = (id: string) => { if (drafts[id]) return; if (Object.keys(drafts).length >= 8) { setNotice("已保留 8 份本页草稿，请先保存或明确放弃一份。"); return; } const detail = data.details.find(item => item.work.reference.id === id); if (detail) setDrafts(old => ({ ...old, [id]: makeDraft(detail) })); };
  const create = () => { if (!drafts.new) setDrafts(old => ({ ...old, new: makeDraft(null) })); const next: Selection = { workId: "new", tab: "edit", resultId: null, questionId: null, participant: null }; setSelection(next); setCollapsed(false); };
  const storePending = (call: ApplyCall) => { try { sessionStorage.setItem(pendingKey, JSON.stringify({ commandId: call.command_id, generation: call.generation, scope: call.work?.scope ?? data.descriptor?.scope, project: call.work?.project ?? data.descriptor?.project, work: call.work })); } catch { /* Storage can be unavailable; the in-memory captured intent remains. */ } };
  const clearPending = () => { try { sessionStorage.removeItem(pendingKey); } catch { /* No payload or permission is stored. */ } };
  const receiveChange = async (change: Change, call: ApplyCall | null, label: string) => {
    mutationVersion.current++; setCommand({ status: "committed", change, label }); clearPending();
    if (call && "CreateWork" in call.command) { setDrafts(old => { const next = { ...old }; delete next.new; return next; }); if (selectionRef.current.workId === "new") select(change.reference.id); }
    if (call && "UpdateBrief" in call.command && call.work) { setDrafts(old => { const next = { ...old }; delete next[call.work!.id]; return next; }); setSelection(old => old.workId === call.work!.id ? { ...old, tab: "brief" } : old); }
    await refresh();
  };
  const submitCall = async (call: ApplyCall, label: string): Promise<boolean> => {
    const connection = api.current; if (!connection || commandBusy.current) return false;
    commandBusy.current = true; mutationVersion.current++; readAbort.current?.abort(); setData(old => ({ ...old, loading: false })); setCommand({ status: "pending", call, label }); storePending(call);
    try {
      const response = await connection.apply(call);
      if (!mounted.current) return false;
      if (api.current !== connection || api.current.descriptor.generation !== call.generation) { setCommand({ status: "unknown", call, label }); return false; }
      if ("Rejected" in response) { clearPending(); setCommand({ status: "rejected", failure: response.Rejected, workId: call.work?.id ?? null, label }); if (typeof response.Rejected === "object" && "RevisionConflict" in response.Rejected) await refresh(); return false; }
      if (!("Applied" in response) || response.Applied.command_id !== call.command_id || response.Applied.reference.generation !== call.generation || response.Applied.reference.scope !== connection.descriptor.scope || response.Applied.reference.project !== connection.descriptor.project || call.work && response.Applied.reference.id !== call.work.id) { setCommand({ status: "unknown", call, label }); return false; }
      await receiveChange(response.Applied, call, label); return true;
    } catch (error) { if (mounted.current) { if (error instanceof OutcomeUnknown) setCommand({ status: "unknown", call, label }); else { clearPending(); setCommand({ status: "rejected", failure: errorText(error), workId: call.work?.id ?? null, label }); } } return false; }
    finally { commandBusy.current = false; }
  };
  const perform = async (nativeCommand: OperatorCommand, detail: WorkDetail | null, label: string, expectedRevision?: bigint): Promise<boolean> => {
    const descriptor = api.current?.descriptor;
    if (!descriptor?.generation || !descriptor.available || !descriptor.collaboration || commandBusy.current || hasPending) return false;
    if (detail && (detail.work.reference.generation !== descriptor.generation || detail.eligibility.closed)) return false;
    return submitCall({ generation: descriptor.generation, command_id: newCommandId(), work: detail?.work.reference ?? null, expected_revision: expectedRevision ?? detail?.work.revision ?? null, command: structuredClone(nativeCommand) }, label);
  };
  const lookup = async () => {
    const connection = api.current; if (!connection || commandBusy.current || !(command.status === "unknown" || command.status === "restored")) return;
    const commandId = command.status === "restored" ? command.commandId : command.call.command_id; const generation = command.status === "restored" ? command.generation : command.call.generation;
    if (generation !== connection.descriptor.generation || command.status === "restored" && (command.scope !== connection.descriptor.scope || command.project !== connection.descriptor.project)) { setNotice("数据已更换，旧操作不能查询或重放。请保留原命令身份并刷新工作。"); return; }
    commandBusy.current = true;
    try { const response = await connection.lookup(commandId); if ("Read" in response && "Command" in response.Read) { const change = response.Read.Command; if (change && change.command_id === commandId && change.reference.generation === generation) await receiveChange(change, command.status === "unknown" ? command.call : null, "查询原操作"); else setNotice("尚无原操作回执；原请求仍可能提交。保持原命令身份。"); } else if ("Rejected" in response) setNotice(failureText(response.Rejected)); }
    catch (error) { setNotice(errorText(error)); } finally { commandBusy.current = false; }
  };
  const retry = () => { if (command.status === "unknown" && command.call.generation === api.current?.descriptor.generation) void submitCall(command.call, command.label); };
  const abandonOldGeneration = () => { const generation = command.status === "restored" ? command.generation : command.status === "unknown" ? command.call.generation : null; if (generation && data.descriptor?.generation && generation !== data.descriptor.generation) { clearPending(); setCommand({ status: "idle" }); setNotice("旧代操作未重放，请查看当前工作状态后再明确操作。"); } };
  const selected = data.details.find(item => item.work.reference.id === selection.workId) ?? null;
  const draftKey = selection.workId ?? "new"; const draft = drafts[draftKey]; const question = selected?.work.questions.find(item => item.id === selection.questionId) ?? selected?.work.questions.find(item => !item.answer) ?? selected?.work.questions.at(-1); const talkKey = `${selection.workId}:${selection.tab === "messages" ? "note" : question?.id ?? "none"}`;
  const talkDraft: ConversationDraft = talkDrafts[talkKey] ?? { answer: "", amendment: false, amendmentBrief: null, note: "", roles: ["Implementer"], recipients: "", acknowledged: false };
  const unavailable = !data.descriptor?.available ? "服务不可用" : !data.descriptor.collaboration ? "当前入口只读" : !data.descriptor.generation ? "数据代不可用" : hasPending ? command.status === "pending" ? "操作正在提交" : "先查询原操作结果" : selected?.eligibility.closed ? "工作已接受" : selected && selected.work.reference.generation !== data.descriptor.generation ? "数据已更换" : null;
  const updateDraft = (value: BriefDraft) => setDrafts(old => ({ ...old, [draftKey]: value }));
  const updateTalk = (value: ConversationDraft) => setTalkDrafts(old => ({ ...old, [talkKey]: value }));
  const confirmDiscard = () => { if (!discard) return; setDrafts(old => { const next = { ...old }; delete next[discard]; return next; }); if (discard === "new" && selection.workId === "new") { setSelection(old => ({ ...old, workId: null })); setCollapsed(true); } else if (selection.workId === discard) setSelection(old => ({ ...old, tab: "brief" })); setDiscard(null); };
  return { data, selection, view, source, query: queries[view], filter, drafts, draft, talkDraft, command, notice, discard, collapsed, selected, unavailable, thread, moreThread: () => { if (selected) void loadThread(selected.work.reference, true); }, refreshThread: () => { if (selected) void loadThread(selected.work.reference); }, setNotice, setDiscard, setCollapsed, setSource, setFilter, setSelection, changeView, setQuery: (query: string) => setQueries(old => ({ ...old, [view]: query })), clearFilters: () => { setSource(null); setFilter("all"); setQueries(old => ({ ...old, [view]: "" })); }, select, create, connect, refresh: () => void refresh(), more: () => void refresh(api.current, true), refreshCandidates: () => void refresh(api.current, false, true), moreCandidates: () => void refresh(api.current, false, true, true), perform, lookup, retry, abandonOldGeneration, updateDraft, updateTalk, confirmDiscard, inspectParticipant: (key: string) => { setSelection(old => ({ ...old, participant: key })); setCollapsed(false); }, copyLink: async () => { const params = new URLSearchParams(); if (selection.workId && selection.workId !== "new") params.set("work", selection.workId); params.set("tab", selection.tab); if (selection.resultId) params.set("result", selection.resultId); if (data.descriptor?.generation) params.set("generation", data.descriptor.generation); const link = `${location.origin}${location.pathname}#${params}`; try { await navigator.clipboard.writeText(link); setNotice("链接已复制"); } catch { setNotice(link); } } };
}
