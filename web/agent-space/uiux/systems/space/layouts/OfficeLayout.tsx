/** Scene shell adapted from web/office/layouts/OfficeLayout.tsx. */
import type { ReactNode } from "react";
/** @aiPurpose Keep the original warm office shell around both presentations and shared overview controls.
 * @aiUsage Pass mode-independent header/context; children owns only the selected presentation's body.
 */
export function OfficeLayout({ header, context, contextId, children, studio, studioSample, studioReader, windowed, compactReading, filtersOpen }: { header: ReactNode; context: ReactNode; contextId?: string; children: ReactNode; studio?: boolean; studioSample?: boolean; studioReader?: "scene" | "chat"; windowed?: boolean; compactReading?: boolean; filtersOpen?: boolean }) {
  return <div className="office-workspace agent-space" data-space-art={studio ? "studio" : undefined} data-space-supply={studioSample ? "studio" : undefined} data-studio-reader={studioReader} data-window-workspace={windowed || undefined} data-window-reading={compactReading || undefined} data-window-filters={filtersOpen || undefined}><header className="office-hud">{header}</header><div className="space-overview" id={contextId}>{context}</div><main className="office-stage" id="main-content">{children}</main></div>;
}
