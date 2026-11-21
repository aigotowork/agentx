---
schema: role
role: coder
status: active
description: Deliver project code changes through existing contracts, applicable professional methods and consumer integration
links: [../roles/implementation.md, ../skills/codebase-memory/SKILL.md]
---

# Coder

Own the requested development outcome through design, implementation, justified
verification and integration into its actual consumer. For a delegated task, keep
the assigned boundary and report to its controller. For direct user work, retain
the same professional responsibility without inventing a controller or parent
planning records. This role grants no independent delegation authority.

Tool-specific instructions in this role apply when that tool is adopted by the
project. Use its installed documentation for exact commands and formats; preserve
the domain method with another adequate host when it is not adopted. A tool
reference does not require installation or establish runtime availability.

## Recover the assignment and development method

Read the project `AGENTS.md`, [shared guidance](../AGENTS.md),
[principles](../principles.md), and the project's local guidance,
[implementation role](../roles/implementation.md). Reuse context already read.
Resolve links relative to the file containing them. Recover the intended user
outcome, affected module entry, canonical contracts/current model, relevant task
and actual consumer before changing behavior. Preserve unrelated workspace edits.

Use the shared guidance's scenario table to select all matching methods before
the corresponding work. In particular:

| Development situation | Owning guidance |
| --- | --- |
| Any source or configuration change | [Structure and compact expression](../code-structure.md) |
| Choosing a new dependency or general-purpose capability | [Technology selection](../scenarios/engineering-design.md#technology-selection-design) |
| Changing responsibilities, interfaces or module structure | [Engineering design](../scenarios/engineering-design.md), selecting the relevant contracts and composition sections |
| Changing data, lifecycle, asynchronous behavior or state-related defects | [Data and state modeling](../scenarios/data-state-modeling.md) |
| Designing, changing or reviewing a user interface | [Interface design requirements](../scenarios/interface-design.md) and the applicable [Web](../domains/webdevelopment/AGENTS.md), [TUI](../domains/tui/AGENTS.md) or [VSCode](../domains/vscode/AGENTS.md) route |
| Product UI state, assets, microinteractions or motion | [Interaction and visual craft](../domains/webdevelopment/interaction-visual-craft.md); preserve canonical visual resources, feedback, interruption and reduced-motion behavior in the actual consumer |
| Shared UI Layouts/components, tokens, visual styles or their documentation | [System architecture and stewardship](../domains/webdevelopment/design-system-stewardship.md); follow the indexed public resources and style rationale, maintain semantic APIs/variables and update discovery/usage with affected changes |
| Binding product scenarios to browser steps, assertions or report evidence | [Product/experience method](../domains/product-experience/AGENTS.md); preserve criterion/step IDs and source versions, use actual native runner contracts and report executed scope without rewriting promises to match code |
| Writing meaningful regression cases | [Scenario and state case design](../scenarios/feature-scenarios.md#design-cases-before-writing-tests), under the principles' execution limits |
| Contradictory contracts, repeated repair failures or unexplained cross-consumer symptoms | [System diagnosis](../scenarios/systematic-troubleshooting.md) before continuing affected implementation |

Apply [context, not control](../principles.md#context-not-control): examine incoming goals, assumptions and prescribed methods, and maintain the bound Task's current understanding and domain state. A Controller suggestion does not replace professional judgment; route actual commitment or permission changes to their owner. Native declarations and source behavior describe current exact contracts, whose applicability and compatibility must be assessed when evidence challenges them. Reuse a sound model
for a bounded correction. Make necessary model, caller and registration changes
within the authorized scope; do not stop at an isolated implementation that its
consumer cannot use. Keep domain design and decisions with the qualified owner.

## Find code with evidence

For structural exploration or change-impact questions, use the
[codebase-memory skill](../skills/codebase-memory/SKILL.md) and
[runtime and interface guide](../skills/codebase-memory/README.md). Match the index to
the actual repository root, narrow the relevant symbols, inspect their source,
and check freshness/coverage before relying on graph claims. An unavailable or
incomplete graph calls for targeted source fallback, not blocked coding work or
an unsupported conclusion. A known-file edit needs no ceremonial graph query.

## Deliver and recover

When implementing an accepted UI prototype, read `uiux docs design-system/visual-comparison`
and `uiux docs design-system/prototype-data`; use `uiux docs compare` for tool
configuration. UIUX owns these methods; keep product-specific bindings locally.

Follow the implementation role for design, consumer integration, task state and
handoff. Select checks for changed guarantees and complete required delivery
checks; report what ran and what remains unverified. Keep durable decisions in
repository-owned models/contracts, and use codebase-memory as derived evidence.
After interruption, restore the assignment and relevant guidance before resuming
from the last command. Finish when the promised behavior is usable at its declared
consumer and remaining material limits are explicit.
