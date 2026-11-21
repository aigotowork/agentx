/** @aiPurpose Keep raw evidence, delivery facts, audit and contextual actions with one subject. */
import { useState } from "react";
import type {
  Attempt,
  RecoveryCycle,
  RecoveryList,
  Incident,
} from "../../shared/contracts";
import {
  Badge,
  Button,
  CopyValue,
  Empty,
  Field,
  Icon,
  IconButton,
  Tabs,
} from "../../../uiux/systems/recovery/recovery.web";
import {
  attemptStatus,
  categoryLabels,
  cycleLabels,
  formatTime,
  incidentStatus,
  latestAttempt,
  reasonFor,
  selectedIncident,
  type Selection,
} from "./model";
import type { RecoveryController, WriteRecord } from "./use-recovery";
/**
 * @aiPurpose Display authoritative delivery and observation evidence.
 * @aiUsage Show delivery and observation separately; raw reason and IDs remain locally expandable.
 */
/** @aiPurpose Compose AttemptRecord presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function AttemptRecord({ attempt }: { attempt: Attempt }) {
  const status = attemptStatus(attempt);
  return (
    <article className="attempt-card">
      <header>
        <Badge {...status}>{status.label}</Badge>
        <time>{formatTime(attempt.updatedAt, true)}</time>
      </header>
      <dl className="fact-grid">
        <Field label="投递事实">
          {attempt.delivery?.status === "acknowledged"
            ? "宿主已接受"
            : attempt.delivery?.status === "not_sent"
              ? "明确未发送"
              : attempt.delivery?.status === "unknown"
                ? "效果未知"
                : "尚无投递证据"}
        </Field>
        <Field label="触发">
          {attempt.trigger === "automatic" ? "自动" : "人工"}
        </Field>
      </dl>
      {attempt.phase === "accepted" && (
        <div className="inline-state info">
          <Icon name="clock" />
          <span>等待执行观察</span>
        </div>
      )}
      {attempt.phase === "unknown" && (
        <div className="inline-state unknown">
          <Icon name="unknown" />
          <span>保留原操作，不补发</span>
        </div>
      )}
      <details className="disclosure">
        <summary>观察依据与操作 ID</summary>
        {attempt.reason && <p className="record-reason">{attempt.reason}</p>}
        <dl className="record-identities">
          <Field label="operationId">
            <CopyValue value={attempt.operationId} />
          </Field>
          <Field label="attemptId">
            <CopyValue value={attempt.id} />
          </Field>
          {attempt.delivery && (
            <>
              <Field label="messageId">
                <CopyValue value={attempt.delivery.messageId} />
              </Field>
              <Field label="回执依据">
                <code>{attempt.delivery.reasonCode}</code>
              </Field>
            </>
          )}
          {attempt.guardReason && (
            <Field label="核验拒绝">
              <code>{attempt.guardReason}</code>
            </Field>
          )}
          {attempt.previousAttemptId && (
            <Field label="前序尝试">
              <CopyValue value={attempt.previousAttemptId} />
            </Field>
          )}
          <Field label="轮次观察">
            <code>
              {attempt.baselineGeneration} →{" "}
              {attempt.observedGeneration ?? "未知"}
            </code>
          </Field>
        </dl>
      </details>
    </article>
  );
}
/**
 * @aiPurpose Display the persisted client command identity and receipt state.
 * @aiUsage Display a persisted browser request until an authoritative receipt reconnects its identity.
 */
/** @aiPurpose Compose LocalRecord presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function LocalRecord({ record }: { record: WriteRecord }) {
  return (
    <article className="attempt-card local-record">
      <header>
        <Badge
          tone={
            record.status === "unknown"
              ? "unknown"
              : record.status === "rejected"
                ? "danger"
                : "info"
          }
          icon={record.status === "unknown" ? "unknown" : "clock"}
        >
          {record.status === "unknown"
            ? "请求效果未知"
            : record.status === "sending"
              ? "提交中"
              : record.status === "rejected"
                ? "请求被拒绝"
                : "已收到回执"}
        </Badge>
        <time>{formatTime(record.at, true)}</time>
      </header>
      <p className="record-reason">{record.message}</p>
      <CopyValue value={record.id} label="复制原请求 ID" />
      <details className="disclosure">
        <summary>
          {record.kind === "continue"
            ? "固定发送目标"
            : record.kind === "manage"
              ? "链管理请求"
              : "策略请求"}
        </summary>
        <pre className="error-body">
          {JSON.stringify(record.input, null, 2)}
        </pre>
      </details>
      {record.status === "unknown" && (
        <p className="action-note">保留原请求，仅刷新核查。</p>
      )}
    </article>
  );
}
/**
 * @aiPurpose Present a recovery chain and its available management actions.
 * @aiUsage Present one recovery chain and request a separate confirmation before management writes.
 */
/** @aiPurpose Compose ChainBlock presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ChainBlock({
  cycle,
  onManage,
  disabled,
}: {
  cycle: RecoveryCycle;
  onManage(c: RecoveryCycle): void;
  disabled: boolean;
}) {
  return (
    <section className="chain-block">
      <header>
        <h3>恢复链</h3>
        <Badge
          icon="clock"
          tone={cycle.status === "blocked" ? "warning" : "neutral"}
        >
          {cycleLabels[cycle.status]}
        </Badge>
      </header>
      <dl className="fact-grid">
        <Field label="累计尝试">{cycle.attemptCount}</Field>
        <Field label="下次执行">
          {cycle.nextAt ? formatTime(cycle.nextAt, true) : "无安排"}
        </Field>
        <Field label="退避档位">
          {[5, 20, 60][cycle.retryIndex] ?? 60} 分钟
        </Field>
        <Field label="最近变化">{formatTime(cycle.updatedAt, true)}</Field>
      </dl>
      {cycle.reason && <p className="record-reason">{cycle.reason}</p>}
      <Button
        variant="quiet"
        icon="settings"
        disabled={disabled || ["stopped", "recovered"].includes(cycle.status)}
        onClick={() => onManage(cycle)}
      >
        管理恢复链
      </Button>
    </section>
  );
}
/**
 * @aiPurpose Keep subject evidence and persistent actions together.
 * @aiUsage Inspect the selected domain object; preserve source facts beside the local request barrier.
 */
/** @aiPurpose Compose Detail presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Detail({
  controller,
  selection,
  onBack,
  onContinue,
  onManage,
}: {
  controller: RecoveryController;
  selection: Selection | null;
  onBack(): void;
  onContinue(items: Incident[]): void;
  onManage(c: RecoveryCycle): void;
}) {
  const [tab, setTab] = useState<"facts" | "attempts" | "audit">("facts");
  const data = controller.data;
  if (!data || !selection)
    return (
      <aside className="detail-panel detail-empty">
        <Empty icon="inspector" title="选择一项查看" />
      </aside>
    );
  const item = selectedIncident(selection, data),
    selectedAttempt =
      selection.kind === "attempt"
        ? data.attempts.find((a) => a.id === selection.id)
        : undefined,
    cycle =
      selection.kind === "cycle"
        ? data.automation.cycles.find((c) => c.id === selection.id)
        : data.automation.cycles
            .filter((c) => c.incidentId === item?.id)
            .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))[0];
  const agentId = item?.agentId || selectedAttempt?.agentId || cycle?.agentId,
    title = item?.title || cycle?.title || agentId || "记录已不可用";
  const attempts = data.attempts
    .filter((a) => a.agentId === agentId)
    .sort(
      (a, b) =>
        Number(b.id === selectedAttempt?.id) -
          Number(a.id === selectedAttempt?.id) ||
        b.createdAt.localeCompare(a.createdAt),
    );
  const audits = data.automation.events
    .filter((e) => e.agentId === agentId || (cycle && e.cycleId === cycle.id))
    .sort((a, b) => b.at.localeCompare(a.at));
  const local = controller.records.filter(
    (r) =>
      r.kind === "continue" &&
      r.agentIds?.includes(agentId || "") &&
      ["unknown", "sending", "rejected"].includes(r.status),
  );
  const focusAttempt =
    selectedAttempt || (item ? latestAttempt(item, data) : undefined);
  const pending = !!agentId && controller.pendingAgent(agentId);
  const status =
    selectedAttempt && !pending
      ? attemptStatus(selectedAttempt)
      : item
        ? incidentStatus(item, data, pending)
        : null;
  const blocked = item ? reasonFor(item, data) : "当前没有可恢复的异常";
  const continueDisabled =
    !item || !!blocked || pending || controller.commandBlocked;
  return (
    <aside className="detail-panel" aria-label="恢复详情">
      <header className="detail-heading">
        <IconButton
          className="detail-back"
          label="返回队列"
          icon="back"
          onClick={onBack}
        />
        <span className="detail-object">
          <Icon name="terminal" />
        </span>
        <div className="detail-identity">
          <span className="detail-provider">{item?.provider || "AGENT"}</span>
          <h2>{title}</h2>
        </div>
        {status && <Badge {...status}>{status.label}</Badge>}
      </header>
      <div className="detail-context">
        <Icon name="folder" />
        <span>
          {item
            ? `${item.projectName || "未归属项目"} / ${item.workspaceName || "未归属 Workspace"}`
            : cycle
              ? `${cycle.projectName} / ${cycle.workspaceName}`
              : agentId}
        </span>
      </div>
      <Tabs<"facts" | "attempts" | "audit">
        label="对象详情"
        value={tab}
        onChange={setTab}
        items={[
          { id: "facts", label: "停止事实" },
          {
            id: "attempts",
            label: "尝试",
            count: attempts.length + local.length,
          },
          { id: "audit", label: "审核", count: audits.length },
        ]}
      />
      <div className="detail-body" data-testid="detail-scroll">
        {local
          .filter((r) => r.status === "unknown" || r.status === "sending")
          .map((r) => (
            <LocalRecord key={r.id} record={r} />
          ))}
        {tab === "facts" ? (
          <>
            {focusAttempt && <AttemptRecord attempt={focusAttempt} />}
            <IncidentFacts item={item} />
            {cycle && (
              <ChainBlock
                cycle={cycle}
                onManage={onManage}
                disabled={controller.commandBlocked}
              />
            )}
          </>
        ) : tab === "attempts" ? (
          <>
            {local
              .filter((r) => r.status === "rejected")
              .map((r) => (
                <LocalRecord key={r.id} record={r} />
              ))}
            {attempts.length
              ? attempts.map((a) => <AttemptRecord key={a.id} attempt={a} />)
              : !local.length && <Empty icon="document" title="暂无恢复尝试" />}
          </>
        ) : audits.length ? (
          <ol className="audit-list">
            {audits.map((event) => (
              <li key={event.id}>
                <span className="audit-point" />
                <div>
                  <time>{formatTime(event.at, true)}</time>
                  <strong>{event.kind}</strong>
                  <p>{event.message}</p>
                </div>
              </li>
            ))}
          </ol>
        ) : (
          <Empty icon="document" title="暂无相关审核记录" />
        )}
      </div>
      <footer className="detail-footer">
        {(blocked || pending || controller.stale) && (
          <div className="action-note">
            <Icon
              name={pending ? "unknown" : controller.stale ? "warning" : "info"}
            />
            <span>
              {pending
                ? "原请求待核实，不补发"
                : controller.stale
                  ? "观察已过期，请刷新"
                  : blocked}
            </span>
          </div>
        )}
        <div className="detail-actions">
          <Button
            icon="recovery"
            onClick={() => void controller.refresh()}
            disabled={controller.fetching}
          >
            刷新观察
          </Button>
          {item && (
            <Button
              icon="run"
              variant="primary"
              disabled={continueDisabled}
              onClick={() => onContinue([item])}
            >
              继续原 Agent
            </Button>
          )}
        </div>
      </footer>
    </aside>
  );
}
function IncidentFacts({ item }: { item?: Incident }) {
  return item ? (
    <section className="incident-facts">
      <dl className="fact-grid">
        <Field label="来源">
          {item.source === "hook" ? "轮次终止事件" : "状态扫描"}
        </Field>
        <Field label="发现时间">{formatTime(item.observedAt, true)}</Field>
        <Field label="异常类别">{categoryLabels[item.category]}</Field>
        <Field label="异常状态">
          {item.state === "detected"
            ? "已发现"
            : item.state === "submitted"
              ? "已提交"
              : "已替代"}
        </Field>
      </dl>
      {item.blockedReason && (
        <div className="inline-state warning">
          <Icon name="pause" />
          <span>{item.blockedReason}</span>
        </div>
      )}
      <div className="section-title">
        <h3>原始错误</h3>
        {item.code && <code>{item.code}</code>}
      </div>
      <pre className="error-body">{item.message || "无错误正文"}</pre>
      <details className="disclosure">
        <summary>来源元数据</summary>
        <dl className="record-identities">
          <Field label="Agent ID">
            <CopyValue value={item.agentId} />
          </Field>
          <Field label="Incident ID">
            <CopyValue value={item.id} />
          </Field>
          <Field label="异常版本">
            <code>
              revision {item.revision} · generation {item.generation}
            </code>
          </Field>
          <Field label="turnId">
            {item.turnId ? (
              <CopyValue value={item.turnId} />
            ) : (
              <span>未记录</span>
            )}
          </Field>
          <Field label="恢复基线">
            {item.baselineCaptured ? "已捕获" : "未捕获"}
          </Field>
          <Field label="最近用户消息">
            {formatTime(item.lastUserMessageAt, true)}
          </Field>
        </dl>
      </details>
    </section>
  ) : (
    <div className="inline-state neutral">
      <Icon name="info" />
      <span>当前响应中没有对应异常，保留操作记录。</span>
    </div>
  );
}
