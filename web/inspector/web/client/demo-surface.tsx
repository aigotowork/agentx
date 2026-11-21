/** Historical fixture surfaces load only after the explicit source=demo selection. */
import { useMemo } from 'react';
import type { PluginSurfaceProps } from '../../host/contracts';
import chunk from '../../plugin/client/surfaces';
type Route = { screenId: string; params?: Record<string, string> };
/** @aiPurpose Retain historical Inspector fixture scenarios without initializing them on the live path.
 * @aiUsage Lazy-load only for source=demo; caller owns route and presentation state. */
export default function DemoSurface({ route, open, props }: { route: Route; open: (route: Route) => void; props: PluginSurfaceProps }) {
  const surfaces = useMemo(() => chunk().createSurfaces({
    openScreen: open,
    playAudio: async (source: { base64: string; mimeType: string }) => { await new Audio(`data:${source.mimeType};base64,${source.base64}`).play(); },
  } as unknown as Parameters<ReturnType<typeof chunk>['createSurfaces']>[0]), [open]);
  const Surface = surfaces[route.screenId] ?? surfaces.observatory;
  return <Surface {...props} />;
}
