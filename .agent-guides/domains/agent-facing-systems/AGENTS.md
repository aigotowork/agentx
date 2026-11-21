---
schema: document
status: active
description: Reusable method for coherent contracts, projections and agent discovery
links: [".sketches/design.md"]
---

# Agent-facing systems

Use this domain when designing, exposing or maintaining a product capability for agent callers through CLI, API, SDK, MCP, documentation or Agent Skills. The desired result is a complete caller task with consistent domain meaning, discoverable instructions and evidence from the actual boundary.

Keep domain policy in its owner, native declarations authoritative, and projections explicit. Choose only surfaces needed by real callers. This domain composes shared engineering and documentation methods; it grants no execution authority and requires no additional approval, delegation or global configuration.

For a new capability, start with [model](model.md) and [workflow](workflow.md). For an established capability, select all matching rows and reuse its sound model.

| Situation | Read / use | Result |
| --- | --- | --- |
| Choosing objects, boundaries, consumers or a source of truth; interpreting “shared schema” | [Model and ownership](model.md) | A proportionate capability model and source chain |
| Delivering a new agent-facing capability or adopting this method in another project | [Workflow](workflow.md) | One complete, observable caller path |
| Changing native operations, schema/metadata, effects/errors, derivation/generation schemes, CLI/HTTP/SDK/MCP mappings or provider versions | [Contracts and projections](contracts.md) | Owned declarations, selected derivation mechanisms and explicit adapter policies |
| Adding llms entries, agent recipes, Skill distribution or MCP client setup | [Discovery and caller experience](discovery.md) | Selective, executable guidance in the real consuming layout |
| Regenerating, changing public behavior, diagnosing drift or deciding whether delivery is complete | [Maintenance and evidence](maintenance.md) | Routed updates, bounded checks and recovery evidence |
| Starting files or recording a decision without an existing project form | [Adaptable templates](templates.md) | Small scaffolds with explicit placeholders |
| Learning from the originating Browser Environments implementation or finding its sources | [Browser Environments example](browser-environments.md) | Source-linked choices and evidence limits |

Normal product use follows that product's operating Skill or caller guide. Private module changes need only their existing native contracts; they do not require a network protocol, generator or agent integration. Authoring rationale and route walkthroughs live in the [design](.sketches/design.md), outside the normal reading path.
