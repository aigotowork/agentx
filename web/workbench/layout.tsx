/** @aiPurpose Express the selected resource matrix and contextual-detail spatial contract. */
import type { ReactNode } from "react";
import { useEffect, useState } from "react";

interface WorkspaceLayoutProps { connection: ReactNode; navigation: ReactNode; collection: ReactNode; detail: ReactNode; context?: ReactNode }

/**
 * @aiPurpose Arrange fixed connection, resource navigation, collection and local detail.
 * @aiUsage Pass semantic Blocks through the named slots; omit detail to expand the collection.
 * @aiLayout CSS owns scrolling and narrow reflow; this layout owns no domain state.
 */
export function ResourceWorkspaceLayout({ connection, navigation, collection, detail, context }: WorkspaceLayoutProps) {
  const [navigationOpen, setNavigationOpen] = useState(false);
  useEffect(() => {
    const dismiss = (event: KeyboardEvent) => { if (event.key !== "Escape") return; const disclosures = [...document.querySelectorAll<HTMLDetailsElement>("details[open]")]; const last = disclosures.at(-1); if (last) { last.open = false; last.querySelector<HTMLElement>("summary")?.focus(); } };
    const outside = (event: PointerEvent) => { for (const disclosure of document.querySelectorAll<HTMLDetailsElement>(".connection-details[open], .collection-settings[open], .row-menu[open]")) if (!disclosure.contains(event.target as Node)) disclosure.open = false; };
    document.addEventListener("keydown", dismiss); document.addEventListener("pointerdown", outside); return () => { document.removeEventListener("keydown", dismiss); document.removeEventListener("pointerdown", outside); };
  }, []);
  const bodyClass = detail ? "workspace-body has-detail" : "workspace-body";
  return <div className="workspace"><header className="connection-bar"><button className="navigation-toggle action" aria-expanded={navigationOpen} aria-controls="workspace-navigation" onClick={() => setNavigationOpen(value => !value)}>资源</button>{connection}</header><div className={bodyClass}><aside id="workspace-navigation" className={navigationOpen ? "resource-navigation navigation-open" : "resource-navigation"} onKeyDown={event => { if (event.key === "Escape") { setNavigationOpen(false); document.querySelector<HTMLButtonElement>(".navigation-toggle")?.focus(); } }}><button className="navigation-toggle action" onClick={() => setNavigationOpen(false)}>返回工作区</button>{navigation}</aside><main className="workspace-main"><section className="resource-collection" aria-label="资源集合">{collection}</section>{detail && <section className="selected-resource" aria-label="选中资源">{detail}</section>}</main></div>{context}</div>;
}
