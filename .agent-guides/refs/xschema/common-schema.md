---
schema: document
description: Shared minimal metadata structures for agents using xschema.
---

<!-- Generated from the declared xschema sources; see sources.json. Do not edit this copy. -->


# Shared structures

Metadata describes the subject carried by a document, file or directory: its type, state and relationships. Keep that structured metadata separate from the substantive content.

Prefer paths starting with `./` or `../`, relative to the metadata file's directory. A directory carrier uses the directory it describes. Existing paths without these prefixes remain project-root relative; query addresses also remain root-relative. See the [path contract](./schema-contract.md#common) for syntax and workspace boundaries. Markdown uses YAML front matter and body content. Standalone YAML may use `content`. A directory uses one `.xmeta.yaml` or `.xmeta.md`.

## Common fields

```yaml
description: ...
tags: []
links: []
```

These fields are generally optional; a Scenario requires at least one tag. Use tags for discovery and combined filtering, and links for resource-level related material without duplicating body links.

## Document or directory

```yaml
schema: document # package for a directory with its own context
description: ...
```

When maintaining a directory's artifact catalog, use `artifacts` in that same Package carrier:

```yaml
schema: package
description: Example directory responsibility.
artifacts:
  - {kind: design, path: ./.docs, description: Durable design decisions.}
```

Use `schema: package` for new directory metadata; legacy `module` inputs normalize to this same model. See [compatibility boundaries](./schema-contract.md#module) when maintaining old queries. A Package describes directory responsibility and contents, including private source, tests or documentation; it does not imply an installable package or public API. Omit artifacts when no catalog is needed. List the directory's concrete resources and describe their purposes using [Package fields](./schema-contract.md#package). Keep general layouts, path templates and naming conventions in the project's [placement guidance](../../documentation.md#artifact-placement-rules); an artifact may link applicable guidance. Do not maintain a separate inventory for the same directory.

## Role, Scenario or Skill

```yaml
schema: role
role: designer # required; use a consistent role category
description: ...
tags: [design] # optional
```

```yaml
schema: scenario
description: ...
tags: [frontend, design] # required; at least one tag
```

Use `schema: skill` for a reusable capability entry and preserve its native protocol. All three retain the common description, tags and links fields. Keep conditions, routing, methods and usage in the body; use [progressive guidance authoring](../../guidance-authoring.md) when composing any of these three subjects.

## Task

```yaml
schema: task
status: active # planned | active | blocked | completed | cancelled
agent: ... # optional
# blocking_reason: ... # required when blocked; remove otherwise
```

Preserve unrelated fields and unknown values. Assignment context supplies the Agent binding, authority and reply target.

- Planning or design author: use [Roadmap](./controller-schema.md#roadmap) and [Proposal](./controller-schema.md#proposal) structures; no Controller role is required.
- Worker: maintain the assigned Task; no additional schema reading required.
- Controller: also read [Controller structures](./controller-schema.md).

## Typed resource filenames

For the supplied Task and Delivery models, always create `.task.md` and `.delivery.md` records with their matching content schemas. The current defaults have no legacy filename fallback; this supersedes the conditional filename guidance below for these two models. See the [canonical contract](./schema-contract.md#canonical-task-and-delivery-standard).

Author `subject.task.md`, `subject.delivery.md`, `subject.proposal.md`, `subject.roadmap.md` and `subject.task-meta.md` with the unchanged content schema. Use `delivery`, with this spelling. The suffix selects candidates before content reads; it does not supply model identity or replace validation. Keep directory carriers, `AGENTS.md`, `SKILL.md`, stable controller entry points and fixed fixture/evidence names at their native protocol paths.

The default collection declares authoritative Task/Delivery filename bindings with `assertModel: true`: only `*.task.md` and `*.delivery.md` are candidates. No conventional old names, YAML Task/Delivery resources or arbitrary-front-matter fallback remains. Content schema and fields still validate; `source_model_mismatch` diagnoses a candidate of the wrong model, and explicit out-of-binding declarations remain auditable without becoming model projections. Formal originals versus historical fixture populations need separate explicit source selections. Unrelated custom models retain generic discovery. See the [source contract](./schema-contract.md#canonical-task-and-delivery-standard).
