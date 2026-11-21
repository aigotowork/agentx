/** Consume the public no-build Page API; the shipped asset has no sibling-checkout runtime dependency. */
import {sceneDebugRegistry} from './scene/debug/registry';
import type {CanvasDescription} from './scene/debug/contracts';
import {createSpaceInspectionProvider, type SpaceInspectionProvider} from './scene/debug/dancingai-provider';

type Lease = {dispose(): void};
// Narrow consumer ports for the browser artifact, not another SDK or execution API.
interface Inspection {register(provider: SpaceInspectionProvider): Lease; subscribe(listener: (event: {type: string}) => void): () => void}
interface BrowserPage {mount(options: {endpoint: string; frames: false}): Lease & {ready: Promise<void>; open(): void; suspend(): void}}
export interface SpaceInspection extends Lease {setVisible(visible: boolean): void}

export function mountSpaceInspection(): SpaceInspection {
  const browser = Reflect.get(window, 'DancingAI') as BrowserPage | undefined;
  if (!browser) {console.error('DancingAI browser asset was not loaded'); return {dispose() {}, setVisible() {}};}
  const mount = browser.mount({endpoint: document.querySelector<HTMLMetaElement>('meta[name="dancingai-endpoint"]')?.content || '/dancingai/', frames: false});
  // Keep the public inspection provider registered while its floating UI is idle.
  // The product header explicitly opens it, away from ordinary composer actions.
  mount.suspend();
  let visible = false, ready = false;
  let disposed = false, href = location.href;
  let stopRegistry = () => {}, stopInspection = () => {};
  const registrations = new Map<string, Lease>();
  const clear = () => {for (const lease of registrations.values()) lease.dispose(); registrations.clear();};
  function sync() {
    if (disposed) return;
    const inspection = (Reflect.get(window, 'dancingai') as {inspection?: Inspection} | undefined)?.inspection;
    if (!inspection) return;
    if (href !== location.href) {clear(); href = location.href;}
    const result = sceneDebugRegistry.facade.list({kind: 'canvases'});
    if (!result.ok) throw new Error(result.error.message);
    const canvases = result.value as CanvasDescription[];
    for (const [id, lease] of registrations) if (!canvases.some(canvas => canvas.id === id)) {lease.dispose(); registrations.delete(id);}
    for (const canvas of canvases) if (!registrations.has(canvas.id)) registrations.set(canvas.id,
      inspection.register(createSpaceInspectionProvider(sceneDebugRegistry.facade, canvas)));
  }
  function changed() {try {sync();} catch (error) {console.error('Agent Space inspection registration failed', error);}}
  void mount.ready.then(() => {
    if (disposed) return;
    ready = true;
    if (visible) mount.open(); else mount.suspend();
    const inspection = (Reflect.get(window, 'dancingai') as {inspection: Inspection}).inspection;
    stopInspection = inspection.subscribe(event => {if (event.type === 'disposed') queueMicrotask(changed);});
    stopRegistry = sceneDebugRegistry.subscribe(changed);
    window.addEventListener('agentx:location-changed', changed);
    window.addEventListener('popstate', changed);
    window.addEventListener('hashchange', changed);
    changed();
  }).catch(error => console.error('DancingAI workspace failed to mount', error));
  return {setVisible(value) {
    if (disposed) return;
    visible = value;
    if (ready) { if (visible) mount.open(); else mount.suspend(); }
  }, dispose() {
    if (disposed) return; disposed = true;
    stopRegistry(); stopInspection(); clear(); mount.dispose();
    window.removeEventListener('agentx:location-changed', changed);
    window.removeEventListener('popstate', changed); window.removeEventListener('hashchange', changed);
  }};
}
