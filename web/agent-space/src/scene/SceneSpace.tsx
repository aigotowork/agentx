/** Scene composition adapted from web/hall/HallHome.tsx without application/runtime writers. */
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { AgentResource, SceneMessage, AgentSpaceSupply } from "../offline/model";
import type { HallLayoutMemory, PopulationSnapshot } from "./population";
import { projectHall, type HallActor } from "./model";
import { focusCamera, overview, recoverReturnAnchor, resizeCamera, worldToScreen, zoomCamera, type Camera, type Point, type Viewport } from "./geometry";
import { hallActorGoal } from "./motion";
import { HallScene, type HallPropAnchor } from "./HallScene";
import { useOverlay } from "../interactions/SpaceInteractions";
import { useAgentCommands } from "../interactions/AgentCommands";
import { HumanMailbox } from "./HumanMailbox";
import { ExpandedObject } from "./ExpandedObject";
import { SceneControls } from "../../uiux/systems/space/space.web";
import { studioEntryCamera } from "./studio-art";
import { roomIsLit, type RoomLights } from "./ambience";
interface ReturnView { camera: Camera; key: string | null; resized?: boolean }
/** Page-owned scene memory survives a presentation unmount, without retaining a second UI tree. */
export interface SceneMemory { camera: Camera; viewport: Viewport; scope: string; returnView: ReturnView | null; expanded: boolean; paused: boolean; studioEntryPending?: boolean; roomLights?: RoomLights }
interface Props {
  memory: { current: SceneMemory | null };
  layoutMemory: HallLayoutMemory; population?: PopulationSnapshot; identityScope: string;
  human?: AgentSpaceSupply["human"]; onHumanNavigate?(id: string): void;
  messages?: readonly SceneMessage[];
  agents: readonly AgentResource[]; names: ReadonlyMap<string, string>; visibleKeys: ReadonlySet<string>;
  selectedId: string | null; reading: boolean; reader: ReactNode; directory: ReactNode;
  panel: "" | "directory" | "mailbox"; onPanel(panel: "" | "directory" | "mailbox"): void;
  scope: string; showEmpty?: boolean; onSelect(id: string): boolean; onClose(): void; onCabinet(): void; onReset(): void;
  studio?: boolean; studioSample?: boolean; awaitStudioRoster?: boolean;
  workspace?: ReactNode;
}
/** @aiPurpose Compose the actual Canvas world, camera, props and object-anchored conversation window.
 * @aiUsage Project scoped resources before query/state dimming; parent owns selection, history and drafts.
 */
export function SceneSpace(props: Props) {
  const stage = useRef<HTMLDivElement>(null), [viewport, setViewport] = useState<Viewport>({ width: 0, height: 0 });
  const [camera, setCamera] = useState<Camera>(() => props.memory.current?.camera ?? { x: 12, y: 12, zoom: .75 });
  const [expanded, setExpanded] = useState(props.memory.current?.expanded ?? false), [paused, setPaused] = useState(props.memory.current?.paused ?? false);
  const [roomLights, setRoomLights] = useState<RoomLights>(() => props.memory.current?.roomLights ?? {});
  function toggleRoomLight(key: string) { setRoomLights(lights => ({ ...lights, [key]: !roomIsLit(lights, key) })); }
  const directory = props.panel === 'directory', humanOpen = props.panel === 'mailbox';
  const setDirectory = (open: boolean) => props.onPanel(open ? 'directory' : '');
  const setHumanOpen = (open: boolean) => props.onPanel(open ? 'mailbox' : '');
  const commands = useAgentCommands(), directoryRoot = useRef<HTMLElement>(null);
  const closeDirectory = useOverlay(directory, () => setDirectory(false), 30, directoryRoot);
  const returnView = useRef<ReturnView | null>(props.memory.current?.returnView ?? null);
  const previousViewport = useRef<Viewport | null>(props.memory.current?.viewport ?? null);
  const previousScope = useRef(props.memory.current?.scope ?? props.scope);
  const studioEntryPending = useRef(props.studio && (props.memory.current?.studioEntryPending ?? true));
  const currentCamera = useRef(camera); currentCamera.current = camera;
  const scene = useMemo(() => projectHall(props.agents, props.names, viewport.width || 900, props.layoutMemory, props.population, props.identityScope, props.studio ? "studio" : undefined), [props.agents, props.names, viewport.width, props.layoutMemory, props.population, props.identityScope, props.studio]);
  const actor = scene.actors.find(item => item.key === props.selectedId);
  const point = actor ? hallActorGoal(actor) : scene.rooms.find(room => room.cabinet.resources.some(resource => resource.id === props.selectedId))?.cabinet.at ?? { x: scene.width / 2, y: 80 };
  const selectedLayout = useRef<{ id: string; home: Point; rest: Point; point: Point } | null>(null);
  useEffect(() => {
    const previous = selectedLayout.current;
    if (!props.workspace && actor && props.reading && previous?.id === actor.key && (previous.home.x !== actor.home.x || previous.home.y !== actor.home.y || previous.rest.x !== actor.rest.x || previous.rest.y !== actor.rest.y)) {
      const dx = (previous.point.x - point.x) * currentCamera.current.zoom, dy = (previous.point.y - point.y) * currentCamera.current.zoom;
      setCamera(value => ({ ...value, x: value.x + dx, y: value.y + dy }));
      if (returnView.current) returnView.current.camera = { ...returnView.current.camera, x: returnView.current.camera.x + dx, y: returnView.current.camera.y + dy };
    }
    selectedLayout.current = actor ? { id: actor.key, home: actor.home, rest: actor.rest, point } : null;
  }, [scene, actor?.key, props.reading]);
  function closeHuman() { setHumanOpen(false); requestAnimationFrame(() => stage.current?.querySelector<HTMLButtonElement>('[data-hall-object="human:bigboss"]')?.focus({ preventScroll: true })); }
  const anchors: HallPropAnchor[] = scene.rooms.filter(room => room.cabinet.resources.length).map(room => ({ id: "cabinet", roomKey: room.key, title: `停止历史 ${room.cabinet.resources.length}`, ...room.cabinet.at }));
  function entryCamera() { return props.studio ? studioEntryCamera(scene, viewport) : overview(scene, { ...viewport, height: Math.max(0, viewport.height - 62) }); }
  function panorama() {
    studioEntryPending.current = false;
    returnView.current = null;
    if (!props.workspace) props.onClose();
    setExpanded(false);
    const fit = Math.max(.2, Math.min(2, (viewport.width - 24) / scene.width, (viewport.height - 74) / scene.height));
    setCamera({ zoom: fit, x: (viewport.width - scene.width * fit) / 2, y: 12 });
  }
  function pan(dx: number, dy: number) { studioEntryPending.current = false; setCamera(value => ({ ...value, x: value.x + dx, y: value.y + dy })); }
  function zoom(factor: number) { studioEntryPending.current = false; setCamera(value => zoomCamera(value, factor, { x: viewport.width / 2, y: viewport.height / 2 })); }
  function focusDirectory() { stage.current?.querySelector<HTMLElement>('.office-camera-controls button[aria-expanded]')?.focus({ preventScroll: true }); }
  function select(selected: HallActor, _point: Point) {
    if (!props.visibleKeys.has(selected.key) || !props.onSelect(selected.key)) focusDirectory();
  }
  useEffect(() => {
    const node = stage.current; if (!node) return;
    const observer = new ResizeObserver(([entry]) => { if (entry) setViewport({ width: entry.contentRect.width, height: entry.contentRect.height }); });
    observer.observe(node); return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!viewport.width || !viewport.height) return;
    const before = previousViewport.current, scopeChanged = previousScope.current !== props.scope;
    const resized = !!before && (before.width !== viewport.width || before.height !== viewport.height);
    previousViewport.current = viewport; previousScope.current = props.scope;
    if (scopeChanged) returnView.current = null;
    if (resized && returnView.current) {
      returnView.current.camera = resizeCamera(returnView.current.camera, before!, viewport);
      returnView.current.resized = true;
    }
    // A pinned listener supplies the first usable room asynchronously. Fit its entry once, unless the user already moved the camera.
    const studioEntry = studioEntryPending.current && !props.awaitStudioRoster && scene.rooms.length > 0;
    if (studioEntry) studioEntryPending.current = false;
    const baseline = scopeChanged || !before || studioEntry ? entryCamera() : currentCamera.current;
    if (props.reading && !props.workspace) {
      returnView.current ??= { camera: baseline, key: props.selectedId };
      if (studioEntry) returnView.current.camera = baseline;
      returnView.current.key = props.selectedId;
      setCamera(focusCamera(point, viewport, baseline));
    } else {
      const saved = returnView.current; returnView.current = null;
      if (saved) setCamera(saved.resized ? recoverReturnAnchor(saved.camera, point, viewport) : saved.camera);
      else if (scopeChanged || !before || studioEntry) setCamera(baseline);
      else if (resized) setCamera(resizeCamera(baseline, before!, viewport));
      setExpanded(false);
      // Filter edits keep their input focus; explicit reader return restores its person.
      const editingFilter = document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLSelectElement;
      if (saved?.key && !editingFilter) requestAnimationFrame(() => requestAnimationFrame(() => {
        const node = stage.current;
        if (!node || returnView.current) return;
        const anchor = [...node.querySelectorAll<HTMLElement>("[data-hall-object]")].find(target => !target.hidden && target.dataset.hallObject === saved.key);
        if (anchor) {
          const box = anchor.getBoundingClientRect(), bounds = node.getBoundingClientRect();
          const x = box.x + box.width / 2, y = box.y + box.height / 2;
          if (x > bounds.left && x < bounds.right && y > bounds.top && y < bounds.bottom) {
            anchor.focus({ preventScroll: true }); return;
          }
        }
        focusDirectory();
      }));
    }
  }, [props.reading, props.selectedId, props.scope, props.awaitStudioRoster, viewport.width, viewport.height]);
  useEffect(() => {
    if (viewport.width && viewport.height) props.memory.current = { camera, viewport, scope: props.scope, returnView: returnView.current, expanded, paused, studioEntryPending: studioEntryPending.current, roomLights };
  }, [camera, viewport, props.scope, props.reading, props.selectedId, props.awaitStudioRoster, expanded, paused, roomLights, props.memory]);
  const selected = props.reading && props.selectedId ? { key: props.selectedId, point } : undefined;
  const readerAnchor = worldToScreen(point, camera);
  const empty = props.visibleKeys.size === 0;
  const debugControls = { zoom, panorama, pause: setPaused };
  return <div ref={stage} className="space-scene-stage"><HallScene studioSample={props.studioSample} humanOpen={humanOpen} debugControls={debugControls} human={props.human} onHuman={() => setHumanOpen(!humanOpen)} messages={props.messages} scene={scene} camera={camera} viewport={viewport} visibleKeys={props.visibleKeys} paused={paused} selected={selected} props={anchors} roomLights={roomLights} onRoomLight={toggleRoomLight} onActor={select} onActorContext={(actor, anchor) => commands?.menu(actor.resource, anchor)} onProp={props.onCabinet} onPan={pan} />
    {humanOpen && <HumanMailbox feed={props.human} onClose={closeHuman} onNavigate={props.onHumanNavigate} />}
    <SceneControls paused={paused} directory={directory} onZoom={zoom} onPanorama={panorama} onPause={() => setPaused(value => !value)} onDirectory={() => setDirectory(!directory)} />
    {props.showEmpty !== false && empty && !props.reading && !directory && <div className={props.studio && !scene.actors.length ? "studio-empty-room" : "scene-empty"}>{props.studio && !scene.actors.length ? <span role="status">暂无成员</span> : <><span>暂无成员或无匹配</span><button type="button" onClick={props.onReset}>清除筛选</button></>}</div>}
    {props.workspace ?? <ExpandedObject title="人物对话" kind="conversation" point={readerAnchor} viewport={viewport} expanded={expanded} visible={props.reading} onExpand={() => setExpanded(value => !value)} onClose={props.onClose}>{props.reader}</ExpandedObject>}
    {directory && (!props.reading || props.workspace) && <section ref={directoryRoot} className="scene-directory-window" data-canvas-workspace={!!props.workspace || undefined} aria-label="场景名册"><header><strong>人物名册</strong><button type="button" aria-label="关闭名册" onClick={closeDirectory}>×</button></header>{props.directory}</section>}
  </div>;
}
