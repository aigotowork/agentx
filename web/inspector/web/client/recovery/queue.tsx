/** @aiPurpose Present scoped Agent, attempt and cycle collections with fixed explicit selection. */
import { useMemo } from "react";
import type { RecoveryList, Incident, Target } from "../../shared/contracts";
import { Badge, Empty, Icon, IconButton, Tabs } from "../../../uiux/systems/recovery/recovery.web";
import {
  attemptStatus,
  categoryLabels,
  cycleLabels,
  formatTime,
  incidentStatus,
  matches,
  reasonFor,
  visibleIncidents,
  type Filters,
  type Selection,
} from "./model";
import type { RecoveryController } from "./use-recovery";
export type QueueMode = "agents" | "records" | "cycles";
export interface QueueProps {
  controller: RecoveryController;
  filters: Filters;
  setFilters(value: Filters): void;
  mode: QueueMode;
  setMode(mode: QueueMode): void;
  selection: Selection | null;
  onSelect(value: Selection): void;
  checked: Target[];
  onToggle(item: Incident): void;
  onFilters(): void;
}
/**
 * @aiPurpose Control collection navigation and browser filters.
 * @aiUsage Control collection and view filters; search never changes the fixed command targets.
 */
/** @aiPurpose Compose QueueToolbar presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function QueueToolbar({
  controller,
  filters,
  setFilters,
  mode,
  setMode,
  onFilters,
}: QueueProps) {
  const data = controller.data;
  const active = Object.entries(filters).filter(
    ([k, v]) => v && k !== "search" && k !== "status",
  ).length;
  const attention = data
    ? visibleIncidents(
        data,
        { ...filters, status: "attention" },
        controller.pendingAgent,
      ).length
    : 0;
  return (
    <div className="queue-tools">
      <div className="queue-mode">
        <Tabs
          label="恢复内容"
          value={mode}
          onChange={setMode}
          items={[
            { id: "agents", label: "Agent", count: data?.incidents.length },
            { id: "records", label: "操作记录", count: data?.attempts.length },
            {
              id: "cycles",
              label: "恢复链",
              count: data?.automation.cycles.length,
            },
          ]}
        />
      </div>
      <div className="search-row">
        <label className="search-box">
          <Icon name="search" />
          <input
            id="recovery-search"
            aria-label="搜索当前主机记录"
            type="search"
            placeholder="搜索 Agent、项目、错误…"
            value={filters.search}
            onChange={(e) => setFilters({ ...filters, search: e.target.value })}
          />
          <kbd>/</kbd>
        </label>
        <IconButton
          icon="filter"
          label={"筛选范围" + (active ? ` · ${active} 项` : "")}
          active={!!active}
          onClick={onFilters}
        />
        {(filters.search || active > 0) && (
          <IconButton
            icon="close"
            label="清除搜索和范围"
            onClick={() =>
              setFilters({
                ...filters,
                search: "",
                project: "",
                workspace: "",
                category: "",
              })
            }
          />
        )}
      </div>
      {mode === "agents" ? (
        <div className="status-filters">
          <Tabs
            label="Agent 状态"
            value={filters.status}
            onChange={(status) => setFilters({ ...filters, status })}
            items={[
              { id: "attention", label: "关注", count: attention },
              { id: "ready", label: "可继续" },
              { id: "pending", label: "未决" },
              { id: "blocked", label: "受阻" },
              { id: "all", label: "全部" },
            ]}
          />
        </div>
      ) : (
        <div className="collection-meta">
          <span>
            {mode === "records" ? "按提交时间倒序" : "按最近变化倒序"}
          </span>
          <span>当前主机</span>
        </div>
      )}
    </div>
  );
}
function Groups({
  items,
  children,
}: {
  items: Incident[];
  children(item: Incident): React.ReactNode;
}) {
  const groups = new Map<string, { title: string; items: Incident[] }>();
  for (const item of items) {
    const key = JSON.stringify([item.projectId, item.workspaceId]);
    if (!groups.has(key))
      groups.set(key, {
        title: `${item.projectName || "未归属项目"} / ${item.workspaceName || "未归属 Workspace"}`,
        items: [],
      });
    groups.get(key)!.items.push(item);
  }
  return (
    <>
      {[...groups].map(([key, group]) => (
        <section key={key} className="queue-group">
          <h3>
            <Icon name="folder" />
            <span>{group.title}</span>
            <b>{group.items.length}</b>
          </h3>
          {group.items.map(children)}
        </section>
      ))}
    </>
  );
}
/**
 * @aiPurpose Present domain collections with fixed explicit recovery targets.
 * @aiUsage Keep queue selection and scroll mounted while the adjacent inspector opens and closes.
 */
/** @aiPurpose Compose Queue presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Queue({
  controller,
  filters,
  mode,
  selection,
  onSelect,
  checked,
  onToggle,
  ...props
}: QueueProps) {
  const data = controller.data;
  const items = useMemo(
    () =>
      data ? visibleIncidents(data, filters, controller.pendingAgent) : [],
    [data, filters, controller.pendingAgent],
  );
  const records = useMemo(
    () =>
      data
        ? [...data.attempts]
            .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
            .filter((a) => {
              const i = data.incidents.find((i) => i.id === a.incidentId);
              return i
                ? matches(i, filters)
                : !filters.project &&
                    !filters.workspace &&
                    !filters.category &&
                    `${a.agentId} ${a.reason} ${a.operationId}`
                      .toLowerCase()
                      .includes(filters.search.toLowerCase());
            })
        : [],
    [data, filters],
  );
  const cycles = useMemo(
    () =>
      data
        ? [...data.automation.cycles]
            .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
            .filter((c) =>
              matches(
                {
                  ...c,
                  category: data.incidents.find((i) => i.id === c.incidentId)
                    ?.category,
                },
                filters,
              ),
            )
        : [],
    [data, filters],
  );
  return (
    <section className="queue-panel" aria-label="恢复队列">
      <QueueToolbar
        controller={controller}
        filters={filters}
        mode={mode}
        selection={selection}
        onSelect={onSelect}
        checked={checked}
        onToggle={onToggle}
        {...props}
      />
      <div className="queue-scroll" data-testid="queue-scroll">
        {!data ? (
          <Empty
            title={
              controller.fetching
                ? "正在读取…"
                : controller.error
                  ? "读取失败"
                  : "等待数据"
            }
          />
        ) : mode === "agents" ? (
          items.length ? (
            <Groups items={items}>
              {(item) => {
                const pending = controller.pendingAgent(item.agentId),
                  status = incidentStatus(item, data, pending),
                  blocked = reasonFor(item, data) || pending;
                return (
                  <div
                    className={
                      "incident-row " +
                      (selection?.kind === "incident" &&
                      selection.id === item.id
                        ? "selected"
                        : "")
                    }
                    key={item.id}
                  >
                    <label className="select-target">
                      <input
                        type="checkbox"
                        aria-label={"选择 " + item.title}
                        checked={checked.some(
                          (t) =>
                            t.incidentId === item.id &&
                            t.revision === item.revision,
                        )}
                        disabled={!!blocked || controller.commandBlocked}
                        onChange={() => onToggle(item)}
                      />
                    </label>
                    <button
                      className="queue-row-main"
                      onClick={() =>
                        onSelect({ kind: "incident", id: item.id })
                      }
                    >
                      <span className={"object-icon " + status.tone}>
                        <Icon name="terminal" />
                      </span>
                      <span className="row-content">
                        <strong>{item.title || item.agentId}</strong>
                        <span className="row-message">
                          {item.blockedReason || item.message || "无错误正文"}
                        </span>
                        <span className="row-meta">
                          {categoryLabels[item.category]}
                          <span>·</span>
                          {item.provider}
                          <span>·</span>
                          {formatTime(item.observedAt)}
                        </span>
                      </span>
                      <span className="row-tail">
                        <Badge {...status}>{status.label}</Badge>
                        <Icon name="arrow" />
                      </span>
                    </button>
                  </div>
                );
              }}
            </Groups>
          ) : (
            <Empty
              title={
                filters.status === "attention"
                  ? "当前无关注项"
                  : "没有匹配的 Agent"
              }
              action={
                <button
                  className="text-action"
                  onClick={() => props.setMode("records")}
                >
                  查看操作记录 <Icon name="arrow" />
                </button>
              }
            />
          )
        ) : mode === "records" ? (
          records.length ? (
            records.map((a) => {
              const item = data.incidents.find((i) => i.id === a.incidentId),
                status = attemptStatus(a);
              return (
                <button
                  key={a.id}
                  className={
                    "record-row " +
                    (selection?.kind === "attempt" && selection.id === a.id
                      ? "selected"
                      : "")
                  }
                  onClick={() => onSelect({ kind: "attempt", id: a.id })}
                >
                  <Icon name={status.icon} className={status.tone} />
                  <span className="row-content">
                    <strong>{item?.title || a.agentId}</strong>
                    <span className="row-meta">
                      {item?.workspaceName || a.incidentId} <span>·</span>{" "}
                      {item?.provider || "—"} <span>·</span>{" "}
                      {formatTime(a.createdAt, true)}
                    </span>
                  </span>
                  <span className="row-tail">
                    <Badge {...status}>{status.label}</Badge>
                    <span className="row-meta">
                      {a.trigger === "automatic" ? "自动" : "人工"}
                    </span>
                  </span>
                </button>
              );
            })
          ) : (
            <Empty title="没有匹配的操作记录" />
          )
        ) : cycles.length ? (
          cycles.map((c) => (
            <button
              key={c.id}
              className={
                "record-row " +
                (selection?.kind === "cycle" && selection.id === c.id
                  ? "selected"
                  : "")
              }
              onClick={() => onSelect({ kind: "cycle", id: c.id })}
            >
              <Icon
                name="recovery"
                className={c.status === "blocked" ? "warning" : "accent"}
              />
              <span className="row-content">
                <strong>{c.title || c.agentId}</strong>
                <span className="row-message">
                  {c.reason || `${c.projectName} / ${c.workspaceName}`}
                </span>
                <span className="row-meta">
                  {c.attemptCount} 次尝试 <span>·</span>{" "}
                  {c.nextAt
                    ? `下次 ${formatTime(c.nextAt, true)}`
                    : formatTime(c.updatedAt, true)}
                </span>
              </span>
              <Badge
                tone={
                  c.status === "blocked"
                    ? "warning"
                    : c.status === "waiting"
                      ? "accent"
                      : "neutral"
                }
                icon="clock"
              >
                {cycleLabels[c.status]}
              </Badge>
            </button>
          ))
        ) : (
          <Empty title="没有匹配的恢复链" />
        )}
      </div>
      <footer className="queue-foot">
        <span>{data ? `扫描 ${data.scannedAgents} 个 Agent` : "尚未读取"}</span>
        <span>
          {data ? formatTime(data.scannedAt) : "—"}{" "}
          <span
            className={
              "status-pixel " + (controller.stale ? "warning" : "accent")
            }
          />
        </span>
      </footer>
    </section>
  );
}
