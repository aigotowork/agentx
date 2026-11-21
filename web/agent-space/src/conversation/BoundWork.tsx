import type { FileBindingView } from '../../../management/api/native';
import type { AgentResource } from '../offline/model';

const statuses: Record<string, string> = {
  not_started: '未开始', ongoing: '进行中', completed: '已完成', paused: '已暂停',
  blocked: '受阻', failed: '失败', cancelled: '已取消', unknown: '未识别', undeclared: '未声明',
};
function statusLabel(binding: FileBindingView): string {
  return binding.status ? statuses[binding.status] ?? binding.status : '未提供';
}
function observedAt(value: number | null | undefined): string {
  return value == null ? '无成功读取记录' : new Date(value * 1000).toLocaleString();
}
function WorkFile({ binding }: { binding: FileBindingView }) {
  const kind = binding.schema === 'task' ? 'Task' : binding.schema === 'delivery' ? 'Delivery' : '文件';
  const filename = binding.path.split(/[\\/]/).pop() || binding.path;
  const label = binding.description || filename;
  const declaredStatus = statusLabel(binding);
  const state = binding.diagnostic ? `上次成功快照 · ${declaredStatus}` : `文件状态 · ${declaredStatus}`;
  return <article className="bound-work-file">
    <div className="bound-work-row"><small>{kind}</small><strong>{label}</strong><span>{state}</span></div>
    {binding.diagnostic && <p role="status">读取异常 · {binding.diagnostic}</p>}
    <details><summary>文件与观测详情</summary><dl>
      <dt>路径</dt><dd><code>{binding.path}</code></dd>
      <dt>原始状态</dt><dd>{binding.raw_status ?? '未声明'}</dd>
      <dt>最近检查</dt><dd>{observedAt(binding.checked_at)}</dd>
      <dt>最近成功读取</dt><dd>{observedAt(binding.last_success_at)}</dd>
    </dl></details>
  </article>;
}

/** @aiPurpose Show file-declared work alongside the selected exact Agent's runtime status.
 * @aiUsage Place inside conversation history; native bindings are read-only, disclosure never sends or changes selection.
 * @aiBehavior Diagnostics label retained snapshots; absent supply and an empty observed association list remain distinct.
 */
export function BoundWork({ agent }: { agent: AgentResource }) {
  const bindings = agent.fileBindings;
  if (bindings === undefined) return null;
  const count = bindings.length;
  const issues = bindings.filter(binding => binding.diagnostic).length;
  const summary = `关联工作 · ${count}${issues ? ` · ${issues} 项读取异常` : ''}`;
  const runtime = agent.nativeStatus ?? '未提供';
  const record = agent.ref.record === 'stopped' ? '停止记录' : '当前实例';
  return <details className="bound-work" key={agent.id}>
    <summary>{summary}</summary>
    <div className="bound-work-content"><small>Agent 运行状态 · {runtime} · {record}</small>
      {count ? bindings.map(binding => <WorkFile key={binding.path} binding={binding} />) : <span>当前观测未提供此精确实例的文件绑定</span>}
    </div>
  </details>;
}
