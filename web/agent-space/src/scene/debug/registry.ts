/** A tiny registry for mounted Canvas adapters. No drawing, clock, controller or mutable global state API. */
import { detached, failure, success, validate, OBJECT_KINDS, type CanvasAdapter, type SceneDebugFacade, type Parameter, type Result, type MethodDescriptor, type ListRequest, type InvokeRequest, type WaitRequest } from "./contracts";

const text = (description: string, required = true): Parameter => ({ type: "string", required, description });
const number = (description: string, minimum: number, maximum: number, required = true): Parameter => ({ type: "number", required, description, minimum, maximum });
const canvasParameter = { canvasId: text("Opaque ID returned by list({kind:'canvases'})") };
const listParameters = {
  kind: { ...text("Discovery collection"), enum: ["canvases", "objects", "actions"] },
  canvasId: text("Required for objects/actions", false), objectId: text("Optional action target", false),
  objectKind: { ...text("Optional object kind filter", false), enum: OBJECT_KINDS },
};
const waitParameters = {
  ...canvasParameter, afterFrame: number("Resolve after this presented frame ID", 0, Number.MAX_SAFE_INTEGER),
  afterInputRevision: number("Also require a newer committed input revision", 0, Number.MAX_SAFE_INTEGER, false),
  timeoutMs: number("Bounded timeout, default 2000ms; no polling/RAF", 1, 10000, false),
};
const methodDescriptions: Record<string, MethodDescriptor> = {
  describe: { parameters: {}, result: "Interface version, model, methods, coordinates and error codes" },
  list: { parameters: listParameters, result: "Detached canvas/object/action descriptors with current availability" },
  inspect: { parameters: { ...canvasParameter, objectId: text("Object ID discovered within this canvas") }, result: "One detached semantic object from the last presented frame" },
  snapshot: { parameters: canvasParameter, result: "Last presented frame, current canvas layout metrics and semantic objects" },
  invoke: { parameters: { ...canvasParameter, actionId: text("Discovered action ID"), objectId: text("Required for object actions", false), args: { type: "object", required: false, description: "Arguments defined by the action parameters; default {}" } }, result: "Acceptance receipt; presentation is not confirmed" },
  capture: { parameters: canvasParameter, result: "Actual Canvas PNG data URL and same-task frame/layout metadata; excludes all DOM overlays" },
  waitForFrame: { parameters: waitParameters, result: "Promise of a newer presented frame or explicit timeout/disposal error; compare requested state separately" },
};

export class SceneDebugRegistry {
  private readonly adapters = new Map<string, CanvasAdapter>();
  private readonly listeners = new Set<() => void>();
  subscribe(listener: () => void): () => void {this.listeners.add(listener); return () => this.listeners.delete(listener);}
  private changed(): void {for (const listener of this.listeners) {try {listener();} catch (error) {console.error('Scene registry observer failed', error);}}}
  private serial = 0;
  private readonly retired = new Set<string>();
  private readonly prefix = `canvas-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  readonly facade: SceneDebugFacade;

  constructor() {
    this.facade = Object.freeze<SceneDebugFacade>({
      [Symbol.for('@dancingai/objects/describe/v1')]: () => ({
        type: 'AgentXSceneDebug', description: 'Existing Agent Space public facade. Read canvasId/objectId from DancingAI inspection data; native invoke receipts do not confirm presentation.',
        methods: Object.entries(methodDescriptions).map(([name, method]) => ({name, description: method.result,
          effect: name === 'invoke' ? 'write' : 'read', parameters: name === 'describe' ? [] : [{name: 'request', type: 'object', required: true, fields: method.parameters}],
          returns: {description: method.result}})),
      }),
      describe: () => success(detached({
        name: "agentxSceneDebug", version: 1, methods: methodDescriptions,
        model: { objectKinds: OBJECT_KINDS, relations: "parentId and relatedIds refer to opaque IDs within the same canvas", identity: "Canvas IDs are mount-specific; actor details retain exact native references; furniture has no implied action" },
        coordinates: {
          world: "Scene units before camera", canvas: "Last-frame Canvas-local CSS coordinates after camera",
          client: "Current viewport CSS coordinates", page: "Current document CSS coordinates including scroll",
          backing: "Actual Canvas backing pixels; ratios use frame viewport and backing size",
          bounds: "Sprite image boxes include transparent padding; model zones are geometric bounds; not opaque-pixel hit regions or occlusion tests",
          dom: "Live DOM target rect, measured at read time; may differ from the last presented Canvas while input is pending",
        },
        observation: "waitForFrame uses presentations only. Invocation receipt is not state or presentation confirmation. No-op/hidden scene can time out.",
        errors: ["invalid_arguments", "no_scene", "unknown_canvas", "no_frame", "unknown_object", "unsupported_action", "unavailable_action", "action_failed", "capture_failed", "disposed", "frame_timeout"],
      })),
      list: request => this.list(request),
      inspect: request => this.withAdapter(request, { ...canvasParameter, objectId: text("Object ID") }, adapter => {
        const objects = adapter.objects();
        if (!objects.ok) return objects;
        const object = objects.value.find(item => item.id === request.objectId);
        return object ? success(detached(object)) : failure("unknown_object", "Object is absent from the last presented frame");
      }),
      snapshot: request => this.withAdapter(request, canvasParameter, adapter => adapter.snapshot()),
      capture: request => this.withAdapter(request, canvasParameter, adapter => adapter.capture()),
      invoke: request => this.invoke(request),
      waitForFrame: async request => {
        const error = validate(request, waitParameters);
        if (error) return failure("invalid_arguments", error);
        if (!Number.isInteger(request.afterFrame) || request.afterInputRevision !== undefined && !Number.isInteger(request.afterInputRevision)) return failure("invalid_arguments", "Frame/revision thresholds must be integers");
        const resolved = this.resolve(request.canvasId);
        return resolved.ok ? resolved.value.waitForFrame(request) : resolved;
      },
    });
  }

  mount(create: (id: string) => CanvasAdapter): { id: string; dispose(): void } {
    const id = `${this.prefix}-${++this.serial}`, adapter = create(id);
    this.adapters.set(id, adapter);
    this.changed();
    return { id, dispose: () => {
      if (!this.adapters.delete(id)) return;
      this.changed();
      this.retired.add(id);
      if (this.retired.size > 64) this.retired.delete(this.retired.values().next().value!);
      adapter.dispose();
    } };
  }

  private resolve(id: string): Result<CanvasAdapter> {
    const adapter = this.adapters.get(id);
    if (adapter) return success(adapter);
    return this.adapters.size || this.retired.has(id) ? failure("unknown_canvas", "Canvas ID is unknown or belongs to a disposed mount") : failure("no_scene", "No Canvas scene is mounted");
  }

  private withAdapter<T>(request: { canvasId: string }, parameters: Record<string, Parameter>, read: (adapter: CanvasAdapter) => Result<T>): Result<T> {
    const error = validate(request, parameters);
    if (error) return failure("invalid_arguments", error);
    const resolved = this.resolve(request.canvasId);
    return resolved.ok ? read(resolved.value) : resolved;
  }

  private list(request: ListRequest): ReturnType<SceneDebugFacade["list"]> {
    const error = validate(request, listParameters);
    if (error) return failure("invalid_arguments", error);
    if (request.kind === "canvases") return success(detached([...this.adapters.values()].map(adapter => adapter.describe())));
    if (!request.canvasId) return failure("invalid_arguments", "canvasId is required for objects/actions");
    const resolved = this.resolve(request.canvasId);
    if (!resolved.ok) return resolved;
    if (request.kind === "actions") return resolved.value.actions(request.objectId);
    const objects = resolved.value.objects();
    return objects.ok ? success(objects.value.filter(item => !request.objectKind || item.kind === request.objectKind)) : objects;
  }

  private invoke(request: InvokeRequest): ReturnType<SceneDebugFacade["invoke"]> {
    if (!request || typeof request !== "object" || Array.isArray(request)) return failure("invalid_arguments", "Expected an invocation object");
    const { args, ...outer } = request;
    if (args !== undefined && (!args || typeof args !== "object" || Array.isArray(args))) return failure("invalid_arguments", "args must be an argument object");
    const error = validate(outer, { ...canvasParameter, actionId: text("Action ID"), objectId: text("Action target", false) });
    if (error) return failure("invalid_arguments", error);
    const resolved = this.resolve(request.canvasId);
    return resolved.ok ? resolved.value.invoke({ ...outer, args: args === undefined ? {} : args }) : resolved;
  }
}

/** Default-on global discovery survives an empty/unmounted Space; mounted adapters unregister separately. */
export const sceneDebugRegistry = new SceneDebugRegistry();
if (typeof window !== "undefined") Object.defineProperty(window, "agentxSceneDebug", { value: sceneDebugRegistry.facade, configurable: true });
declare global { interface Window { readonly agentxSceneDebug: SceneDebugFacade } }
