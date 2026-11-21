/** @aiPurpose Confirm fixed recovery targets and edit revision-bound automation independently of browsing. */
import { useMemo, useState } from "react";
import type {
  Incident,
  ManageInput,
  Policy,
  RecoveryCycle,
  RecoveryList,
  Target,
} from "../../shared/contracts";
import { CONTINUE_PROMPT } from "../../../plugin/shared/domain/recovery";
import { Badge, Button, CopyValue, Icon, Modal } from "../../../uiux/systems/recovery/recovery.web";
import { categoryLabels, reasonFor, type Filters } from "./model";
import type { RecoveryController } from "./use-recovery";
export interface ContinueDraft {
  operationId: string;
  items: Incident[];
  targets: Target[];
}
/**
 * @aiPurpose Confirm exact recovery targets and show classified command errors.
 * @aiUsage Confirm fixed incident revisions and the canonical prompt; never infer targets from current filters.
 */
/** @aiPurpose Compose ContinueDialog presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ContinueDialog({
  draft,
  controller,
  onClose,
}: {
  draft: ContinueDraft;
  controller: RecoveryController;
  onClose(): void;
}) {
  const [busy, setBusy] = useState(false);
  const data = controller.data;
  const invalid = draft.targets.some((t) => {
    const item = data?.incidents.find((i) => i.id === t.incidentId);
    return (
      !item ||
      item.revision !== t.revision ||
      reasonFor(item, data!) ||
      controller.pendingAgent(item.agentId)
    );
  });
  return (
    <Modal
      title={`继续 ${draft.items.length} 个 Agent`}
      onClose={onClose}
      busy={busy}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            取消
          </Button>
          <Button
            variant="primary"
            icon="run"
            busy={busy}
            disabled={
              controller.commandBlocked || invalid || draft.items.length > 500
            }
            onClick={async () => {
              setBusy(true);
              const done = await controller.execute({
                kind: "continue",
                input: {
                  operationId: draft.operationId,
                  targets: draft.targets,
                },
              });
              setBusy(false);
              if (done) onClose();
            }}
          >
            确认继续 {draft.items.length} 项
          </Button>
        </>
      }
    >
      <div className="confirm-targets">
        {draft.items.map((item) => (
          <div className="confirm-target" key={item.id}>
            <Icon name="terminal" />
            <div>
              <strong>{item.title || item.agentId}</strong>
              <span>
                {item.projectName} / {item.workspaceName}
              </span>
            </div>
          </div>
        ))}
      </div>
      <p className="consequence">向以上原会话发送固定跟进指令。</p>
      <details className="disclosure">
        <summary>查看跟进正文</summary>
        <p className="fixed-prompt">{CONTINUE_PROMPT}</p>
      </details>
      <details className="disclosure">
        <summary>本次操作与固定目标</summary>
        <CopyValue value={draft.operationId} />
        <ul className="technical-list">
          {draft.targets.map((t) => (
            <li key={t.incidentId}>
              <code>
                {t.incidentId} · r{t.revision}
              </code>
            </li>
          ))}
        </ul>
      </details>
      {invalid && (
        <div className="inline-state warning">
          <Icon name="warning" />
          <span>目标已变化或有未决操作。关闭后重新核验。</span>
        </div>
      )}
      {draft.items.length > 500 && (
        <div className="inline-state warning">最多选择 500 个目标。</div>
      )}
      {controller.writeError && (
        <p className="form-error" role="alert">
          {controller.writeError}
        </p>
      )}
    </Modal>
  );
}
interface ScopeOption {
  id: string;
  name: string;
}
function ScopeChoices({
  label,
  options,
  values,
  onChange,
  disabled,
}: {
  label: string;
  options: ScopeOption[];
  values: string[];
  onChange(values: string[]): void;
  disabled?: boolean;
}) {
  const [search, setSearch] = useState("");
  const known = new Set(options.map((o) => o.id));
  const all = [
    ...options,
    ...values
      .filter((id) => !known.has(id))
      .map((id) => ({ id, name: `${id} · 目录未返回` })),
  ];
  const visible = all.filter((o) =>
    `${o.name} ${o.id}`.toLowerCase().includes(search.toLowerCase()),
  );
  return (
    <details className="scope-picker">
      <summary>
        <span>{label}</span>
        <span>{values.length ? `${values.length} 项` : "全部"}</span>
      </summary>
      <div className="scope-picker-content">
        <div className="scope-search">
          <input
            aria-label={"搜索" + label}
            type="search"
            placeholder={"搜索" + label}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          {values.length > 0 && (
            <Button
              variant="quiet"
              disabled={disabled}
              onClick={() => onChange([])}
            >
              全部
            </Button>
          )}
        </div>
        <div className="scope-options">
          {visible.map((o) => (
            <label key={o.id} className="scope-option">
              <input
                type="checkbox"
                checked={values.includes(o.id)}
                disabled={disabled}
                onChange={() =>
                  onChange(
                    values.includes(o.id)
                      ? values.filter((x) => x !== o.id)
                      : [...values, o.id],
                  )
                }
              />
              <span>{o.name}</span>
            </label>
          ))}
          {!visible.length && <span className="muted">无匹配</span>}
        </div>
      </div>
    </details>
  );
}
/**
 * @aiPurpose Edit the current browser collection scope.
 * @aiUsage Edit view filters only; automatic policy scope belongs to PolicyDialog.
 */
/** @aiPurpose Compose FilterDialog presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function FilterDialog({
  filters,
  data,
  onChange,
  onClose,
}: {
  filters: Filters;
  data: RecoveryList;
  onChange(v: Filters): void;
  onClose(): void;
}) {
  const catalog = data.automation.catalog;
  return (
    <Modal
      title="筛选当前视图"
      onClose={onClose}
      footer={
        <>
          <Button
            variant="quiet"
            onClick={() =>
              onChange({ ...filters, project: "", workspace: "", category: "" })
            }
          >
            清除范围
          </Button>
          <Button variant="primary" onClick={onClose}>
            完成
          </Button>
        </>
      }
    >
      <label className="form-field">
        <span>项目</span>
        <select
          value={filters.project}
          onChange={(e) => onChange({ ...filters, project: e.target.value })}
        >
          <option value="">全部项目</option>
          {catalog.projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      <label className="form-field">
        <span>Workspace</span>
        <select
          value={filters.workspace}
          onChange={(e) => onChange({ ...filters, workspace: e.target.value })}
        >
          <option value="">全部 Workspace</option>
          {catalog.workspaces.map((w) => (
            <option key={w.id} value={w.id}>
              {w.name}
            </option>
          ))}
        </select>
      </label>
      <label className="form-field">
        <span>异常类别</span>
        <select
          value={filters.category}
          onChange={(e) => onChange({ ...filters, category: e.target.value })}
        >
          <option value="">全部类别</option>
          {Object.entries(categoryLabels).map(([id, name]) => (
            <option key={id} value={id}>
              {name}
            </option>
          ))}
        </select>
      </label>
    </Modal>
  );
}
/**
 * @aiPurpose Edit and explicitly save a versioned automatic recovery policy.
 * @aiUsage Keep a local policy draft and original revision until explicit save or discard.
 */
/** @aiPurpose Compose PolicyDialog presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function PolicyDialog({
  controller,
  onClose,
}: {
  controller: RecoveryController;
  onClose(): void;
}) {
  const policy = controller.data!.automation.policy,
    [draft, setDraft] = useState<Policy>(() => ({
      ...policy,
      projectIds: [...policy.projectIds],
      workspaceIds: [...policy.workspaceIds],
    })),
    [busy, setBusy] = useState(false);
  const changed = JSON.stringify(policy) !== JSON.stringify(draft),
    conflict = policy.revision !== draft.revision,
    invalid =
      !Number.isInteger(draft.scanIntervalSeconds) ||
      draft.scanIntervalSeconds < 30 ||
      draft.scanIntervalSeconds > 300 ||
      draft.projectIds.length > 500 ||
      draft.workspaceIds.length > 500;
  return (
    <Modal
      title="自动恢复策略"
      onClose={onClose}
      busy={busy}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            取消
          </Button>
          <Button
            icon="check"
            variant="primary"
            busy={busy}
            disabled={
              controller.commandBlocked || !changed || conflict || invalid
            }
            onClick={async () => {
              setBusy(true);
              const done = await controller.execute({
                kind: "configure",
                input: draft,
              });
              setBusy(false);
              if (done) onClose();
            }}
          >
            确认保存并应用
          </Button>
        </>
      }
    >
      <label className="toggle-setting">
        <span>
          <strong>自动恢复</strong>
          <small>5 分钟 → 20 分钟 → 每小时</small>
        </span>
        <input
          type="checkbox"
          role="switch"
          aria-label="自动恢复"
          checked={draft.enabled}
          disabled={busy}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
        />
      </label>
      <label className="form-field interval-field">
        <span>
          扫描间隔 <small>30–300 秒</small>
        </span>
        <input
          aria-label="扫描间隔秒数"
          type="number"
          min={30}
          max={300}
          step={1}
          value={draft.scanIntervalSeconds}
          disabled={busy}
          onChange={(e) =>
            setDraft({ ...draft, scanIntervalSeconds: Number(e.target.value) })
          }
        />
      </label>
      <div className="section-title">
        <h3>自动范围</h3>
        <Badge>r{draft.revision}</Badge>
      </div>
      <ScopeChoices
        label="项目"
        options={controller.data!.automation.catalog.projects}
        values={draft.projectIds}
        onChange={(projectIds) => setDraft({ ...draft, projectIds })}
        disabled={busy}
      />
      <ScopeChoices
        label="Workspace"
        options={controller.data!.automation.catalog.workspaces}
        values={draft.workspaceIds}
        onChange={(workspaceIds) => setDraft({ ...draft, workspaceIds })}
        disabled={busy}
      />
      <p className="action-note">空选为全部；项目与 Workspace 取交集。</p>
      {changed && (
        <p className="consequence">
          {draft.enabled
            ? "保存后，主机将按此策略自动向范围内原 Agent 发送跟进。"
            : "保存后关闭自动发送。"}{" "}
          已交给宿主的跟进无法撤回。
        </p>
      )}
      {conflict && (
        <div className="inline-state warning">
          <span>策略已更新，草稿保留。</span>
          <Button
            variant="quiet"
            onClick={() =>
              setDraft({
                ...policy,
                projectIds: [...policy.projectIds],
                workspaceIds: [...policy.workspaceIds],
              })
            }
          >
            载入当前策略
          </Button>
        </div>
      )}
      {invalid && (
        <p className="form-error">间隔须为 30–300 秒，单类范围最多 500 项。</p>
      )}
      {controller.writeError && (
        <p className="form-error" role="alert">
          {controller.writeError}
        </p>
      )}
    </Modal>
  );
}
/**
 * @aiPurpose Confirm a single recovery chain management action.
 * @aiUsage Confirm one chain action by stable cycle ID; a transport failure must not auto retry.
 */
/** @aiPurpose Compose ManageDialog presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ManageDialog({
  cycle,
  controller,
  onClose,
}: {
  cycle: RecoveryCycle;
  controller: RecoveryController;
  onClose(): void;
}) {
  const [action, setAction] = useState<ManageInput["action"]>(
      cycle.status === "paused" ? "resume" : "pause",
    ),
    [busy, setBusy] = useState(false);
  const live = controller.data?.automation.cycles.find(
      (c) => c.id === cycle.id,
    ),
    changed =
      !live ||
      live.status !== cycle.status ||
      live.updatedAt !== cycle.updatedAt;
  const options =
    cycle.status === "paused"
      ? [
          ["resume", "恢复退避安排"],
          ["stop", "停止此链"],
        ]
      : [
          ["pause", "暂停此链"],
          ["stop", "停止此链"],
        ];
  return (
    <Modal
      title="管理恢复链"
      onClose={onClose}
      busy={busy}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            取消
          </Button>
          <Button
            variant={action === "stop" ? "danger" : "primary"}
            busy={busy}
            disabled={
              controller.commandBlocked ||
              changed ||
              ["stopped", "recovered"].includes(cycle.status)
            }
            onClick={async () => {
              setBusy(true);
              const done = await controller.execute({
                kind: "manage",
                input: { cycleId: cycle.id, action },
              });
              setBusy(false);
              if (done) onClose();
            }}
          >
            确认
            {action === "pause"
              ? "暂停"
              : action === "resume"
                ? "恢复安排"
                : "停止"}
          </Button>
        </>
      }
    >
      <div className="confirm-target">
        <Icon name="recovery" />
        <div>
          <strong>{cycle.title || cycle.agentId}</strong>
          <span>
            {cycle.projectName} / {cycle.workspaceName}
          </span>
        </div>
      </div>
      <div className="manage-choices">
        {options.map(([value, label]) => (
          <label key={value} className="choice-row">
            <input
              type="radio"
              name="chain-action"
              checked={action === value}
              disabled={busy}
              onChange={() => setAction(value as ManageInput["action"])}
            />
            <span>{label}</span>
          </label>
        ))}
      </div>
      <p className="consequence">
        {action === "resume"
          ? "允许按当前策略重新安排。"
          : "阻止此链后续排队发送。"}{" "}
        已交给宿主的跟进无法撤回。
      </p>
      <details className="disclosure">
        <summary>恢复链 ID</summary>
        <CopyValue value={cycle.id} />
      </details>
      {changed && (
        <p className="form-error">恢复链已变化，请关闭后重新核验。</p>
      )}
      {controller.writeError && (
        <p className="form-error" role="alert">
          {controller.writeError}
        </p>
      )}
    </Modal>
  );
}
