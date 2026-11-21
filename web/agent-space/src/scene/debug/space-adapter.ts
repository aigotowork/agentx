/** Current Space adapter: observes actual presentations and invokes only explicit existing callback owners. */
import type { SceneInput, SceneFrame, HallPropAnchor } from "../runtime/contracts";
import type { HallActor } from "../model";
import type { Point } from "../geometry";
import { sceneObjects } from "./objects";
import type { SpaceGameWorld } from "../../../uiux/systems/space/components/presentation";
import { detached, failure, success, validate, type ActionDescriptor, type CanvasAdapter, type CanvasDescription, type CanvasMetrics, type Capture, type DebugObject, type FrameStamp, type Invocation, type InvokeRequest, type Parameter, type Result, type SceneDebugControls, type Snapshot, type WaitRequest } from "./contracts";

export interface SceneCallbacks {
  actor(actor: HallActor, point: Point): void;
  human(): void;
  prop(prop: HallPropAnchor): void;
  light?(roomKey: string): void;
  pan(dx: number, dy: number): void;
  controls?: SceneDebugControls;
}
const numeric = (description: string, minimum: number, maximum: number): Parameter => ({ type: "number", required: true, description, minimum, maximum });
const resultFields: ActionDescriptor["result"]["fields"] = {
  requestId: { type: "string", meaning: "Mount-local invocation identity" },
  accepted: { type: "boolean:true", meaning: "Existing callback returned without throwing" },
  actionId: { type: "string", meaning: "Invoked registered action ID" },
  objectId: { type: "string|null", meaning: "Exact discovered target, or null for a canvas action" },
  afterFrame: { type: "integer", meaning: "Require a later presented frame ID to observe subsequent drawing" },
  afterInputRevision: { type: "integer", meaning: "Require a later committed input revision to observe callback-driven state" },
  presentation: { type: "string:not-confirmed", meaning: "Acceptance never asserts presentation or intended outcome" },
};
const actionResult = (meaning: string): ActionDescriptor["result"] => ({ meaning, fields: resultFields });
const ACTIONS: readonly Omit<ActionDescriptor, "availability">[] = [
  { id: "object.activate", purpose: "Activate the existing actor/human/prop interaction", scope: "object", targetKinds: ["actor", "human", "prop"], parameters: {}, result: actionResult("Callback accepted, not a presentation or delivery receipt"), effects: ["Actor: select exact conversation", "Human: toggle listed-message feed", "Prop: resource callback or local room lamp"] },
  { id: "camera.pan", purpose: "Pan the existing world camera", scope: "canvas", targetKinds: [], parameters: { dx: numeric("Horizontal Canvas CSS pixels", -10000, 10000), dy: numeric("Vertical Canvas CSS pixels", -10000, 10000) }, result: actionResult("Pan callback accepted; observe camera in a later input/frame"), effects: ["Existing camera translation only"] },
  { id: "camera.zoom", purpose: "Zoom around the existing viewport center", scope: "canvas", targetKinds: [], parameters: { factor: numeric("Relative zoom factor; owner retains actual zoom limits", .1, 10) }, result: actionResult("Zoom callback accepted; actual resulting zoom is owner-constrained"), effects: ["Existing camera zoom and centered translation"] },
  { id: "camera.panorama", purpose: "Run the existing panorama control", scope: "canvas", targetKinds: [], parameters: {}, result: actionResult("Panorama callback accepted"), effects: ["Existing overview camera", "Close selection and directory, collapse reader"] },
  { id: "animation.pause", purpose: "Set decorative animation pause through its existing owner", scope: "canvas", targetKinds: [], parameters: { paused: { type: "boolean", required: true, description: "Desired decorative pause state" } }, result: actionResult("Pause callback accepted; verify paused/still in a later frame"), effects: ["Existing decorative pause; no native Agent state change"] },
];
interface Presented { sample: SceneFrame; stamp: FrameStamp; game?: SpaceGameWorld }
interface Waiter { request: WaitRequest; finish(result: Result<FrameStamp>): void }

export class SpaceDebugAdapter implements CanvasAdapter {
  private disposed = false;
  private latest: SceneInput | null = null;
  private callbacks: SceneCallbacks | null = null;
  private readonly revisions = new WeakMap<SceneInput, number>();
  private inputRevision = 0;
  private frameId = 0;
  private requestId = 0;
  private presented: Presented | null = null;
  private readonly waiters = new Set<Waiter>();

  constructor(private readonly id: string, private readonly surface: HTMLCanvasElement, private readonly dom: (id: string) => HTMLElement | undefined) {}

  update(input: SceneInput, callbacks: SceneCallbacks): void {
    if (this.disposed) return;
    this.latest = input;
    this.callbacks = callbacks;
    this.revisions.set(input, ++this.inputRevision);
  }

  /** Called only after Canvas drawing and semantic anchor synchronization complete in the same task. */
  present(sample: SceneFrame, game?: SpaceGameWorld): void {
    if (this.disposed) return;
    const stamp: FrameStamp = {
      canvasId: this.id, frameId: ++this.frameId, inputRevision: this.revisions.get(sample.input) ?? 0,
      presentedAt: Date.now(), elapsed: sample.elapsed, still: sample.still, paused: sample.input.paused,
      camera: { ...sample.input.camera }, viewport: { ...sample.input.viewport }, selectedKey: sample.input.selected?.key ?? null,
    };
    this.presented = { sample, stamp, game };
    for (const waiter of [...this.waiters]) if (this.matches(waiter.request, stamp)) waiter.finish(success(detached(stamp)));
  }

  describe(): CanvasDescription {
    return detached({ id: this.id, label: this.latest?.scene.art === "studio" ? "AgentX Catalog Studio" : "AgentX pixel office", state: this.presented ? "presented" : "awaiting-frame", lastFrame: this.presented?.stamp ?? null });
  }

  objects(): Result<DebugObject[]> {
    const frame = this.readFrame();
    if (!frame.ok) return frame;
    return success(detached(sceneObjects(frame.value.sample, this.metrics(frame.value.sample.input), this.dom, frame.value.game)));
  }

  actions(objectId?: string): Result<ActionDescriptor[]> {
    const frame = this.readFrame();
    if (!frame.ok) return frame;
    let object: DebugObject | undefined;
    if (objectId !== undefined) {
      const found = this.findObject(objectId);
      if (!found.ok) return found;
      object = found.value;
    }
    return success(detached(ACTIONS.map(action => ({ ...action, availability: this.availability(action, object) }))));
  }

  snapshot(): Result<Snapshot> {
    const frame = this.readFrame();
    if (!frame.ok) return frame;
    const canvas = this.metrics(frame.value.sample.input);
    return success(detached({ frame: frame.value.stamp, canvas, objects: sceneObjects(frame.value.sample, canvas, this.dom, frame.value.game) }));
  }

  capture(): Result<Capture> {
    const frame = this.readFrame();
    if (!frame.ok) return frame;
    try {
      const dataUrl = this.surface.toDataURL("image/png");
      if (!dataUrl.startsWith("data:image/png;base64,")) return failure("capture_failed", "Canvas returned no PNG pixels (possibly zero backing size)");
      return success(detached({ frame: frame.value.stamp, canvas: this.metrics(frame.value.sample.input), mimeType: "image/png", dataUrl, includes: "canvas-only" }));
    } catch (error) { return failure("capture_failed", error instanceof Error ? error.message : "Canvas PNG encoding failed"); }
  }

  invoke(request: InvokeRequest): Result<Invocation> {
    const frame = this.readFrame();
    if (!frame.ok) return frame;
    const action = ACTIONS.find(item => item.id === request.actionId);
    if (!action) return failure("unsupported_action", "Action is not registered by this scene");
    const error = validate(request.args ?? {}, action.parameters);
    if (error) return failure("invalid_arguments", error);
    if (action.scope === "canvas" && request.objectId !== undefined) return failure("invalid_arguments", "Canvas action does not take an object target");
    let object: DebugObject | undefined;
    if (request.objectId !== undefined) {
      const found = this.findObject(request.objectId);
      if (!found.ok) return found;
      object = found.value;
    }
    const availability = this.availability(action, object);
    if (!availability.available) return failure("unavailable_action", availability.reason ?? "Action is unavailable");
    const receipt: Invocation = {
      requestId: `${this.id}:request-${++this.requestId}`, accepted: true, actionId: action.id, objectId: object?.id ?? null,
      afterFrame: frame.value.stamp.frameId, afterInputRevision: this.inputRevision, presentation: "not-confirmed",
    };
    try {
      this.perform(action.id, object, request.args ?? {});
      return success(receipt);
    } catch (error) { return failure("action_failed", error instanceof Error ? error.message : "Existing callback failed"); }
  }

  waitForFrame(request: WaitRequest): Promise<Result<FrameStamp>> {
    if (this.disposed) return Promise.resolve(failure("disposed", "Canvas mount was disposed"));
    const stamp = this.presented?.stamp;
    if (stamp && this.matches(request, stamp)) return Promise.resolve(success(detached(stamp)));
    return new Promise(resolve => {
      const waiter: Waiter = { request: { ...request }, finish: result => {
        clearTimeout(timer);
        this.waiters.delete(waiter);
        resolve(result);
      } };
      const timer = setTimeout(() => waiter.finish(failure("frame_timeout", "No matching presentation before timeout; scene may be hidden, unchanged or disposed")), request.timeoutMs ?? 2000);
      this.waiters.add(waiter);
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const waiter of [...this.waiters]) waiter.finish(failure("disposed", "Canvas mount was disposed before presentation"));
    this.latest = null;
    this.callbacks = null;
    this.presented = null;
  }

  private readFrame(): Result<Presented> {
    if (this.disposed) return failure("disposed", "Canvas mount was disposed");
    return this.presented ? success(this.presented) : failure("no_frame", "Canvas is mounted but no frame has been presented");
  }

  private findObject(id: string): Result<DebugObject> {
    const result = this.objects();
    if (!result.ok) return result;
    const found = result.value.find(object => object.id === id);
    return found ? success(found) : failure("unknown_object", "Object ID is absent from this canvas's last presented frame");
  }

  private availability(action: Omit<ActionDescriptor, "availability">, object?: DebugObject): ActionDescriptor["availability"] {
    let reason: string | null = null;
    if (!this.latest || this.latest !== this.presented?.sample.input) reason = "Committed input is awaiting presentation";
    else if (!this.latest.viewport.width || !this.latest.viewport.height) reason = "Scene viewport has no drawable size";
    else if (action.scope === "object") {
      if (!object) reason = "Discover an object and supply objectId";
      else if (!action.targetKinds.includes(object.kind)) reason = "This object kind has no activation callback";
      else if (!object.visibility.drawn || object.visibility.filtered || object.visibility.canvasIntersection === "outside" || object.details.domTargetHidden === true) reason = "Object is filtered or outside the interactive view";
      else if (object.details.assetId === "scene-lantern" && !this.callbacks?.light) reason = "Scene owner did not supply a light control";
      else if (object.kind === "actor" && !this.latest.scene.actors.some(actor => actor.key === object.details.key)) reason = "Exact actor identity is no longer current";
    } else if (action.id !== "camera.pan" && !this.callbacks?.controls) reason = "Scene owner did not supply this control";
    return { available: !reason, reason };
  }

  private perform(id: string, object: DebugObject | undefined, args: Record<string, unknown>): void {
    const callbacks = this.callbacks!, input = this.latest!;
    if (id === "object.activate" && object) {
      if (object.kind === "actor") {
        const actor = input.scene.actors.find(actor => actor.key === object.details.key)!;
        callbacks.actor(actor, { ...object.geometry.anchor!.world });
      } else if (object.kind === "human") callbacks.human();
      else if (object.kind === "prop" && object.details.assetId === "scene-lantern") callbacks.light!(object.details.roomKey as string);
      else if (object.kind === "prop") {
        const anchor = object.details.anchor as { x: number; y: number };
        const prop = input.props.find(prop => prop.id === object.details.anchorId && prop.x === anchor.x && prop.y === anchor.y);
        if (!prop) throw new Error("Prop is no longer current");
        callbacks.prop(prop);
      }
    } else if (id === "camera.pan") callbacks.pan(args.dx as number, args.dy as number);
    else if (id === "camera.zoom") callbacks.controls!.zoom(args.factor as number);
    else if (id === "camera.panorama") callbacks.controls!.panorama();
    else if (id === "animation.pause") callbacks.controls!.pause(args.paused as boolean);
  }

  private matches(request: WaitRequest, frame: FrameStamp): boolean {
    return frame.frameId > request.afterFrame && (request.afterInputRevision === undefined || frame.inputRevision > request.afterInputRevision);
  }

  private metrics(input: SceneInput): CanvasMetrics {
    const box = this.surface.getBoundingClientRect();
    const scroll = { x: typeof window === "undefined" ? 0 : window.scrollX, y: typeof window === "undefined" ? 0 : window.scrollY };
    return {
      backing: { width: this.surface.width, height: this.surface.height },
      client: { x: box.x, y: box.y, width: box.width, height: box.height },
      page: { x: box.x + scroll.x, y: box.y + scroll.y, width: box.width, height: box.height },
      cssToBacking: { x: input.viewport.width ? this.surface.width / input.viewport.width : 0, y: input.viewport.height ? this.surface.height / input.viewport.height : 0 },
      canvasToClient: { x: input.viewport.width ? box.width / input.viewport.width : 0, y: input.viewport.height ? box.height / input.viewport.height : 0 },
    };
  }
}
