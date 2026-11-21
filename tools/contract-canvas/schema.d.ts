/** @aiPurpose Define the portable Canvas facts supplied by a consumer. */
export type NodeKind = 'interface' | 'class' | 'value' | 'union' | 'enum' | 'method' | 'outcome' | 'capability' | 'external' | 'utility';
export type RelationKind = 'owns' | 'accepts' | 'returns' | 'contains' | 'references' | 'observes' | 'rejects' | 'composes' | 'variant' | 'supports' | 'calls' | 'aliases' | 'extends' | 'implements' | 'wraps' | 'delegates';
/** @aiPurpose Separate declaration evidence from proposed design. @aiUsage Inspect independently of support states. */
export type Provenance = 'source-backed' | 'proposed';
/** Defaults to public. Implementation anchors remain addressable through explicit disclosure. */
export type DeclarationVisibility = 'public' | 'implementation';
/** @aiPurpose Own one abstraction boundary independently of domains and current visibility. @aiUsage Consume in authored array order. */
export interface ContractLayer {
  id: string;
  name: string;
  summary: string;
  status: Provenance;
  sourceRefs: string[];
}
/** Optional project-defined capability statuses and their explanation. */
export type Support = { note?: string; [name: string]: string | undefined };
export interface TypedValue {
  type: string;
  /** Nonliteral semantic dependencies; Promise/Readonly/array syntax is shown in type. */
  typeIds: string[];
  optional?: boolean;
  nullable?: boolean;
}
export interface Field extends TypedValue {
  /** Stable field anchor; valid edge endpoint and inspector selection. */
  id: string;
  name: string;
  summary: string;
  visibility?: DeclarationVisibility;
}
export interface Parameter extends TypedValue { name: string }
interface NodeIdentity {
  id: string;
  name: string;
  domain: string;
  layerId: string;
  provenance: Provenance;
  visibility?: DeclarationVisibility;
  summary: string;
  signature?: string;
  fields: Field[];
  ports: string[];
  support?: Support;
  sourceRefs: string[];
}
/** Methods carry typed inputs, outputs and optional failure/outcome identities in required arrays. */
export type ContractNode = NodeIdentity & (
  | { kind: Exclude<NodeKind, 'method'> }
  | {
    kind: 'method';
    owner: string;
    inputs: Parameter[];
    output: TypedValue;
    errors: string[];
    outcomes: string[];
    effect: 'local' | 'read' | 'submit' | 'observe';
  }
);
interface EdgeIdentity {
  id: string;
  from: string;
  to: string;
  label: string;
  provenance: Provenance;
  sourceRefs: string[];
}
export type ContractEdge = EdgeIdentity & (
  | {
    kind: 'delegates';
    /** Receiver field on the source method owner, including inherited fields. */
    via: string;
    /** Argument/result/failure correspondence or intentional projection. */
    mapping: string;
  }
  | { kind: Exclude<RelationKind, 'delegates'>; via?: never; mapping?: string }
);
export interface SourceEvidence {
  id: string;
  /** Display-only source location. Lines are one-based; the tool does not resolve the path. */
  path: string;
  line: number;
  symbol: string;
  authority: 'semantic' | 'declaration' | 'implementation' | 'generated' | 'native' | 'platform' | 'proposal';
}
export interface Scenario {
  id: string;
  provenance: Provenance;
  name: string;
  summary: string;
  steps: { label: string; nodeIds: string[] }[];
  sourceRefs: string[];
}
export interface ContractModel {
  schemaVersion: 1;
  id: string;
  title: string;
  authority: string;
  layers: ContractLayer[];
  nodes: ContractNode[];
  edges: ContractEdge[];
  sources: SourceEvidence[];
  scenarios: Scenario[];
}

/** @aiPurpose Supply derived, offline-safe layer navigation without a second authored registry. @aiUsage Access globalThis.CONTRACT_LAYERS after loading the generated data script. */
export interface ContractLayersAPI {
  /** Each layer has a title alias and nodeIds derived from node.layerId, including method ports. */
  layers: (ContractLayer & { title: string; nodeIds: string[] })[];
  layer(id: string): ContractLayersAPI['layers'][number] | undefined;
  /** Accepts node, method or exact field anchor IDs; unknown anchors return undefined. */
  layerForNode(id: string): ContractLayersAPI['layers'][number] | undefined;
  /** Canonical incoming/outgoing edges crossing this layer; unknown layer returns []. No flattened/synthetic edges. */
  layerMappings(id: string): ContractEdge[];
}


/** Layout preferences reference model IDs. They do not add declarations or relations. */
export interface ContractView {
  schemaVersion: 1;
  brand?: Partial<Record<'name' | 'title' | 'ariaLabel' | 'edition' | 'editionNumber' | 'footer', string>>;
  initial?: { layerId: string; subjectId?: string };
  layers?: Record<string, { label?: string; initialSubject?: string; order?: string[] }>;
  domains?: Record<string, { label?: string; color?: 'blue' | 'teal' | 'violet' | 'amber' }>;
  supportLabels?: Record<string, string>;
  proposedSupportKeys?: string[];
  neighborOrder?: string[];
  overview?: {
    subjectId: string;
    tab?: string;
    title?: string;
    groups: { label: string; nodeIds: string[] }[];
    ports?: string[];
    highlightFrom?: string;
  };
  methodAnnotations?: Record<string, string[]>;
  walkthrough?: {
    scenarioId: string;
    layerId: string;
    tab?: string;
    title?: string;
    kicker?: string;
    stepTitles?: string[];
  };
  sequence?: {
    tab?: string;
    title?: string;
    kicker?: string;
    stepHeading?: string;
    branchHeading?: string;
    stepLabel?: string;
    branchLabel?: string;
    lanes: { label: string; narrowLabel?: string }[];
    steps: { id: string; label: string; caption: string; from: number; to: number; error?: boolean }[];
    branchSource?: string;
    annotation?: { id: string; label?: string; narrowLabel?: string; ariaLabel?: string; caption?: string; narrowCaption?: string };
  };
  lenses?: ({ id: string; label: string; detail?: string } &
    ({ scene: 'method'; subjectId: string } | { scene: 'sequence' }))[];
}
