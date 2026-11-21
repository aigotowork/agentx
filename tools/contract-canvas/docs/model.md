# Model authoring and format

[Documentation home](../README.md) · [Quickstart](quickstart.md) · [TypeScript contract](../schema.d.ts)

Write the semantic declarations in `model.json`. The [TypeScript contract](../schema.d.ts) defines the record shapes; [the validator](../validate.mjs) checks cross-record rules. The [library model](../example/model.json) is a complete working JSON example. Use the [view](view.md) for ordering, labels and optional diagram setup; keep positions, camera and scene layout out of model nodes.

## Start with what a caller needs to read

In the library example, a reader wants to know what `LoanDesk.previewLoan` accepts, what it returns, and how the returned slip relates to the original catalog copy.

1. Give each declaration a stable ID and one owner layer: `LoanDesk` and `LoanSlip` in `desk`; `CatalogCopy` in `catalog`. The `domain` groups objects in the directory independently of their owner layer.
2. Add `LoanDesk.copy` as a field. Give it display text `CatalogCopy` and `typeIds: ["CatalogCopy"]`. Record both `LoanDesk → owns → LoanDesk.copy` and `LoanDesk.copy → references → CatalogCopy`.
3. Add `LoanDesk.previewLoan` to `LoanDesk.ports` and declare it as its own `method` node with owner `LoanDesk`. Use `CatalogCopy` in the typed input and `LoanSlip` in the typed output; add the matching `owns`, `accepts` and `returns` edges. A string like `previewLoan(copy: CatalogCopy): LoanSlip` is a display signature and cannot replace those links.
4. Add `LoanSlip.copy` back to the same `CatalogCopy` type, with its own ownership and reference edges. A primitive such as the `string` type in `CatalogCopy.barcode` uses `typeIds: []` and still requires a field ownership edge.
5. Use `sources` to identify the project's actual declarations or design decisions. Add a scenario whose steps point to exact IDs such as `LoanDesk.copy` and `LoanDesk.previewLoan`. Use `view.walkthrough` to expose it as a tab.

## Model records

The root requires `schemaVersion: 1`, `id`, `title`, `authority` and the five arrays below. Use a non-empty string for IDs, names and required explanations. Arrays are explicit even when empty. Each layer needs at least one public non-method node so it can be opened.

| Collection | What to write |
| --- | --- |
| `layers` | Ordered declarations of ownership: `id`, `name`, `summary`, `status`, `sourceRefs`. Use separate layers for distinct abstractions, not for every screen. |
| `nodes` | `id`, `name`, `kind`, `domain`, `layerId`, `provenance`, `summary`, `fields`, `ports`, `sourceRefs`. See [NodeKind](../schema.d.ts) for the supported kinds. Optional `visibility`, `signature` and `support` supplement the declaration. |
| `edges` | `id`, `from`, `to`, `kind`, `label`, `provenance`, `sourceRefs`. Endpoints are exact node or field anchors. See the rules below. |
| `sources` | `id`, `path`, positive one-based `line`, `symbol`, `authority`. Paths are displayed/copied as text. The generic tool does not read, resolve or fetch them. A project may check them against its own source. |
| `scenarios` | `id`, `name`, `summary`, `provenance`, `sourceRefs` and at least one `steps` entry. Each step has a `label` and non-empty `nodeIds` list of existing node or field anchors. These checkpoints document a reading path. |

A field contains `id: "<owner-id>.<name>"`, `name`, `summary` and a typed value. Fields and nodes share one anchor namespace, so a field ID cannot collide with a method or another node. Fields inherit layer and provenance from their owner. Nested types are separate nodes; do not put declarations inside a field.

Methods add `owner`, `inputs`, `output`, `errors`, `outcomes` and `effect` to the common node fields. `fields` and `ports` are empty on a method; its parent lists its ID in `ports` and shares layer and provenance. By convention use IDs like `Owner.method`. Each input adds a unique `name` to a typed value; `output` is always a typed value (e.g. `{"type":"void","typeIds":[]}`). `effect` describes the contract as `local`, `read`, `submit` or `observe`. It never causes execution.

A typed value has a human-readable `type`, a `typeIds` array of exact node IDs and optionally boolean `optional`/`nullable`. For `Promise<readonly LoanSlip[]>` the visible expression stays intact and `typeIds` can be `["LoanSlip"]`; the renderer does not parse generic or union syntax. Model named types you want readers to navigate, without inventing declarations for every primitive, container or syntax token.

## Exact relationships

Edge IDs are `<from>|<kind>|<to>`, unique within the graph. Model a relation from its real origin: a field reference starts at that field's anchor, and a method input starts at the method port. The renderer may route the picture through parent cards; the authored edge remains exact.

| Relation | Direction and use |
| --- | --- |
| `owns` | Declaration → its field anchor or port. Required even when the child is also listed in `fields` or `ports`. |
| `references`, `contains` | Field anchor → named type is a common use; each declared field `typeIds` reference needs a directed edge. Use the relation that reflects the actual connection. |
| `accepts`, `returns`, `rejects` | Method → each named input type, output type or error identity, respectively. Each declared outcome also needs a directed edge. Keep declared output variants distinct from observable rejections. |
| `extends`, `implements` | Derived declaration → base class or base interface of the same kind, or class → implemented interface. Extension must be acyclic. Implementation checks the interface's fields and port input/output correspondence. A union is never a base class. |
| `wraps`, `delegates` | Receiver field → the type it actually holds; delegating method → callee method. Delegation requires `via` (the owner's or inherited field) and a non-empty `mapping` (argument/result/failure correspondence). The held type must supply the callee; effects must agree and callee errors/outcomes must remain visible. When the callee has source references, cite one on the edge. Use `calls` and separately model orchestration when the effects or failure contract change. |
| `variant`, `composes`, `calls`, `aliases`, `supports`, `observes` | Describe alternatives, composition, calls, equivalent declarations, support or observation. Represent actual facts; use `variant` from a declaration to branch nodes for an optional sequence branch picker. |

Use `source-backed` for claims about existing declarations and `proposed` for design. A relation with a proposed endpoint stays proposed; a scenario including any proposed subject stays proposed. Existing source claims with evidence should cite more than proposal-only evidence. `support` is optional and carries project-defined status strings and an optional `note`. It does not replace provenance. Set `visibility: "implementation"` for internals hidden from the default public reading surface. It is disclosure metadata, not access control: the viewer can still open those declarations and the generated dataset contains them.

Run the [build/check command](quickstart.md#2-build-and-open-it) to catch broken identifiers, edges and view bindings. Passing these checks establishes structural consistency, not source accuracy or runtime behavior.
