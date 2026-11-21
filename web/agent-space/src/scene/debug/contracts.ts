/** Serializable Canvas discovery/call contracts. Opaque IDs are scoped to a mounted canvas instance. */
import type { Point, Camera, Viewport } from "../geometry";
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type DebugCode = "invalid_arguments" | "no_scene" | "unknown_canvas" | "no_frame" | "unknown_object" | "unsupported_action" | "unavailable_action" | "action_failed" | "capture_failed" | "disposed" | "frame_timeout";
export type Result<T> = { ok: true; value: T } | { ok: false; error: { code: DebugCode; message: string } };
export const success = <T>(value: T): Result<T> => ({ ok: true, value });
export const failure = (code: DebugCode, message: string): Result<never> => ({ ok: false, error: { code, message } });
/** Copy only on explicit reads, never every frame; no consumer receives a runtime reference. */
export function detached<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
export interface Rect extends Point { width: number; height: number }
export type ObjectKind = "room" | "zone" | "actor" | "human" | "furniture" | "prop" | "effect";
export interface Geometry {
  anchor: { world: Point; canvas: Point; client: Point; page: Point; backing: Point } | null;
  bounds: { world: Rect; canvas: Rect; client: Rect; page: Rect; backing: Rect } | null;
  boundsMeaning: string;
  dom: { client: Rect; page: Rect } | null;
}
export interface DebugObject {
  id: string; kind: ObjectKind; label: string; parentId: string | null; relatedIds: string[];
  geometry: Geometry;
  visibility: { filtered: boolean; drawn: boolean; canvasIntersection: "inside" | "partial" | "outside" | "unknown"; reason: string | null };
  rendering: "sprite" | "room-bitmap" | "room-composite" | "model-only" | "aggregate";
  actionIds: string[];
  details: { [key: string]: Json };
}
export interface Parameter {
  type: "number" | "string" | "boolean";
  required: boolean;
  description: string;
  minimum?: number; maximum?: number; enum?: readonly string[];
}
export interface ActionDescriptor {
  id: string; purpose: string; scope: "canvas" | "object"; targetKinds: readonly ObjectKind[];
  parameters: Record<string, Parameter>; result: { meaning: string; fields: Record<keyof Invocation, { type: string; meaning: string }> }; effects: readonly string[];
  availability: { available: boolean; reason: string | null };
}
export interface FrameStamp {
  canvasId: string; frameId: number; inputRevision: number; presentedAt: number;
  elapsed: number; still: boolean; paused: boolean; camera: Camera; viewport: Viewport; selectedKey: string | null;
}
export interface CanvasDescription {
  id: string; label: string; state: "awaiting-frame" | "presented";
  lastFrame: FrameStamp | null;
}
export interface CanvasMetrics {
  backing: { width: number; height: number };
  client: Rect; page: Rect;
  cssToBacking: Point; canvasToClient: Point;
}
export interface Snapshot { frame: FrameStamp; canvas: CanvasMetrics; objects: DebugObject[] }
export interface Capture { frame: FrameStamp; canvas: CanvasMetrics; mimeType: "image/png"; dataUrl: string; includes: "canvas-only" }
export interface Invocation {
  requestId: string; accepted: true; actionId: string; objectId: string | null;
  afterFrame: number; afterInputRevision: number; presentation: "not-confirmed";
}
export interface WaitRequest { canvasId: string; afterFrame: number; afterInputRevision?: number; timeoutMs?: number }
export interface InvokeRequest { canvasId: string; actionId: string; objectId?: string; args?: Record<string, unknown> }
export interface ListRequest { kind: "canvases" | "objects" | "actions"; canvasId?: string; objectId?: string; objectKind?: ObjectKind }
export interface MethodDescriptor {
  parameters: Record<string, Parameter | { type: "object"; required: boolean; description: string }>;
  result: string;
}
export interface InterfaceDescription {
  name: string; version: number; methods: Record<string, MethodDescriptor>;
  model: { objectKinds: readonly ObjectKind[]; relations: string; identity: string };
  coordinates: Record<string, string>; observation: string; errors: readonly DebugCode[];
}
export interface SceneDebugFacade {
  describe(): Result<InterfaceDescription>;
  list(request: ListRequest): Result<(CanvasDescription | DebugObject | ActionDescriptor)[]>;
  inspect(request: { canvasId: string; objectId: string }): Result<DebugObject>;
  snapshot(request: { canvasId: string }): Result<Snapshot>;
  invoke(request: InvokeRequest): Result<Invocation>;
  capture(request: { canvasId: string }): Result<Capture>;
  waitForFrame(request: WaitRequest): Promise<Result<FrameStamp>>;
}
/** Registry knows discovery/call only; current scene rendering stays with its concrete adapter. */
export interface CanvasAdapter {
  describe(): CanvasDescription;
  objects(): Result<DebugObject[]>;
  actions(objectId?: string): Result<ActionDescriptor[]>;
  snapshot(): Result<Snapshot>;
  invoke(request: InvokeRequest): Result<Invocation>;
  capture(): Result<Capture>;
  waitForFrame(request: WaitRequest): Promise<Result<FrameStamp>>;
  dispose(): void;
}
export interface SceneDebugControls {
  zoom(factor: number): void;
  panorama(): void;
  pause(paused: boolean): void;
}
export const OBJECT_KINDS: readonly ObjectKind[] = ["room", "zone", "actor", "human", "furniture", "prop", "effect"];

/** The same primitive parameter promises validate method requests and action arguments. */
export function validate(value: unknown, parameters: Record<string, Parameter>): string | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return "Expected an argument object";
  const record = value as Record<string, unknown>;
  for (const key of Object.keys(record)) if (!Object.hasOwn(parameters, key)) return `Unknown argument: ${key}`;
  for (const [name, parameter] of Object.entries(parameters)) {
    const item = record[name];
    if (item === undefined && !parameter.required) continue;
    if (typeof item !== parameter.type) return `${name} must be ${parameter.type}`;
    if (typeof item === "number" && (!Number.isFinite(item) || parameter.minimum !== undefined && item < parameter.minimum || parameter.maximum !== undefined && item > parameter.maximum)) return `${name} is outside its finite range`;
    if (parameter.enum && !parameter.enum.includes(item as string)) return `${name} is not a supported value`;
  }
  return null;
}
