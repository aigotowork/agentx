/** @aiPurpose Own fresh reads, persisted local command identity and never-retried writes. */
import { useCallback, useEffect, useRef, useState } from "react";
import type {
  Attempt,
  ContinueInput,
  ContinueReply,
  ConfigureReply,
  ManageInput,
  ManageReply,
  Policy,
  RecoveryList,
  WebSession,
} from "../../shared/contracts";
import { ApiFailure, request } from "./api";
type Command =
  | { kind: "continue"; input: ContinueInput }
  | { kind: "manage"; input: ManageInput }
  | { kind: "configure"; input: Policy };
export type WriteRecord = Command & {
  id: string;
  at: string;
  status: "sending" | "unknown" | "received" | "rejected";
  message: string;
  agentIds?: string[];
};
const storageKey = "agentx.inspector.mock.pending.v1";
// Business state belongs to this mock page. Reload resets provider and command barriers together.
const localRecords: {value:WriteRecord[]} = {value:[]};
function readRecords():WriteRecord[] {return localRecords.value;}
function operationId() {
  const words = new Uint32Array(4);
  crypto.getRandomValues(words);
  return `web-${Date.now().toString(36)}-${[...words].map((n) => n.toString(16)).join("")}`;
}
/**
 * @aiPurpose Own recovery queries, freshness and durable command uncertainty.
 * @aiUsage Use once per host surface; queries refresh, commands always require a caller confirmation.
 */
export function useRecovery() {
  const [data, setData] = useState<RecoveryList | null>(null),
    [session, setSession] = useState<WebSession | null>(null),
    [error, setError] = useState<string | null>(null),
    [fetching, setFetching] = useState(false),
    [receivedAt, setReceivedAt] = useState(0),
    [now, setNow] = useState(Date.now()),
    [records, setRecords] = useState<WriteRecord[]>(readRecords),
    [writeError, setWriteError] = useState<string | null>(null);
  const readLock = useRef(false),
    writeLock = useRef(false),
    mounted = useRef(true),
    recordsRef = useRef(records),
    sessionRef = useRef(session),
    latest = useRef(data),
    lifecycle = useRef(0),
    clockOffset = useRef(0);
  const store = useCallback((value: WriteRecord[]) => {
    localRecords.value = value;
    recordsRef.current = value;
    if (mounted.current) setRecords(value);
  }, []);
  const refresh = useCallback(async () => {
    if (readLock.current) return;
    const generation = lifecycle.current;
    readLock.current = true;
    if (mounted.current) setFetching(true);
    try {
      const [s, d] = await Promise.all([
        sessionRef.current
          ? Promise.resolve(sessionRef.current)
          : request<WebSession>("/api/session"),
        request<RecoveryList>("/api/recovery"),
      ]);
      if (
        !Array.isArray(d.incidents) ||
        !Array.isArray(d.attempts) ||
        !d.automation?.policy ||
        !s.csrfToken
      )
        throw new ApiFailure("恢复数据格式不完整", "invalid_response", false);
      if (!mounted.current || generation !== lifecycle.current) return;
      if (!sessionRef.current)
        clockOffset.current = Date.parse(s.serverTime) - Date.now();
      sessionRef.current = s;
      latest.current = d;
      setSession(s);
      setData(d);
      setReceivedAt(Date.now());
      setNow(Date.now());
      setError(null);
      const resolved = recordsRef.current.map((r) =>
        r.kind === "continue" &&
        r.status === "unknown" &&
        r.input.targets.every((t) =>
          d.attempts.some(
            (a) =>
              a.operationId === r.input.operationId &&
              a.incidentId === t.incidentId &&
              a.revision === t.revision,
          ),
        )
          ? { ...r, status: "received" as const, message: "已查到原操作记录" }
          : r,
      );
      if (JSON.stringify(resolved) !== JSON.stringify(recordsRef.current))
        try {
          store(resolved);
        } catch {
          /* Keep the stronger existing barrier if persistence fails. */
        }
    } catch (e) {
      if (mounted.current && generation === lifecycle.current)
        setError(e instanceof Error ? e.message : "读取失败");
    } finally {
      readLock.current = false;
      if (mounted.current) setFetching(false);
    }
  }, [store]);
  useEffect(() => {
    mounted.current = true;
    void refresh();
    const timer = setInterval(() => {
      setNow(Date.now());
      if (document.visibilityState === "visible") void refresh();
    }, 3000);
    const visible = () => {
      setNow(Date.now());
      if (document.visibilityState === "visible") void refresh();
    };
    document.addEventListener("visibilitychange", visible);
    window.addEventListener("online", visible);
    return () => {
      mounted.current = false;
      lifecycle.current++;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", visible);
      window.removeEventListener("online", visible);
    };
  }, [refresh]);
  const serverOffset = clockOffset.current;
  const scanAge = data
    ? now + serverOffset - Date.parse(data.scannedAt)
    : Infinity;
  const stale =
    !data ||
    !!error ||
    now - receivedAt > 15000 ||
    !Number.isFinite(scanAge) ||
    scanAge > 15000 ||
    !!data.automation.monitor.error;
  const unknown = records.filter((r) => r.status === "unknown"),
    sending = records.some((r) => r.status === "sending");
  const commandBlocked =
    stale || !session || sending || unknown.some((r) => r.kind !== "continue");
  const pendingAgent = (agentId: string) =>
    records.some(
      (r) =>
        r.kind === "continue" &&
        ["unknown", "sending"].includes(r.status) &&
        (r.agentIds?.includes(agentId) ||
          r.input.targets.some((t) =>
            latest.current?.incidents.some(
              (i) => i.id === t.incidentId && i.agentId === agentId,
            ),
          )),
    );
  async function execute(command: Command): Promise<boolean> {
    if (writeLock.current || commandBlocked) {
      setWriteError("当前状态不可提交，请刷新核验");
      return false;
    }
    if (
      command.kind === "continue" &&
      command.input.targets.some((t) => {
        const i = latest.current?.incidents.find((i) => i.id === t.incidentId);
        return !i || i.revision !== t.revision || pendingAgent(i.agentId);
      })
    ) {
      setWriteError("目标版本已变化或原操作尚未核实");
      return false;
    }
    writeLock.current = true;
    setWriteError(null);
    const record: WriteRecord = {
      ...command,
      id:
        command.kind === "continue" ? command.input.operationId : operationId(),
      at: new Date().toISOString(),
      status: "sending",
      message: "正在提交",
      agentIds:
        command.kind === "continue"
          ? command.input.targets
              .map(
                (t) =>
                  latest.current?.incidents.find((i) => i.id === t.incidentId)
                    ?.agentId,
              )
              .filter((id): id is string => !!id)
          : undefined,
    };
    try {
      store([...recordsRef.current, record]);
    } catch {
      writeLock.current = false;
      setWriteError("无法保存原操作身份，未发送请求");
      return false;
    }
    try {
      const reply = await request<ContinueReply | ManageReply | ConfigureReply>(
        `/api/recovery/${command.kind}`,
        { body: command.input, csrf: sessionRef.current!.csrfToken },
      );
      if (command.kind === "continue") {
        const attempts = (reply as ContinueReply).attempts;
        if (
          !Array.isArray(attempts) ||
          !command.input.targets.every((t) =>
            attempts.some(
              (a) =>
                a.operationId === command.input.operationId &&
                a.incidentId === t.incidentId &&
                a.revision === t.revision,
            ),
          )
        )
          throw new ApiFailure("回执缺少原目标记录", "invalid_receipt", true);
        setData((current) => {
          if (!current) return current;
          const ids = new Set(attempts.map((a) => a.id));
          const next = {
            ...current,
            attempts: [
              ...current.attempts.filter((a) => !ids.has(a.id)),
              ...attempts,
            ],
          };
          latest.current = next;
          return next;
        });
      } else if (command.kind === "manage") {
        const cycle = (reply as ManageReply).cycle;
        if (cycle?.id !== command.input.cycleId)
          throw new ApiFailure("链回执不匹配", "invalid_receipt", true);
      } else if (!(reply as ConfigureReply).policy)
        throw new ApiFailure("策略回执不完整", "invalid_receipt", true);
      store(
        recordsRef.current.map((r) =>
          r.id === record.id
            ? { ...r, status: "received", message: "已收到本地模拟回执" }
            : r,
        ),
      );
      void refresh();
      return true;
    } catch (e) {
      const failure =
        e instanceof ApiFailure
          ? e
          : new ApiFailure("请求结果未能确认", "effect_unconfirmed", true);
      const changed = recordsRef.current.map((r) =>
        r.id === record.id
          ? {
              ...r,
              status: failure.uncertain
                ? ("unknown" as const)
                : ("rejected" as const),
              message: failure.message,
            }
          : r,
      );
      try {
        store(changed);
      } catch {
        recordsRef.current = changed;
        setRecords(changed);
      }
      if (failure.code === "invalid_csrf") {
        sessionRef.current = null;
        setSession(null);
      }
      setWriteError(
        failure.uncertain
          ? "请求效果未知，保留原操作；刷新核查，勿重复提交。"
          : failure.message,
      );
      void refresh();
      return failure.uncertain;
    } finally {
      writeLock.current = false;
    }
  }
  return {
    data,
    session,
    error,
    fetching,
    stale,
    receivedAt,
    records,
    unknown,
    sending,
    commandBlocked,
    pendingAgent,
    writeError,
    refresh,
    execute,
    createOperationId: operationId,
    clearWriteError: () => setWriteError(null),
  };
}
export type RecoveryController = ReturnType<typeof useRecovery>;
