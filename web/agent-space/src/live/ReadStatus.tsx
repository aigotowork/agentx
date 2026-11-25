import type { AgentResource, RosterCoverage } from '../offline/model';
import type { SpaceLive } from './workbench';

/** @aiPurpose Show the active Office's observed subset and page recovery at its chooser.
 * @aiUsage Supply the published roster, never exact reader subjects; unconfigured pages do not get SDK controls.
 */
export function SpaceReadStatus({ live, roster, rows }: { live: SpaceLive; roster?: RosterCoverage; rows: readonly AgentResource[] }) {
  const disabled = live.connection === 'reset';
  const catalog = live.catalogError && <span className="space-read-status" role="status">{live.kind === 'cloud' ? '项目目录' : '办公室'}读取失败：{live.catalogError} <button type="button" disabled={disabled} onClick={live.retryCatalog}>重试办公室</button></span>;
  if (!roster) return catalog || null;
  if (roster.phase === 'unavailable') return <>{catalog}<span className="space-read-status" role="status">{roster.error ?? '成员列表不可用'}</span></>;
  const count = rows.filter(row => row.ref.record === 'current').length;
  const status = roster.phase === 'error' ? `成员读取失败：${roster.error}`
    : roster.phase === 'loading' ? '正在读取成员'
    : roster.more ? '还有更多' : rows.length ? '已读到最后一页' : '没有成员记录';
  const paging = roster.phase === 'error' ? <button type="button" disabled={disabled} onClick={live.moreMembers}>重试成员</button>
    : roster.more && <button type="button" disabled={disabled || roster.phase === 'loading'} onClick={live.moreMembers}>加载更多成员</button>;
  return <>{catalog}<section className="space-read-status" aria-label="成员覆盖">
    <span role="status">已加载 {count} 位成员 · {rows.length - count} 条历史 · {status}</span>
    {paging}<button type="button" disabled={disabled} onClick={live.refreshMembers}>刷新成员</button>
  </section></>;
}
