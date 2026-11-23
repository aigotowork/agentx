import { CloudAdminError, CloudAdminOutcomeUnknown, type CloudControlInput, type CloudControlReceipt } from '@agentx/client';
import type { Session } from './model';

type AdminSession = Extract<Session, { access: 'admin' }>;
type ControlIdentity = Pick<CloudControlInput, 'request_id' | 'generation' | 'host' | 'installation_id'> & {
  kind: 'launch' | 'stop'; target?: { name: string; incarnation: string };
};
export type ControlEntry = { identity: ControlIdentity; original?: CloudControlInput; receipt?: CloudControlReceipt; issue?: string; busy?: boolean };
const terminal = (entry: ControlEntry) => entry.receipt && ['succeeded', 'failed', 'uncertain'].includes(entry.receipt.status);
export const controlPending = (entry: ControlEntry) => !terminal(entry);
export const controlStatus = (entry: ControlEntry) => ({
  queued: '等待主机接收', running: '主机正在执行', succeeded: '已完成', failed: '执行失败', uncertain: '执行结果未确认',
}[entry.receipt?.status ?? 'uncertain']);

const directoryIssues: Record<string, string> = {
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
    target: command.kind === 'stop' ? { name: command.name, incarnation: command.incarnation } : undefined };
}
function matches(value: CloudControlReceipt, expected: ControlIdentity) {
  return value.request_id === expected.request_id && value.host === expected.host && value.generation === expected.generation
    && value.installation_id === expected.installation_id && value.command.kind === expected.kind
    && (value.command.kind !== 'stop' || value.command.name === expected.target?.name && value.command.incarnation === expected.target?.incarnation);
}

/** Keep only request identity in browser session storage. The complete original stays in memory for SDK receipt verification. */
export class CloudOperations {
  readonly entries: ControlEntry[] = [];
  private readonly storageKey: string;
  constructor(private session: AdminSession, private changed: () => void, private failed: (error: unknown) => void) {
    this.storageKey = 'agentx.space.cloud.controls:' + JSON.stringify([session.scope, session.generation]);
    try {
      const stored: unknown = JSON.parse(sessionStorage.getItem(this.storageKey) ?? '[]');
      if (Array.isArray(stored)) for (const value of stored.slice(0, 60)) {
        if (value?.generation === session.generation && typeof value.request_id === 'string'
          && typeof value.host === 'string' && typeof value.installation_id === 'string'
          && ['launch', 'stop'].includes(value.kind)) this.entries.push({ identity: value });
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
      const receipt = await this.session.client.submitControlOperation(this.session.scope, input, { signal });
      if (!signal.aborted) this.accept(entry, receipt);
    } catch (error) {
      if (signal.aborted || error instanceof CloudAdminOutcomeUnknown) {
        entry.issue = '提交结果未确认，请查询原回执。';
        if (!signal.aborted) throw new Error(entry.issue);
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
        ? this.session.client.recoverControlOperation(this.session.scope, entry.original, { signal })
        : this.session.client.getControlOperation(this.session.scope, entry.identity.request_id, { signal }));
      if (!signal.aborted) this.accept(entry, receipt);
    } catch (error) {
      if (!signal.aborted) {
        this.failed(error);
        entry.issue = error instanceof CloudAdminError && error.status === 404 ? '暂未找到原回执。' : '无法查询原回执。';
      }
    } finally { entry.busy = false; if (!signal.aborted) this.changed(); }
  }
}
