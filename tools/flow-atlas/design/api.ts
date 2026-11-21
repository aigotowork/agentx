/** @aiPurpose Describe the larger target APIs while preserving the bounded authoring implementation boundary.
 * @aiUsage Read ../authoring/index.ts for callable source-binding APIs; these interfaces alone do not implement the target.
 */
import type {
  AtlasBundle, AtlasDefinition, Checked, CompiledAtlas, Diagnostic, Id,
  InstancePortRef, LayoutDocument, OperationPort, Point, Scenario, SchemaExportContext, Snapshot, TypePortRef, ValueSchema, ViewScope,
} from './contracts.js';

/** Draft authoring preserves unresolved references as diagnostics until compilation. */
export interface AtlasAuthoring {
  define(definition: AtlasDefinition): AtlasDefinition;
  scenario(definition: AtlasDefinition, scenario: Scenario): Checked<Scenario>;
  bundle(definition: AtlasDefinition, scenarios: readonly Scenario[], layouts?: readonly LayoutDocument[]): Checked<AtlasBundle>;
}
export type SchemaConversion = Checked<ValueSchema>;
/** Standard Schema validation alone is insufficient to export an inspectable schema. */
export interface SchemaExporter {
  convert(schema: unknown, context: SchemaExportContext): SchemaConversion;
}
export interface ContractBinding {
  readonly procedurePath: readonly string[];
  readonly target: TypePortRef;
  readonly semanticKind: 'command' | 'query';
  readonly inputSchemaId: Id;
  readonly outputSchemaId: Id;
  /** Exact procedure errors map to explicit domain outcome identities. */
  readonly errors: Readonly<Record<string, Id>>;
}
/** Optional upstream-specific adapter; no procedure handler is invoked. */
export interface ProcedureContractImporter {
  import(options: {
    readonly contract: unknown;
    readonly bindings: readonly ContractBinding[];
    readonly schemas: SchemaExporter;
  }): Checked<{ readonly schemas: readonly ValueSchema[]; readonly portFragments: readonly { readonly target: TypePortRef; readonly procedurePath: readonly string[]; readonly port: OperationPort }[] }>;
}

export interface PlayerState {
  readonly scenarioId: Id;
  readonly committedEventId?: Id;
  readonly activeEventId?: Id;
  readonly phase: 'ready' | 'flow' | 'handling' | 'applied';
  readonly progress: number;
  readonly playing: boolean;
  readonly speed: number;
  readonly snapshot: Snapshot;
}
export interface Player {
  state(): PlayerState;
  play(): void;
  pause(): void;
  next(): void;
  previous(): void;
  seek(eventId?: Id): Checked<PlayerState>;
  setSpeed(speed: 0.25 | 0.5 | 1 | 2): void;
  subscribe(listener: (state: PlayerState) => void): () => void;
  dispose(): void;
}

export interface Bounds { readonly x: number; readonly y: number; readonly width: number; readonly height: number; }
export interface SpatialGraph {
  readonly nodes: readonly { readonly id: Id; readonly bounds: Bounds; readonly parentId?: Id }[];
  readonly ports: readonly { readonly id: Id; readonly objectId: Id; readonly point: Point; readonly side: 'left' | 'right' | 'top' | 'bottom' }[];
  readonly edges: readonly { readonly id: Id; readonly fromPortId: Id; readonly toPortId: Id }[];
}
export interface RoutedGraph {
  readonly graph: SpatialGraph;
  readonly routes: Readonly<Record<Id, readonly Point[]>>;
  readonly diagnostics: readonly Diagnostic[];
}
export interface DiagramLayout {
  arrange(graph: SpatialGraph, pinned: readonly Id[], signal: AbortSignal): Promise<Checked<RoutedGraph>>;
  /** Route around fixed object bounds without moving a user's pinned objects. */
  route(graph: SpatialGraph, signal: AbortSignal): Promise<Checked<RoutedGraph>>;
}
export interface LayoutSession {
  value(): LayoutDocument;
  move(objectId: Id, point: Point): Checked<LayoutDocument>;
  pin(objectId: Id, pinned: boolean): Checked<LayoutDocument>;
  reset(scope: 'selection' | 'view', objectIds?: readonly Id[]): void;
  undo(): void;
  redo(): void;
  export(): LayoutDocument;
  /** Reject cross-document layouts; unknown object IDs produce diagnostics. */
  import(layout: unknown): Checked<LayoutDocument>;
}
export interface LayoutStorage {
  load(key: string): Promise<Checked<LayoutDocument | undefined>>;
  save(key: string, layout: LayoutDocument): Promise<Checked<void>>;
}
export interface Viewer {
  readonly player: Player;
  readonly layout: LayoutSession;
  select(ref:
    | { readonly kind: 'object'; readonly objectId: Id }
    | { readonly kind: 'port'; readonly target: InstancePortRef }
    | { readonly kind: 'interaction'; readonly callId: Id }
    | { readonly kind: 'state'; readonly objectId: Id; readonly fieldId: Id }
    | { readonly kind: 'process'; readonly processId: Id; readonly nodeId?: Id }
  ): void;
  setView(scope: ViewScope): Checked<void>;
  fit(): void;
  exportHtml(): Promise<Checked<Uint8Array>>;
  dispose(): void;
}
export interface ViewerFactory {
  /** Borrows atlas/container; owns its listeners/frames/workers, released by dispose. */
  mount(container: HTMLElement, atlas: CompiledAtlas, options: {
    readonly scenarioId: Id;
    readonly layout?: LayoutDocument;
    readonly storage?: LayoutStorage;
  }): Viewer;
}
