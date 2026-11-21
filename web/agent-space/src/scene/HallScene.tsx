/** @aiPurpose Render the portable hall's pixel world and exact-identity accessible actors with one visible animation clock. */
import { useContext, useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { SpaceWorldContext } from "../../uiux/systems/space/components/SpaceTheme";
import { worldToScreen, hallActorGoal, sceneBehavior, HALL_STATE_LABEL, type HallActor, type HallScene as Scene, type Camera, type Viewport, type Point } from "./index";

import type { AgentSpaceSupply, SceneMessage } from "../offline/model";
import { LongPress } from "../interactions/long-press";
import { FactButton, ActorAnchor, SceneObjectLayer } from "../../uiux/systems/space/space.web";
import { adoptedRoleLabel } from "../offline/model";
import { SceneRuntime, browserSceneEnvironment } from "./runtime/runtime";
import { SceneRenderer } from "./runtime/renderer";
import type { HallPropAnchor, SceneFrame, SceneInput } from "./runtime/contracts";
import { sceneDebugRegistry } from "./debug/registry";
import { SpaceDebugAdapter, type SceneCallbacks } from "./debug/space-adapter";
import { objectId } from "./debug/objects";
import type { SceneDebugControls } from "./debug/contracts";
import { loadStudioAssets, type StudioArtSet } from "./studio-assets";
import { STUDIO_ACTOR_SIZE, studioSignal, studioSignalLabel } from "./studio-art";
import { lanternInViewport, roomIsLit, roomLantern, type RoomLights } from "./ambience";
import { spaceGameWorld } from "../../uiux/systems/space/components/presentation";
export type { HallProp, HallPropAnchor } from "./runtime/contracts";

interface Props {
  debugControls?: SceneDebugControls;
  studioSample?: boolean;
  human?: AgentSpaceSupply["human"]; humanOpen?: boolean; onHuman(): void;
  messages?: readonly SceneMessage[];
  scene: Scene; camera: Camera; viewport: Viewport; visibleKeys: ReadonlySet<string>; paused: boolean;
  selected?: { key: string; point: Point }; props: readonly HallPropAnchor[];
  roomLights?: RoomLights; onRoomLight?(key: string): void;
  onActorContext?(actor: HallActor, anchor: HTMLButtonElement): void;
  onActor(actor: HallActor, point: Point): void; onProp(prop: HallPropAnchor): void; onPan(dx: number, dy: number): void;
}
/** @aiPurpose Adapt pure art/motion to canvas and semantic object buttons; all business state stays above this surface.
 * @aiUsage Project the full source roster before filtering. Pausing affects decorative time only.
 */
export function HallScene(props: Props) {
  const root = useRef<HTMLDivElement>(null), canvas = useRef<HTMLCanvasElement>(null);
  const buttons = useRef(new Map<string, HTMLButtonElement>());
  const runtime = useRef<SceneRuntime | null>(null);
  const debug = useRef<SpaceDebugAdapter | null>(null);
  const studio = props.scene.art === "studio";
  const world = useContext(SpaceWorldContext);
  const [studioArt, setStudioArt] = useState<StudioArtSet>();
  const [presentedArt, setPresentedArt] = useState<StudioArtSet>();
  const art = useRef(studioArt); art.current = studioArt;
  const hasArt = studioArt !== undefined;
  const [artStatus, setArtStatus] = useState<"loading" | "ready" | "error">("loading");
  const current = useRef(props); current.current = props;
  const gesture = useRef(new LongPress());
  useEffect(() => () => gesture.current.cancel(), []);
  const pointer = useRef<{ id: number; x: number; y: number } | null>(null);
  useEffect(() => {
    if (!studio) return;
    let mounted = true;
    void loadStudioAssets(world).then(next => {
      if (mounted) setStudioArt(next);
    }, error => {
      if (mounted) { console.error(error); setArtStatus("error"); }
    });
    return () => { mounted = false; };
  }, [studio, world]);
  useEffect(() => {
    if (studio && !hasArt) return;
    const node = root.current, surface = canvas.current;
    if (!node || !surface) return;
    const context = surface.getContext("2d");
    if (!context) return;
    const renderer = new SceneRenderer(node, surface, context, studio ? art.current : undefined);
    const initial = sceneInput(current.current);
    const registration = sceneDebugRegistry.mount(id => {
      const adapter = new SpaceDebugAdapter(id, surface, key => [...node.querySelectorAll<HTMLElement>("[data-scene-object-id]")].find(target => target.dataset.sceneObjectId === key));
      adapter.update(initial, sceneCallbacks(current.current));
      debug.current = adapter;
      return adapter;
    });
    let publishedArt: StudioArtSet | undefined;
    const sceneRuntime = new SceneRuntime(initial, browserSceneEnvironment(), frame => {
      const activeArt = studio ? art.current : undefined;
      if (activeArt && activeArt !== publishedArt) renderer.setStudioArt(activeArt);
      renderer.draw(frame);
      synchronizeActorAnchors(frame, buttons.current);
      debug.current?.present(frame, spaceGameWorld(activeArt?.world));
      if (activeArt && activeArt !== publishedArt && frame.input.viewport.width > 0 && frame.input.viewport.height > 0) {
        publishedArt = activeArt;
        setPresentedArt(activeArt);
        setArtStatus("ready");
      }
    });
    runtime.current = sceneRuntime;
    sceneRuntime.start();
    return () => {
      sceneRuntime.dispose();
      renderer.dispose();
      registration.dispose();
      debug.current = null;
      runtime.current = null;
    };
  }, [studio, hasArt]);
  useEffect(() => {
    const input = sceneInput(props);
    debug.current?.update(input, sceneCallbacks(props));
    runtime.current?.update(input);
  }, [studioArt, props.scene, props.camera, props.viewport, props.visibleKeys, props.paused, props.selected, props.props, props.messages, props.human, props.humanOpen, props.roomLights, props.debugControls, props.onActor, props.onHuman, props.onProp, props.onPan]);
  function startPan(event: PointerEvent<HTMLCanvasElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    pointer.current = { id: event.pointerId, x: event.clientX, y: event.clientY };
  }
  function movePan(event: PointerEvent<HTMLCanvasElement>): void {
    const previous = pointer.current;
    if (!previous || previous.id !== event.pointerId) return;
    props.onPan(event.clientX - previous.x, event.clientY - previous.y);
    pointer.current = { id: event.pointerId, x: event.clientX, y: event.clientY };
  }
  function endPan(): void { pointer.current = null; }
  function renderActor(actor: HallActor, roomKey: string) {
    const controller = actor.resource.role.kind === "adopted" && actor.resource.role.name === "controller";
    const signal = studio ? studioSignal(actor.resource) : null;
    const status = studioSignalLabel(actor.resource);
    const label = [actor.resource.title || "未命名 Agent", HALL_STATE_LABEL[actor.resource.state], signal && status !== HALL_STATE_LABEL[actor.resource.state] ? status : null, actor.resource.source.title].filter(Boolean).join(" · ");
    const position = coordinates(worldToScreen(hallActorGoal(actor), props.camera));
    const bindButton = (node: HTMLButtonElement | null) => {
      if (node) buttons.current.set(actor.key, node);
      else buttons.current.delete(actor.key);
    };
    const select = () => {
      // Placement comes from the current presentation; opening uses only the exact key.
      const latest = current.current;
      const exact = latest.scene.actors.find(member => member.key === actor.key);
      if (root.current && exact && latest.visibleKeys.has(exact.key)) latest.onActor(exact, runtime.current?.point(exact.key) ?? hallActorGoal(exact));
    };
    return <ActorAnchor key={actor.key} facts={<ActorFacts actor={actor} studio={studio} />} controller={controller} label={actor.resource.title || "未命名 Agent"}
      data-scene-object-id={objectId("actor", actor.key)} data-hall-object={actor.key} data-room-key={roomKey} data-status={actor.resource.state} data-behavior={sceneBehavior(actor.resource)} data-studio-signal={signal ?? undefined}
      ref={bindButton} hidden={!props.visibleKeys.has(actor.key)} aria-pressed={props.selected?.key === actor.key}
      aria-label={label} style={position} onClick={event => { const held = gesture.current.consumeClick(); if (!held || event.detail === 0) select(); }}
      onPointerDown={event => { if (event.button === 0 && props.onActorContext) { const anchor = event.currentTarget; if (event.isTrusted) anchor.setPointerCapture(event.pointerId); gesture.current.begin(event.pointerId, event.clientX, event.clientY, () => props.onActorContext?.(actor, anchor)); } }}
      onPointerMove={event => gesture.current.move(event.pointerId, event.clientX, event.clientY)}
      onPointerUp={event => gesture.current.release(event.pointerId)} onPointerCancel={() => gesture.current.cancel()} onLostPointerCapture={event => gesture.current.release(event.pointerId)}
      onContextMenu={event => { if (props.onActorContext) { event.preventDefault(); gesture.current.cancel(); props.onActorContext(actor, event.currentTarget); } }}
      onKeyDown={event => { if (event.key === "ContextMenu" || event.key === "F10" && event.shiftKey) { event.preventDefault(); props.onActorContext?.(actor, event.currentTarget); } }}>
    </ActorAnchor>;
  }
  const sceneStyle = {
    "--office-head-rise": `${(studio ? STUDIO_ACTOR_SIZE.height : 48) * props.camera.zoom}px`,
    "--office-rest-label-width": `${60 * props.camera.zoom}px`,
    ...(studio && { "--studio-actor-width": `${STUDIO_ACTOR_SIZE.width * props.camera.zoom}px` }),
  } as CSSProperties;
  const humanPosition = coordinates(worldToScreen(props.scene.human, props.camera));
  const humanFacts = <><strong>@bigboss · 人类通信端</strong><span>作用域共享 · {props.human?.unhandledCount === undefined ? "待处理数量未提供" : `${props.human.unhandledCount} 封待处理`}</span><span>查看已接受并明确发给 @bigboss 的消息</span></>;
  const mailbox = props.studioSample ? "样例信箱" : "@bigboss";
  return <div ref={root} className="office-scene" style={sceneStyle} aria-label="Agent 大厅场景" data-scene-art={props.scene.art} data-art-status={studio ? artStatus : undefined} data-scene-world={studio ? presentedArt?.world : undefined}>
    <canvas ref={canvas} className="office-canvas" aria-hidden="true" onPointerDown={startPan} onPointerMove={movePan}
      onPointerUp={endPan} onPointerCancel={endPan} onLostPointerCapture={endPan} />
    {studio && artStatus !== "ready" && <div className="studio-art-notice" role={artStatus === "error" ? "alert" : "status"}>{artStatus === "error" ? "工作室画面未加载" : "正在打开工作室…"}</div>}
    {(!studio || hasArt) && <SceneObjectLayer>
      <FactButton className="office-human" data-scene-object-id={objectId("human", "$human:bigboss")} data-hall-object="human:bigboss" style={humanPosition} onClick={props.onHuman}
        aria-label={props.studioSample ? mailbox : "@bigboss 人类通信端"} facts={humanFacts}><span>{mailbox}{!props.studioSample && props.human?.unhandledCount !== undefined ? ` · ${props.human.unhandledCount}` : ""}</span></FactButton>
      {props.scene.rooms.map(room => <span className="hall-room-name" data-room-key={room.key} key={room.key}
        style={coordinates(worldToScreen({ x: room.origin.x + 16, y: room.origin.y - 8 }, props.camera))}>{room.title}</span>)}
      {props.props.map(item => <button type="button" className="office-prop" key={`${item.id}:${item.roomKey ?? `${item.x}:${item.y}`}`}
        data-scene-object-id={objectId("prop", item.id, String(item.x), String(item.y))} data-hall-object={item.id} data-room-key={item.roomKey} style={coordinates(worldToScreen(item, props.camera))} onClick={() => props.onProp(item)}><span>{item.title}</span></button>)}
      {props.onRoomLight && spaceGameWorld(presentedArt?.world) && props.scene.rooms.map(room => {
        const point = worldToScreen(roomLantern(room), props.camera);
        if (!lanternInViewport(point, props.viewport)) return null;
        return <SceneLantern key={room.key} id={objectId("prop", room.key, "lantern")} point={point} lit={roomIsLit(props.roomLights, room.key)} onToggle={() => props.onRoomLight?.(room.key)} />;
      })}
      {props.scene.rooms.flatMap(room => room.actors.map(actor => renderActor(actor, room.key)))}
    </SceneObjectLayer>}
  </div>;
}

function SceneLantern({ id, point, lit, onToggle }: { id: string; point: Point; lit: boolean; onToggle(): void }) {
  return <button className="game-scene-lantern" type="button" data-scene-object-id={id} style={coordinates(point)} aria-label="场景灯" aria-pressed={lit} onClick={onToggle}>
    <span aria-hidden="true">{lit ? "灯亮" : "灯熄"}</span>
  </button>;
}

/** Facts remain React-owned supplied data, independent of decorative animation state. */
function ActorFacts({ actor, studio }: { actor: HallActor; studio: boolean }) {
  const resource = actor.resource, role = resource.role;
  const controller = role.kind === "adopted" && role.name === "controller";
  const roleLabel = `${adoptedRoleLabel(role)}${controller ? " · 控制岗位" : ""}`;
  const status = resource.nativeStatus ?? HALL_STATE_LABEL[resource.state];
  return <>
    <strong>{resource.title}</strong>
    {resource.launchTitle && <span>职责 · {resource.launchTitle}</span>}
    <span>岗位 · {roleLabel}</span>
    {role.kind === "adopted" && <span>{role.description || "岗位说明未提供"}</span>}
    <span>{status}</span>
    {studio && studioSignalLabel(resource) !== status && <span>{studioSignalLabel(resource)}</span>}
    {resource.statusContext && <span>{resource.statusContext}</span>}
    {resource.requestError && <span>模型请求 · {resource.requestError.source} · {resource.requestError.reason}</span>}
    {resource.pendingMessages != null && <span>待接收消息 · {resource.pendingMessages}</span>}
  </>;
}

function coordinates(point: Point): CSSProperties { return { "--object-x": `${point.x}px`, "--object-y": `${point.y}px` } as CSSProperties; }
/** React owns node membership; the same sampled frame drives Canvas and these imperative anchors. */
function synchronizeActorAnchors(frame: SceneFrame, buttons: ReadonlyMap<string, HTMLButtonElement>): void {
  for (const sample of frame.actors) {
    const button = buttons.get(sample.actor.key);
    if (!button) continue;
    button.style.setProperty("--object-x", `${sample.screen.x}px`);
    button.style.setProperty("--object-y", `${sample.screen.y}px`);
    button.hidden = sample.hidden;
  }
}

function sceneInput(props: Props): SceneInput {
  return {
    scene: props.scene, camera: props.camera, viewport: props.viewport, visibleKeys: props.visibleKeys,
    paused: props.paused, selected: props.selected, props: props.props, messages: props.messages,
    humanMessages: props.human?.unhandledCount ?? 0, humanOpen: props.humanOpen, roomLights: props.roomLights,
  };
}

function sceneCallbacks(props: Props): SceneCallbacks {
  return { actor: props.onActor, human: props.onHuman, prop: props.onProp, light: props.onRoomLight, pan: props.onPan, controls: props.debugControls };
}
