import { CloudAdminError, CloudAdminOutcomeUnknown, cloudIncarnationPattern, cloudWorkspaceIdPattern, type CloudControlInput, type CloudControlReceipt } from '@agentx/client';
import type { Session } from './model';

type AdminSession = Extract<Session, { access: 'admin' }>;
type ControlIdentity = Pick<CloudControlInput, 'request_id' | 'generation' | 'host' | 'installation_id'> & {
  kind: CloudControlInput['command']['kind']; workspaceId?: string; target?: { name: string; incarnation: string };
};
export type ControlEntry = { identity: ControlIdentity; original?: CloudControlInput; receipt?: CloudControlReceipt; issue?: string; busy?: boolean };
const terminal = (entry: ControlEntry) => entry.receipt && ['succeeded', 'failed', 'uncertain'].includes(entry.receipt.status);
export const controlPending = (entry: ControlEntry) => !terminal(entry);
export const controlStatus = (entry: ControlEntry) => ({
  queued: '等待主机接收', running: '主机正在执行', succeeded: '已完成', failed: '执行失败', uncertain: '执行结果未确认',
}[entry.receipt?.status ?? 'uncertain']);

const directoryIssues: Record<string, string> = {
  workspace_name_invalid: '工作区名称格式无效。',
  workspace_limit_reached: '主机已达到工作区数量上限。',
  directory_invalid: '工作目录必须是工作区内的相对路径。',
  directory_missing: '工作目录不存在。请检查路径，或选择「目录不存在时创建」；父目录必须已存在。',
  directory_unsafe: '工作目录不是普通目录，或路径中包含符号链接。',
  directory_unavailable: '主机无法访问此工作目录。请检查目录权限。',
  directory_create_failed: '主机无法创建此工作目录。请检查父目录权限。',
};
export const directoryLaunchIssue = (code: string) => Object.hasOwn(directoryIssues, code) ? directoryIssues[code] : undefined;

function identity(input: CloudControlInput): ControlIdentity {
  const { request_id, generation, host, installation_id, command } = input;
  return { request_id, generation, host, installation_id, kind: command.kind,
    workspaceId: command.kind === 'launch' ? command.workspace_id : undefined,
    target: command.kind === 'stop' ? { name: command.name, incarnation: command.incarnation } : undefined };
}
function matches(value: CloudControlReceipt, expected: ControlIdentity) {
  const { command, outcome } = value;
  return value.request_id === expected.request_id && value.host === expected.host && value.generation === expected.generation
    && value.installation_id === expected.installation_id && command.kind === expected.kind
    && (command.kind !== 'launch' || command.workspace_id === expected.workspaceId)
    && (command.kind !== 'stop' || command.name === expected.target?.name && command.incarnation === expected.target?.incarnation)
    && (outcome?.status !== 'workspace_added' || command.kind === 'add_workspace')
    && (outcome?.status !== 'succeeded' || outcome.agent.host === expected.host && (command.kind === 'launch'
      ? outcome.agent.workspace.kind === 'project' && outcome.agent.workspace.workspace_id === expected.workspaceId
      : command.kind === 'stop' && outcome.agent.name === expected.target?.name && outcome.agent.incarnation === expected.target?.incarnation));
}

/** Keep only request identity in browser session storage. The complete original stays in memory for SDK receipt verification. */
export class CloudOperations {
  readonly entries: ControlEntry[] = [];
  private readonly storageKey: string;
  constructor(private session: AdminSession, private changed: () => void, private failed: (error: unknown) => void) {
    this.storageKey = 'agentx.space.namespace.controls:' + JSON.stringify([session.namespace, session.generation]);
    try {
      const stored: unknown = JSON.parse(sessionStorage.getItem(this.storageKey) ?? '[]');
      if (Array.isArray(stored)) for (const value of stored.slice(0, 60)) {
        if (value?.generation === session.generation && typeof value.request_id === 'string'
          && typeof value.host === 'string' && typeof value.installation_id === 'string'
          && (value.kind === 'launch' && typeof value.workspaceId === 'string' && cloudWorkspaceIdPattern.test(value.workspaceId)
            || value.kind === 'stop' && typeof value.target?.name === 'string' && typeof value.target.incarnation === 'string'
              && cloudIncarnationPattern.test(value.target.incarnation)
            || value.kind === 'add_workspace')) this.entries.push({ identity: value });
      }
    } catch { /* The current workbench retains receipts in memory if storage is unavailable. */ }
  }
  private save() {
    try { sessionStorage.setItem(this.storageKey, JSON.stringify(this.entries.slice(0, 60).map(entry => entry.identity))); }
    catch { /* The current workbench retains receipts in memory. */ }
  }
  private accept(entry: ControlEntry, receipt: CloudControlReceipt) {
    if (!matches(receipt, entry.identity)) throw new Error('云端回执与原请求不一致');
    entry.receipt = receipt; entry.issue = undefined;
  }
  async submit(input: CloudControlInput, signal: AbortSignal) {
    const entry: ControlEntry = { identity: identity(input), original: input, busy: true };
    this.entries.unshift(entry);
    this.save(); this.changed();
    try {
      const receipt = await this.session.client.submitControlOperation(this.session.namespace, input, { signal });
      if (!signal.aborted) this.accept(entry, receipt);
    } catch (error) {
      if (signal.aborted || error instanceof CloudAdminOutcomeUnknown) {
        entry.issue = '提交结果未确认，请查询原回执。';
        if (!signal.aborted && input.command.kind !== 'add_workspace') throw new Error(entry.issue);
      } else {
        this.entries.splice(this.entries.indexOf(entry), 1); this.save();
        this.failed(error);
        throw error;
      }
    } finally { entry.busy = false; if (!signal.aborted) this.changed(); }
    return entry;
  }
  async lookup(entry: ControlEntry, signal: AbortSignal) {
    if (entry.busy || signal.aborted) return;
    entry.busy = true;
    try {
      const receipt = await (entry.original
        ? this.session.client.recoverControlOperation(this.session.namespace, entry.original, { signal })
        : this.session.client.getControlOperation(this.session.namespace, entry.identity.request_id, { signal }));
      if (!signal.aborted) this.accept(entry, receipt);
    } catch (error) {
      if (!signal.aborted) {
        this.failed(error);
        entry.issue = error instanceof CloudAdminError && error.status === 404 ? '暂未找到原回执。' : '无法查询原回执。';
      }
    } finally { entry.busy = false; if (!signal.aborted) this.changed(); }
  }
}
