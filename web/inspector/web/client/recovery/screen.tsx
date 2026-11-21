/** @aiPurpose Coordinate recovery selection, navigation, dialogs and explicit user commands. */
import { useEffect, useMemo, useRef, useState } from "react";
import type { Incident, RecoveryCycle, Target } from "../../shared/contracts";
import {
  Badge,
  Button,
  CopyValue,
  Icon,
  IconButton,
  Modal,
} from "../../../uiux/systems/recovery/recovery.web";
import {
  ContinueDialog,
  FilterDialog,
  ManageDialog,
  PolicyDialog,
  type ContinueDraft,
} from "./dialogs";
import { Detail, LocalRecord } from "./detail";
import {
  emptyFilters,
  formatTime,
  reasonFor,
  toTarget,
  visibleIncidents,
  type Filters,
  type Selection,
} from "./model";
import { Queue, type QueueMode } from "./queue";
import { useRecovery } from "./use-recovery";
import { RecoveryLayout } from "./layouts/workspace";
type Layer =
  | { kind: "continue"; draft: ContinueDraft }
  | { kind: "filters" }
  | { kind: "policy" }
  | { kind: "manage"; cycle: RecoveryCycle }
  | { kind: "requests" }
  | null;
/**
 * @aiPurpose Compose the recovery workspace and retain view, target and overlay state.
 * @aiUsage Mount as the recovery surface; no effects are triggered by browsing.
 */
/** @aiPurpose Compose RecoveryScreen presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecoveryScreen() {
  const controller = useRecovery(),
    [filters, setFilters] = useState<Filters>(emptyFilters),
    [mode, setMode] = useState<QueueMode>("agents"),
    [selection, setSelection] = useState<Selection | null>(null),
    [detailOpen, setDetailOpen] = useState(false),
    [checked, setChecked] = useState<Target[]>([]),
    [layer, setLayer] = useState<Layer>(null),
    initialized = useRef(false),
    returnFocus = useRef<HTMLElement | null>(null);
  const data = controller.data;
  useEffect(() => {
    if (!data || initialized.current) return;
    initialized.current = true;
    const attention = visibleIncidents(
      data,
      emptyFilters,
      controller.pendingAgent,
    );
    if (!attention.length && data.attempts.length) {
      setMode("records");
      if (matchMedia("(min-width:901px)").matches) {
        setSelection({
          kind: "attempt",
          id: [...data.attempts].sort((a, b) =>
            b.createdAt.localeCompare(a.createdAt),
          )[0].id,
        });
        setDetailOpen(true);
      }
    } else if (attention.length && matchMedia("(min-width:901px)").matches) {
      setSelection({ kind: "incident", id: attention[0].id });
      setDetailOpen(true);
    }
  }, [data]);
  const close = () => setLayer(null);
  const select = (value: Selection) => {
    returnFocus.current = document.activeElement as HTMLElement;
    setSelection(value);
    setDetailOpen(true);
  };
  const back = () => {
    setDetailOpen(false);
    requestAnimationFrame(() => returnFocus.current?.focus());
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (layer) return;
      if (
        e.key === "/" &&
        !["INPUT", "TEXTAREA", "SELECT"].includes(
          (e.target as HTMLElement).tagName,
        )
      ) {
        e.preventDefault();
        document.getElementById("recovery-search")?.focus();
      }
      if (e.key === "Escape" && detailOpen) {
        e.preventDefault();
        back();
      }
    };
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, [layer, detailOpen]);
  const continueItems = (items: Incident[]) => {
    controller.clearWriteError();
    setLayer({
      kind: "continue",
      draft: {
        operationId: controller.createOperationId(),
        items: structuredClone(items),
        targets: items.map(toTarget),
      },
    });
  };
  const targets = useMemo(
    () =>
      checked.map((t) => ({
        target: t,
        item: data?.incidents.find(
          (i) => i.id === t.incidentId && i.revision === t.revision,
        ),
      })),
    [checked, data],
  );
  const invalidSelection = targets.some(
    ({ item }) =>
      !item ||
      !!reasonFor(item, data!) ||
      controller.pendingAgent(item.agentId),
  );
  const toggle = (item: Incident) =>
    setChecked((current) =>
      current.some((t) => t.incidentId === item.id)
        ? current.filter((t) => t.incidentId !== item.id)
        : [...current, toTarget(item)],
    );
  const unresolved = controller.records.filter(
      (r) => r.status === "unknown" || r.status === "sending",
    ),
    lastRecord = controller.records.at(-1);
  const header = (
    <>
      <div className="workspace-brand">
        <Icon name="inspector" />
        <span className="product-name">Agent Inspector</span>
        <span className="header-divider" />
        <h1>恢复</h1>
      </div>
      <div className="host-name">
        <Icon name="host" />
        <span>{controller.session?.hostName || "连接中"}</span>
      </div>
      <div className="header-actions">
        {data && (
          <button
            className="policy-status"
            onClick={() => {
              controller.clearWriteError();
              setLayer({ kind: "policy" });
            }}
          >
            <Badge
              icon={data.automation.policy.enabled ? "bolt" : "pause"}
              tone={data.automation.policy.enabled ? "accent" : "neutral"}
            >
              自动{data.automation.policy.enabled ? "开启" : "关闭"}
            </Badge>
          </button>
        )}
        <IconButton
          label="刷新观察"
          icon="recovery"
          disabled={controller.fetching}
          onClick={() => void controller.refresh()}
        />
        <IconButton
          label="自动恢复策略"
          icon="settings"
          disabled={!data}
          onClick={() => {
            controller.clearWriteError();
            setLayer({ kind: "policy" });
          }}
        />
      </div>
    </>
  );
  const notice =
    controller.error ||
    data?.automation.monitor.error ||
    (controller.stale && data) ||
    unresolved.length > 0 ? (
      <div className="workspace-notices">
        {(controller.error ||
          data?.automation.monitor.error ||
          (controller.stale && data)) && (
          <div className="notice-row warning" role="status">
            <Icon name="warning" />
            <span>
              {controller.error ||
                data?.automation.monitor.error ||
                "观察已过期"}
              {data && (
                <small> · 保留 {formatTime(data.scannedAt)} 的记录</small>
              )}
            </span>
            <Button
              variant="quiet"
              onClick={() => void controller.refresh()}
              disabled={controller.fetching}
            >
              刷新
            </Button>
          </div>
        )}
        {unresolved.length > 0 && (
          <button
            className="notice-row unknown request-notice"
            onClick={() => setLayer({ kind: "requests" })}
          >
            <Icon name="unknown" />
            <span>
              {unresolved.some((r) => r.status === "unknown")
                ? "原请求待核实"
                : "正在提交"}{" "}
              <b>{unresolved.length}</b>
            </span>
            <span className="notice-end">
              查看原请求 <Icon name="arrow" />
            </span>
          </button>
        )}
      </div>
    ) : null;
  const footer = checked.length ? (
    <footer className="batch-bar">
      <Icon name="check" />
      <strong>已选 {checked.length} 项</strong>
      {invalidSelection && (
        <span className="batch-warning">部分目标已变化</span>
      )}
      <Button variant="quiet" onClick={() => setChecked([])}>
        清除
      </Button>
      <Button
        variant="primary"
        icon="run"
        disabled={
          controller.commandBlocked || invalidSelection || checked.length > 500
        }
        onClick={() => continueItems(targets.map((x) => x.item!))}
      >
        继续所选 {checked.length}
      </Button>
    </footer>
  ) : (
    <footer className="workspace-foot">
      <span>
        {data
          ? `当前主机 · ${data.incidents.length} 条异常 · ${data.attempts.length} 条尝试`
          : "读取当前主机"}
      </span>
      {lastRecord && (
        <button
          className="text-action"
          onClick={() => setLayer({ kind: "requests" })}
        >
          {lastRecord.status === "rejected"
            ? "最近请求被拒绝"
            : lastRecord.status === "received"
              ? "查看最近请求"
              : "原请求"}
          <Icon name="arrow" />
        </button>
      )}
    </footer>
  );
  return (
    <>
      <RecoveryLayout
        header={header}
        notice={notice}
        detailOpen={detailOpen}
        queue={
          <Queue
            controller={controller}
            filters={filters}
            setFilters={setFilters}
            mode={mode}
            setMode={(value) => {
              setMode(value);
              setDetailOpen(false);
              setSelection(null);
            }}
            selection={selection}
            onSelect={select}
            checked={checked}
            onToggle={toggle}
            onFilters={() => setLayer({ kind: "filters" })}
          />
        }
        detail={
          <Detail
            key={selection ? `${selection.kind}:${selection.id}` : "empty"}
            controller={controller}
            selection={selection}
            onBack={back}
            onContinue={continueItems}
            onManage={(cycle) => {
              controller.clearWriteError();
              setLayer({ kind: "manage", cycle });
            }}
          />
        }
        footer={footer}
      />
      {layer?.kind === "continue" && (
        <ContinueDialog
          draft={layer.draft}
          controller={controller}
          onClose={close}
        />
      )}{" "}
      {layer?.kind === "filters" && data && (
        <FilterDialog
          filters={filters}
          data={data}
          onChange={setFilters}
          onClose={close}
        />
      )}{" "}
      {layer?.kind === "policy" && data && (
        <PolicyDialog controller={controller} onClose={close} />
      )}{" "}
      {layer?.kind === "manage" && (
        <ManageDialog
          cycle={layer.cycle}
          controller={controller}
          onClose={close}
        />
      )}{" "}
      {layer?.kind === "requests" && (
        <Modal
          title="本标签页请求"
          onClose={close}
          footer={
            <Button
              icon="recovery"
              onClick={() => void controller.refresh()}
              disabled={controller.fetching}
            >
              刷新核查
            </Button>
          }
        >
          {[...controller.records].reverse().map((record) => (
            <LocalRecord key={record.id} record={record} />
          ))}
          {!controller.records.length && <p className="muted">暂无本地请求</p>}
        </Modal>
      )}
    </>
  );
}
