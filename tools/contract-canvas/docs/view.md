# View configuration

[Documentation home](../README.md) · [Model format](model.md) · [TypeScript contract](../schema.d.ts)

`view.json` chooses how to show existing declarations. It cannot create types, relationships or evidence. The only required field is `schemaVersion: 1`. With no optional configuration, the viewer opens the first layer and its first public type in the layer scene. Configured ordering and starting subjects can override those defaults.

| Key | Purpose and constraints |
| --- | --- |
| `brand` | Set `name`, `title`, `ariaLabel`, `edition`, `editionNumber` or `footer` as plain strings. The built-in controls use Chinese and English. |
| `initial` | Choose `layerId` and optionally `subjectId`, a public non-method node in that layer. |
| `layers` | Map existing layer IDs to `label`, `initialSubject` and/or `order`. Starting subjects and order entries refer to public non-method nodes in that layer. Ordering can prioritize a subset; it does not remove the other declarations. |
| `domains` | Map domains used in the model to labels and accent colors: `blue`, `teal`, `violet` or `amber`. Domain groups do not alter layer ownership. |
| `supportLabels`, `proposedSupportKeys` | Display labels for project-defined `support` keys; the latter selects which status keys also appear for proposed declarations. The proposal marker remains separate. |
| `neighborOrder` | Prefer named nodes when ordering related types in the type scene. |
| `overview` | Optional overview with `subjectId`, `groups` and optional `tab`/`title`. Each group has `label` and one to four public types in the subject's layer; a type appears in one group at most. `ports` can prefer one method per displayed owner. `highlightFrom` names the origin of an edge highlight. |
| `methodAnnotations` | Map a method ID to related node IDs (excluding methods) to show compact callouts in its method scene. |
| `walkthrough` | Bind one model `scenarioId` and display `layerId`; every checkpoint needs at least one subject in that layer and may link outside it. Optional `tab`, `title`, `kicker` and `stepTitles` affect presentation. When supplied, the titles match the checkpoint count. |
| `sequence` | Optional static trace with two or three lanes and at least one step; see below. |
| `lenses` | Optional shortcut buttons with unique `id`, `label` and optional `detail`. Use `scene: "method"` with a method `subjectId`, or `scene: "sequence"` with a configured sequence. |

The [library view](../example/view.json) starts with `LoanDesk` and exposes `preview-a-loan`. Its [model](../example/model.json) owns the order and identities of its two checkpoints; the view supplies shorter titles without copying those identities.

## When to use a sequence

Use a walkthrough when readers should inspect related contract records in order. Use a sequence when the reader must see the direction of an exchange between participants. A sequence's `lanes` contain `label` and optional `narrowLabel`. Each `steps` record contains a model node `id`, `label`, `caption`, zero-based `from` and `to` lane indexes, and optional boolean `error`. The indexes differ. The first step anchors the sequence tab; later steps can refer to subjects in other layers.

Optional `branchSource` references a model node with one to six outgoing `variant` edges to branch nodes. The viewer reads those edges instead of keeping a second list. `annotation` can show one additional model node by `id`, with optional label/caption, narrow variants and `ariaLabel`. Display text can be customized using the sequence headings and labels in the [schema](../schema.d.ts).

Unconfigured optional scenes and shortcuts are hidden. To see a change, [regenerate the static page and data](quickstart.md#2-build-and-open-it) and refresh.
