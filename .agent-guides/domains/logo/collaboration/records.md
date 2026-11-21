---
schema: document
description: Minimal semantic records for logo design objects, collaborative rounds, execution inputs, feedback and scoped evidence
---

# Records and semantic contracts

Use these records when a decision, artifact or relationship needs to survive an iteration or handoff. They define meaning for this method. They are not an implemented JSON schema, database or mandatory collection of forms. Short Markdown records are sufficient until a tool needs a native format. A future implementation must own native validation and have these guides reference it.

Use the consuming project's existing design/task record with links to sources and assets; do not create a second task tracker. Avoid copying canonical geometry, license text or the same decision into multiple trackers. Scale detail to the work. A one-off refinement can record its retained cue, changed version and result in a few lines.

## Project and brief

Retain the project identity and actual brand spelling, the current task and deliverable, known audience/use, intended associations, confirmed preferences, retained constraints, material unknowns and available assets. Distinguish facts, user statements and author hypotheses. Keep the user’s scope and session constraints, including image inspection and generation preferences, visible.

A brief revision should identify which question or constraint changed. Do not reinterpret “explore more freely” as permanent permission to change the product name, publish a logo or use a paid API.

## Collaborative round and plan

A round groups work around a design question, not a generation call or a mandatory workflow step. It may contain reference study, several production activities, comparison and a decision. Use a stable identity when a round needs to survive interruption or link concurrent work. Retain only the relationships needed to resume:

| Meaning | Context to retain |
| --- | --- |
| Current question | Brief revision, intended judgment and relevant scope/constraints |
| Subjects and baseline | Direction, reference traits, exact source/presentation versions and the comparison being made |
| Feedback basis | Actual submitted feedback revisions, author and target; unresolved wording remains visible |
| Interpretation and plan | Attributed interpretation, plan revision, retained/changed/explored/controlled choices and proposed check |
| Work and results | Related operations, actual artifacts, evidence and unresolved dependencies |
| Outcome and continuation | Scoped decision, remaining question, next action and branch relationship where useful |

User statements, agent interpretations and accepted decisions remain distinct. A plan revision identifies its basis and meaningful delta; it is not a request for mandatory approval. A branch may inherit a baseline and selected constraints without inheriting every preference from another candidate. Preserve the inheritance scope rather than assuming all feedback becomes a global brand rule.

The [conversation method](conversation.md) owns forming the plan; [collaboration design](workbench.md) owns how it supports an interaction. A short refinement can carry this context in a few sentences instead of separate forms for every row.

## Direction

A direction is a design proposition motivated by the brief, not a color preset. Give it a stable ID and retain:

| Field meaning | Useful content |
| --- | --- |
| Brief connection | The brand question or intended association being explored |
| Proposition | One sentence explaining the organizing idea and how the viewer encounters it |
| Formal hypothesis | Subject, relationship/topology, identity form, visual language and composition as relevant |
| Open and retained choices | What is fixed in this iteration and what remains to explore |
| Reference principles | The exact source traits used and why; link reference IDs |
| Distinction | How it differs from the nearest direction beyond a renamed story or recolor |
| Evaluation question | What this round should help the user judge |
| Limits | Material assumptions, unresolved production needs or untested audience readings |

A missing choice stays unknown; do not fill every facet with arbitrary defaults. A material-led direction may legitimately specify finish before silhouette. Label it accordingly so it is not confused with a fully developed shape concept.

## Reference

Retain reference ID; local asset or URL; creator/source when known; role in the work; extracted visible traits; application context; rights/permitted role or unknown status; and provenance for generated or transformed material. The roles are explained in [research](../research/inspiration.md).

Tags use `facet_id/value_id` from the [vocabulary](../resources/style-vocabulary.yaml), with optional scoped extensions. For each interpretive or automatically proposed tag, keep who proposed it, supporting note and review state. Confidence can be qualitative and must not be invented as a measured probability. A user or model label can be corrected without changing the source asset.

A generated reference includes provider/model when known, the actual prompt, input references and their roles, output identity and transformations. Do not collect credentials in these records.

## Candidate, source and variant

- A **candidate** has a stable identity, a direction relationship and a concise proposition. It can own several asset versions.
- An **asset version** identifies the actual source file, source kind, role, editable status, intended bounds/background and version. Preserve required font/resource dependencies and provenance. Use readable versions; do not invent checksum receipts as a substitute for source identity.
- A **variant** names its source asset version, changed properties, role, actual file and status. Examples are a compact contour, a single-ink asset or a metal rendering. Do not infer existence from an export filename that was planned but never created.
- A **derivation** records inputs and transformation. If topology, lettering or the recognition cue changes, retain it as a revised asset or different candidate rather than claiming an unchanged finish.

Keep actual geometric definitions with the source. Construction guides refer to that geometry or identify themselves as reconstructed analysis. A raster placed inside SVG remains a raster asset; editable vector and editable raster are separate capabilities.

An editable source may be a native vector document, SVG, a construction script with its parameters, lettering source or a three-dimensional scene. Preserve the actual editing path and required application/font/resource context. When a native master and a production SVG coexist, retain their derivation and identify where edits belong instead of treating independent files as silently synchronized masters. Geometric parameters and optical corrections stay with that source; a construction overlay is a version-linked presentation view, not a second geometry registry or part of the clean logo export.

## Presentation

Retain board/scene ID and version; design question; exact candidate/asset versions; view roles; relevant size/theme/material parameters; layout policy; and output path. A sheet may compare different concepts, controlled variations, construction or applications. State which dimensions are controlled and which intentionally differ.

References, generated inspiration, candidates and mockups must have distinct roles in the presentation data even if they share a page. Display names can be friendly; IDs must remain stable when names change.

## Feedback and decision

A feedback record needs the actual statement or annotation, its author when relevant, a target and scope. For selectable feedback, retain the option ID and vocabulary version, whether the user is describing, retaining or requesting change, and whether it was actually submitted. The [feedback vocabulary](../resources/feedback-vocabulary.yaml) owns these options; the [conversation method](conversation.md) owns interpretation and next-round planning. The target may be a reference trait, a direction, an asset version or a presentation region. Do not require a board if the user gave useful text feedback. When the target is ambiguous, retain that ambiguity and propose the smallest useful contrast; do not silently broaden the preference.

A decision records retained/changed/deferred choices, rationale, affected direction/version and next action. Preserve “selected for exploration,” “selected for refinement” and “accepted for this use” as distinct meanings. User preference is evidence of that user's preference, not a general audience result or a release authorization.

A next-round brief is the round's design plan described above. Feedback revisions keep the actual author and target when the user corrects, retracts or narrows an opinion. An agent interpretation links its feedback basis and uncertainty; it does not overwrite the original statement.

When inputs change, mark affected interpretations or checks as needing review; preserve unaffected decisions. Color feedback can remain valid after a wordmark spacing change, while a recognition observation may not survive an anatomy change.

## Activity and input snapshot

An activity is work such as researching, composing a board, rendering, generating or exporting. Its input snapshot identifies the initiating round and plan revision, brief revision, exact sources/references, included feedback revisions and actual parameters/capability. An activity can be manual; do not invent a running background job for ordinary agent work.

For an executable operation, retain its identity, actual execution state, output identities, errors/limits and supported cancellation or retry behavior. Keep the initiating inputs unchanged after dispatch. Later feedback or a revised plan remains available for subsequent work, but must not silently rewrite what a running activity consumed. A retry or replacement identifies its relationship to the earlier attempt and uses the implementation's real duplicate-request contract.

Results link back to those inputs even when another subject is currently selected. Record which feedback was included, which the result claims to address and which remains open; inclusion alone does not prove resolution. Preserve usable partial outputs. Distinguish a missing output from one that exists but is unverified or no longer fits a changed brief. An unknown save/operation outcome remains unknown until checked; a timeout alone proves neither success nor failure.

## Durable work context and view state

Persist the brief, scoped feedback/decisions, source lineage and round relationships needed for handoff. Preserve an unfinished annotation as a draft when supported, with its target/version and honest save state. Local draft text, durably recorded feedback and feedback included in an activity are different facts. Autosave, direct actions and explicit submission may have different commit behavior; declare that behavior in the native interaction contract.

Shortlist, pinned baseline and relevant return context may need restoration because they carry an unfinished comparison. Hover, pointer movement and every zoom increment do not need a domain event record. Choose persistence/navigation ownership from the actual task. A restored view must not imply an unrecorded design decision, and an unsupported recovery capability must not be claimed as automatic.

## Coverage and evidence

For the current task, connect each material promise to an artifact or decision, a check or feedback source, and its state. Use states such as `not-run`, `partial`, `passed`, `failed` and `not-applicable`, with a reason where needed. These describe scoped evidence, not a second overall project tracker.

Do not require every style, medium or application in every brief. Preserve explicit exclusions and open choices. A deferred user requirement remains incomplete; it is not `not-applicable` merely because it is inconvenient.

Evidence identifies the claim, subject asset/version, procedure or observer, actual result and limits. File parsing, rendering, visual judgment, user preference, audience recognition, fabrication and legal review are different evidence types.

## A compact example

This is an illustrative record, not an existing asset or runnable manifest:

```yaml
project: sample-workshop
brief_revision: 2
direction:
  id: toolmaker-signature
  brief_connection: Express considered craft without appearing institutional.
  proposition: A custom ligature behaves like a signed maker's stamp.
  retained: [brand-spelling, warm-expression]
  references: [reference-etching-01]
  distinction: Letter structure carries recognition; the other territory uses a character.
  question: Does this feel like a personal signature or an official seal?
asset:
  id: signature-master
  version: 3
  candidate: signature-01
  source_kind: vector
  file: assets/signature-v3.svg
  editable: true
presentation:
  id: signature-comparison
  version: 1
  asset_versions: [signature-master@3]
  views: [source, engraved-expression]
feedback:
  target: signature-comparison@1/engraved-expression
  statement: I like the restrained texture, but not the formal border.
  retained_facet: material/engraved-metal
  changed_property: border
coverage:
  - promise: Engraved expression for the selected signature.
    state: partial
    basis: Material study exists; the final production source is not accepted.
```

Real records must point to real files and use the actual project context. Do not create empty forms to satisfy this example.

A [worked two-round example](worked-round.md) demonstrates mixed preferences, fixed activity inputs, later feedback and honest continuation. It is illustrative; its files and operations do not exist in this package.
