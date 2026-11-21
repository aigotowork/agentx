import type { ReactNode } from 'react';
import './workspace.css';
/** @aiPurpose Arrange existing Inspector live tabs, resource list and selected evidence without losing selection.
 * @aiUsage Supply toolbar, optional resource sidebar and one selected content block; layout owns no domain state. */
export function WorkspaceLayout({ toolbar, sidebar, children }: { toolbar: ReactNode; sidebar?: ReactNode; children: ReactNode }) {
  return <section className="native-inspector"><header>{toolbar}</header><div className={sidebar ? 'native-workspace' : 'native-workspace single'}>{sidebar && <aside aria-label="原生会话">{sidebar}</aside>}<section className="native-content" aria-label="Inspector 内容">{children}</section></div></section>;
}
