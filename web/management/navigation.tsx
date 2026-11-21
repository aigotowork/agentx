import type { MouseEvent } from 'react';
import { Icon } from './presentation';
import type { Management } from './use-management';
import { managementEndpoint } from './api/client';

export type Destination = 'agents' | 'workspaces' | 'operations';
const destinations: { id: Destination; label: string }[] = [
  { id: 'agents', label: 'Agents' },
  { id: 'workspaces', label: '工作区' },
  { id: 'operations', label: '操作记录' },
];

/** @aiPurpose Read a Management destination from the static host URL. @aiUsage Fall back to Agents for the root and unknown hashes. */
export function destinationAtLocation(): Destination {
  return destinations.find(({ id }) => location.hash.split('?')[0] === `#${id}`)?.id ?? 'agents';
}

/** @aiPurpose Restore the exact browser receipt selected in its destination. @aiUsage Call on first open and on browser history navigation. */
export function receiptAtLocation(): string | undefined {
  if (destinationAtLocation() !== 'operations') return undefined;
  return new URLSearchParams(location.hash.split('?')[1]).get('receipt') ?? undefined;
}

/** @aiPurpose Keep dataset, primary destinations and launch available across the console. @aiUsage Pass resolved dataset state and route callbacks from the root. */
export function ConsoleNavigation({ model, page, pending, navigate, onLaunch }: {
  model: Management; page: Destination; pending: number; navigate: (page: Destination) => void; onLaunch: () => void;
}) {
  const descriptor = model.descriptor;
  const connection = model.error ? (descriptor ? '读取异常' : '未连接') : descriptor ? '已连接' : '连接中';
  const scopeLabel = descriptor?.project || descriptor?.scope || '管理服务';
  const follow = (event: MouseEvent<HTMLAnchorElement>, destination: Destination) => {
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    navigate(destination);
  };
  return <header className="console-chrome app-header">
      <a className="brand" href="#agents" onClick={event => follow(event, 'agents')} aria-label="AgentX 管理台">
        <span className="brand-symbol" aria-hidden="true"><svg viewBox="0 0 32 32" fill="none"><path d="M7 23 16 7l9 16M12 19h8M16 7v7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/></svg></span>
        <strong>AgentX</strong>
      </a>
    <div className="navigation-strip">
      <nav aria-label="管理台导航">{destinations.map(({ id, label }) => <a key={id} href={`#${id}`} aria-current={page === id ? 'page' : undefined} onClick={event => follow(event, id)}>{label}{id === 'operations' && pending > 0 && <span className="count" aria-label={`${pending} 条待处理`}>{pending}</span>}</a>)}</nav>
    </div>
    <details className="scope-control"><summary aria-label={`数据范围：${connection}，${scopeLabel}`} title={`${connection} · ${scopeLabel}`}><span className="connection-dot" data-connection={model.error ? 'error' : descriptor ? 'ready' : 'pending'} /><span className="scope-status">{connection}</span><span className="project-name">{scopeLabel}</span><Icon name="down" /></summary>
      <dl><dt>连接</dt><dd>{connection}</dd><dt>管理端点</dt><dd><code>{managementEndpoint}</code></dd><dt>项目</dt><dd>{descriptor?.project || '未读取'}</dd><dt>数据范围</dt><dd>{descriptor?.scope || '未读取'}</dd><dt>数据代次</dt><dd><code>{descriptor?.generation || '未读取'}</code></dd></dl>
    </details>
    <div className="header-actions"><button className="primary icon-button" disabled={!descriptor} onClick={onLaunch}><Icon name="plus" />启动 Agent</button></div>
  </header>;
}
