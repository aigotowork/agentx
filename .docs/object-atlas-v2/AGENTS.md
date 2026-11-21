# AgentX Contract Canvas consumer

Read the [consumer handoff](README.md) and [model boundary](model/README.md) for AgentX data. General authoring, configuration and maintenance belong to the [tool documentation](../../tools/contract-canvas/README.md). The [extraction Task](../../.controller/contract-canvas-tool/tasks/tool.task.md) and [accepted layer baseline](../../.controller/object-atlas-v2/layers.delivery.md) retain evidence. This reader does not share runtime state with AgentX services or Web products.

| Change | Owner |
| --- | --- |
| Type/field/port meaning, support, layers, visibility and evidence | [Model](model/README.md); modify canonical JSON and retain source correspondence |
| Opening scene, branding, ordering, draft walkthrough, SDK overview, membership and recovery lenses | [AgentX view](view.json); use the [portable view contract](../../tools/contract-canvas/schema.d.ts) |
| Shared layout, rendering, interactions and portable checks | [Contract Canvas tool](../../tools/contract-canvas/AGENTS.md); edit the canonical implementation there |
| Static binding, artifact freshness and repository-specific checks | [AgentX check](model/check.mjs); run with `--write-wrapper` after changing either JSON or the shared template |

Keep :18988 and direct-file opening working. The local `viewer` link exposes only shared renderer assets to the existing static server. Retain exact identities, fields and ports, local reference preview and comparison, cross-layer return, and mobile drawers. The extraction Task links current consumer evidence.
