/** @aiPurpose Define the larger target model and shared identity, JSON Schema and diagnostic vocabulary.
 * @aiUsage Authoring implements a bounded source-binding slice; other target contracts remain design-only.
 * Implemented domain/workbench semantics are owned by ../domain/contracts.ts; these broader legacy shapes remain target-only.
 * JSON-facing shapes become inferred exports of runtime schemas in implementation.
 * See README.md for invariants that structural TypeScript cannot prove.
 */

export type Id = string;
export type Json = null | boolean | number | string | readonly Json[] | { readonly [key: string]: Json };
export interface Source { readonly id: Id; readonly label: string; readonly uri: string; readonly basis: 'design' | 'source' | 'observation'; }
export interface Explanation { readonly summary: string; readonly detail?: string; readonly sourceIds: readonly Id[]; }
/** Inert, portable provenance. Extensions use an owner namespace, such as `acme:review`. */
export interface DesignMetadata {
  readonly summary?: string;
  readonly responsibility?: string;
  readonly tags?: readonly string[];
  readonly sourceLinks?: readonly { readonly label: string; readonly uri: string }[];
  readonly documents?: readonly { readonly id: Id; readonly label: string; readonly uri: string }[];
  readonly extensions?: Readonly<Record<`${string}:${string}`, Json>>;
}
export interface Identity { readonly id: Id; readonly label: string; readonly explanation?: Explanation; readonly metadata?: DesignMetadata; }
export interface SchemaRef { readonly schemaId: Id; }
/** Portable JSON Schema input/output, with unsupported conversion reported explicitly. */
export interface ValueSchema extends Identity { readonly jsonSchema: Readonly<Record<string, Json>>; }

export interface Domain extends Identity { readonly parentId?: Id; }
export interface Module extends Identity { readonly domainId: Id; readonly parentId?: Id; }
/** Dependency cycles are project policy; containment cycles are structural errors. */
export interface ModuleDependencyPolicy { readonly dependencyCycles?: 'warning' | 'error' | 'allow'; }
/** Shared converter context; labels are optional so existing exporter implementations remain compatible. */
export interface SchemaExportContext { readonly id: Id; readonly label?: string; readonly io: 'input' | 'output'; }

export interface Field extends Identity { readonly schema: SchemaRef; }
export interface StateDefinition extends Identity {
  readonly fields: readonly Field[];
  /** Optional finite lifecycle. Independent fields are not flattened into this enum. */
  readonly lifecycle?: {
    readonly fieldId: Id;
    readonly states: readonly { readonly id: Id; readonly label: string; readonly terminal?: boolean }[];
    readonly transitions: readonly Transition[];
  };
}
/** A legal edge is declared here; a trace must explicitly witness its occurrence. */
export interface Transition extends Identity {
  readonly from: Id;
  readonly to: Id;
  readonly triggerPortId: Id;
  readonly outcomeId?: Id;
  readonly guard?: string; // Human explanation, never an evaluated expression.
}

export type OutcomeMeaning = 'accepted' | 'returned' | 'observed' | 'rejected' | 'unknown' | 'failed' | 'no-response';
export interface Outcome extends Identity {
  readonly meaning: OutcomeMeaning;
  readonly payload?: SchemaRef;
}
export interface OperationPort extends Identity {
  readonly kind: 'command' | 'query';
  readonly input: SchemaRef;
  readonly outcomes: readonly Outcome[];
}
export interface EventPort extends Identity {
  readonly kind: 'event';
  readonly direction: 'emit' | 'receive';
  readonly payload: SchemaRef;
}
export type Port = OperationPort | EventPort;

/** Type-level public surface; a scenario may instantiate it more than once. */
export interface ObjectType extends Identity {
  readonly domainId: Id;
  readonly moduleId?: Id;
  readonly kind: 'entity' | 'service' | 'actor' | 'store' | 'process' | 'logical';
  readonly state: StateDefinition;
  readonly ports: readonly Port[]; // Port IDs are scoped to the ObjectType.
}
export interface TypePortRef { readonly typeId: Id; readonly portId: Id; }
export type Relationship = Identity & (
  | { readonly kind: 'uses'; readonly fromTypeId: Id; readonly target: TypePortRef }
  | { readonly kind: 'association'; readonly fromTypeId: Id; readonly toTypeId: Id; readonly cardinality: string }
  | { readonly kind: 'event-route'; readonly from: TypePortRef; readonly to: TypePortRef }
);

/** Graph roles bind to concrete object instances only inside a scenario. */
export interface Role extends Identity { readonly typeId: Id; }
export interface RolePortRef { readonly roleId: Id; readonly portId: Id; }
export type ProcessNode = Identity & (
  | { readonly kind: 'start' }
  | { readonly kind: 'end'; readonly outcome: 'completed' | 'rejected' | 'unknown' }
  | { readonly kind: 'operation'; readonly callerRoleId: Id; readonly target: RolePortRef }
  | { readonly kind: 'event'; readonly from: RolePortRef; readonly to: readonly RolePortRef[] }
  | { readonly kind: 'decision'; readonly question: string }
  | { readonly kind: 'fork'; readonly joinId: Id }
  | { readonly kind: 'join'; readonly forkId: Id }
);
export interface ProcessFlow extends Identity {
  readonly from: Id;
  readonly to: Id;
  /** Authored branch condition, not an executable guard. */
  readonly condition?: string;
  readonly outcomeId?: Id;
}
export interface ProcessDefinition extends Identity {
  readonly roles: readonly Role[];
  readonly nodes: readonly ProcessNode[];
  readonly flows: readonly ProcessFlow[];
  readonly startId: Id;
}

/** Immutable design document: types, relationships and possible process paths. */
export interface AtlasDefinition extends Identity {
  readonly format: 'flow-atlas/design-1';
  readonly sources: readonly Source[];
  readonly schemas: readonly ValueSchema[];
  readonly domains: readonly Domain[];
  readonly modules: readonly Module[];
  readonly objectTypes: readonly ObjectType[];
  readonly relationships: readonly Relationship[];
  readonly processes: readonly ProcessDefinition[];
}

export interface ObjectInstance extends Identity { readonly typeId: Id; }
export interface InstancePortRef { readonly objectId: Id; readonly portId: Id; }
export interface PortCaller { readonly objectId: Id; readonly portId?: Id; }
export interface Channel extends Identity {
  readonly fromObjectId: Id;
  readonly toObjectId: Id;
  readonly transport: string;
}
export type ValueCell =
  | { readonly kind: 'known'; readonly value: Json }
  | { readonly kind: 'unknown'; readonly reason: string }
  | { readonly kind: 'redacted'; readonly reason: string };
export interface Snapshot {
  readonly objects: Readonly<Record<Id, Readonly<Record<Id, ValueCell>>>>;
}
export interface StateWrite {
  readonly objectId: Id;
  readonly fieldId: Id;
  readonly value: ValueCell;
  readonly nature: 'change' | 'observation';
  readonly transitionId?: Id;
}
export interface StateDifference {
  readonly objectId: Id;
  readonly fieldId: Id;
  readonly before: ValueCell;
  readonly after: ValueCell;
  readonly nature: 'change' | 'observation';
  readonly causeEventId: Id;
  readonly transitionId?: Id;
}
export type Evidence =
  | { readonly kind: 'authored'; readonly explanation?: string }
  | { readonly kind: 'observed'; readonly sourceIds: readonly Id[]; readonly recordedAt: string };
export interface TraceBase extends Identity {
  readonly processNodeId?: Id;
  /** Causal prerequisites; array storage order alone does not prove causality. */
  readonly after: readonly Id[];
  readonly evidence: Evidence;
}
export type TraceEvent = TraceBase & (
  | { readonly kind: 'call'; readonly callId: Id; readonly from: PortCaller; readonly target: InstancePortRef; readonly channelId?: Id; readonly input: ValueCell }
  | { readonly kind: 'reply'; readonly callId: Id; readonly outcomeId: Id; readonly payload?: ValueCell }
  | { readonly kind: 'outcome'; readonly callId: Id; readonly outcomeId: Id; readonly reason: string }
  | { readonly kind: 'emit'; readonly emissionId: Id; readonly from: InstancePortRef; readonly payload: ValueCell; readonly channelId?: Id }
  | { readonly kind: 'receive'; readonly emissionId: Id; readonly target: InstancePortRef }
  | { readonly kind: 'state'; readonly causeEventId: Id; readonly writes: readonly StateWrite[] }
  | { readonly kind: 'channel'; readonly channelId: Id; readonly state: 'connected' | 'disconnected' | 'unknown'; readonly reason: string }
  | { readonly kind: 'checkpoint'; readonly note: string }
);
export interface ProcessVisit extends Identity {
  readonly nodeId: Id;
  readonly occurrence: number; // Loops have separate visits, not duplicate process nodes.
  readonly eventIds: readonly Id[];
  readonly viaFlowId?: Id;
}
export interface Scenario extends Identity {
  readonly definitionId: Id;
  readonly objects: readonly ObjectInstance[];
  readonly channels: readonly Channel[];
  readonly initial: Snapshot;
  readonly process?: { readonly processId: Id; readonly bindings: Readonly<Record<Id, Id>>; readonly visits: readonly ProcessVisit[] };
  /** One recorded presentation order consistent with `after`, not a scheduler. */
  readonly events: readonly TraceEvent[];
}

/** Semantic content and presentation are serialized separately. */
export type ViewKind = 'relations' | 'process' | 'sequence' | 'state';
export type ViewScope =
  | { readonly kind: 'relations'; readonly scenarioId?: Id }
  | { readonly kind: 'process'; readonly processId: Id; readonly scenarioId?: Id }
  | { readonly kind: 'sequence'; readonly scenarioId: Id }
  | { readonly kind: 'state'; readonly typeId: Id; readonly instance?: { readonly scenarioId: Id; readonly objectId: Id } };
export interface Point { readonly x: number; readonly y: number; }
export interface Viewport { readonly x: number; readonly y: number; readonly zoom: number; }
export interface LayoutDocument {
  readonly format: 'flow-atlas/layout-1';
  readonly definitionId: Id;
  readonly scope: ViewScope;
  /** Binds cached layout to actual content, not merely a reused human ID. */
  readonly contentDigest: string;
  readonly positions: Readonly<Record<Id, Point>>;
  readonly collapsed: readonly Id[];
  readonly pinned: readonly Id[];
  readonly routes: Readonly<Record<Id, readonly Point[]>>;
  readonly viewport: Viewport;
}
export interface AtlasBundle {
  readonly definition: AtlasDefinition;
  readonly scenarios: readonly Scenario[];
  readonly layouts: readonly LayoutDocument[];
}

export interface Diagnostic {
  readonly severity: 'error' | 'warning';
  readonly code: string;
  readonly path: readonly (string | number)[];
  readonly message: string;
}
export type Checked<T> =
  | { readonly ok: true; readonly value: T; readonly diagnostics: readonly Diagnostic[] }
  | { readonly ok: false; readonly diagnostics: readonly Diagnostic[] };

/** Data, process and trace public methods are pure and never call a business handler. */
export interface AtlasCompiler {
  compile(bundle: unknown): Checked<CompiledAtlas>;
}
export interface CompiledAtlas {
  readonly bundle: AtlasBundle;
  snapshot(scenarioId: Id, throughEventId?: Id): Snapshot; // Omitted means initial.
  changes(scenarioId: Id, eventId: Id): readonly StateDifference[];
  interactions(scenarioId: Id, throughEventId?: Id): readonly InteractionProjection[];
}
export interface InteractionProjection {
  readonly id: Id;
  readonly kind: 'operation' | 'event';
  readonly from: PortCaller;
  readonly target: InstancePortRef;
  readonly phase: 'pending' | 'accepted' | 'returned' | 'observed' | 'rejected' | 'unknown' | 'failed' | 'no-response';
  readonly startedAtEventId: Id;
  readonly completedAtEventId?: Id;
}
