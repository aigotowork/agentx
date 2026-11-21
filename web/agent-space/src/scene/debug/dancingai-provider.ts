/** Project the existing presented-frame API; never expose private renderer/runtime state. */
import {sceneObjectSources} from './source-locations';
import type {SceneDebugFacade, CanvasDescription, DebugObject, Json, Result} from './contracts';

function read<T>(result: Result<T>): T {
  if (!result.ok) throw Object.assign(new Error(result.error.message), {code: result.error.code});
  return result.value;
}
const json = (value: unknown): Json => JSON.parse(JSON.stringify(value)) as Json;
const ROOT = '$canvas';

export function createSpaceInspectionProvider(api: SceneDebugFacade, canvas: CanvasDescription) {
  const source = `agent-space:${canvas.id}`;
  const summary = (object: DebugObject) => ({id: object.id, label: object.label, type: `AgentSpace/${object.kind}`});
  function snapshot() {
    const result = api.snapshot({canvasId: canvas.id});
    if (!result.ok && result.error.code === 'no_frame') return undefined;
    return read(result);
  }
  function captureScene() {
    const capture = read(api.capture({canvasId: canvas.id}));
    return {dataUrl: capture.dataUrl, scope: 'scene' as const,
      capturedAt: new Date(capture.frame.presentedAt).toISOString(),
      gaps: ['Canvas-only PNG; DOM labels, readers and DancingAI overlays are excluded.']};
  }
  return {
    protocolVersion: 1 as const, id: source, label: `Agent Space · ${canvas.label}`,
    isCurrent: () => {
      const result = api.list({kind: 'canvases'});
      return result.ok && result.value.some(item => item.id === canvas.id);
    },
    // Use the same presented geometry as inspection; do not dispatch business clicks.
    hitTest(point: {x: number; y: number}) {
      const frame = snapshot();
      if (!frame) return;
      const contains = (box: {x: number; y: number; width: number; height: number} | null | undefined) => !!box && box.width > 0 && box.height > 0 && point.x >= box.x && point.y >= box.y && point.x <= box.x + box.width && point.y <= box.y + box.height;
      if (!contains(frame.canvas.client)) return;
      const priority = (kind: DebugObject['kind']) => ['actor', 'human', 'prop'].includes(kind) ? 3 : kind === 'effect' ? 2 : kind === 'furniture' ? 1 : 0;
      const candidates = frame.objects.filter(object => object.visibility.drawn && object.visibility.canvasIntersection !== 'outside' && (contains(object.geometry.bounds?.client) || contains(object.geometry.dom?.client)));
      candidates.sort((a, b) => priority(b.kind) - priority(a.kind) || (a.geometry.bounds?.client.width ?? Infinity) * (a.geometry.bounds?.client.height ?? Infinity) - (b.geometry.bounds?.client.width ?? Infinity) * (b.geometry.bounds?.client.height ?? Infinity));
      return candidates[0]?.id ?? ROOT;
    },
    list(limit: number) {
      const frame = snapshot();
      const items = [{id: ROOT, label: canvas.label, type: 'AgentSpaceCanvas'}, ...(frame?.objects.map(summary) ?? [])];
      return {items: items.slice(0, limit), truncated: items.length > limit};
    },
    inspect(id: string) {
      const frame = snapshot();
      const object = frame?.objects.find(item => item.id === id);
      if (id !== ROOT && !object) return;
      const actions = api.list({kind: 'actions', canvasId: canvas.id, ...(object ? {objectId: id} : {})});
      const relations = (frame?.objects ?? []).filter(item => (item.parentId ?? ROOT) === id)
        .map(item => ({type: 'contains', target: {source, id: item.id}}));
      for (const related of object?.relatedIds ?? []) relations.push({type: 'related', target: {source, id: related}});
      const details=object?.details??{};
      const data: Json = {
        activity: details.behavior??details.activity??(object?object.kind:'canvas'),
        animation: details.animation??(object?.kind==='effect'?json(details):null),
        state: json({status:details.status??null,selected:details.selected??null,open:details.open??null,unhandledMessages:details.unhandledMessages??null}),
        position: details.sampled??(object?.geometry.anchor?.world?json(object.geometry.anchor.world):null),
        canvasId: canvas.id, objectId: object?.id ?? null,
        frame: frame ? json(frame.frame) : null,
        observation: object ? json(object) : {state: frame ? 'presented' : 'awaiting-frame'},
        actions: actions.ok ? json(actions.value.filter(action => object || 'scope' in action && action.scope === 'canvas')) : [],
        actionError: actions.ok ? null : json(actions.error)};
      return {...(object ? summary(object) : {id: ROOT, label: canvas.label, type: 'AgentSpaceCanvas'}),
        sources:sceneObjectSources(object),data, relations, revision: frame ? String(frame.frame.frameId) : undefined,
        gaps: [object ? 'Semantic observation from the last presented frame, not a mutable scene instance.' : 'Native access returns the existing agentxSceneDebug facade; use canvasId with its methods.',
          'Action acceptance does not confirm presentation. Use waitForFrame and inspect the resulting state.'],
        capabilities: frame ? {captureScene, bounds: () => {
          const current = snapshot();
          return id === ROOT ? current?.canvas.client : current?.objects.find(item => item.id === id)?.geometry.bounds?.client;
        }} : {},
      };
    },
    // The scene API deliberately returns detached semantic records. Only its existing
    // public facade is a native object; snapshots must not masquerade as live actors.
    getObject(id: string) {return id === ROOT ? api : undefined;},
  };
}
export type SpaceInspectionProvider = ReturnType<typeof createSpaceInspectionProvider>;
