---
schema: document
description: Develop and observe terminal interaction, application state, actual output and retained checkpoints.
---

# Terminal UI development

Use for terminal layout, focus/selection, drafts, asynchronous updates, resize and restoration. Start with the product's [feature scenario](../../scenarios/feature-scenarios.md) and current contract. Pure domain checks need no terminal run. The application owns data, effects and process/terminal lifetime; an observer borrows only the selected access.

| Task | Read/apply |
| --- | --- |
| Design or optimize page structure, information density, visual hierarchy, selectors or contextual actions | [Workspace design](workspace-design.md): reduce recall, typing and navigation while keeping frequent work in coherent, readable workspaces |
| Change input, selection, editing, resize or async lifetime | [Interaction](interaction.md): stable identity, transitions and state/render consistency |
| Choose capture fidelity, framework seams or inspect Ink state | [Observation tools](tools.md): actual dependencies, completion boundary, original output and cleanup |
| Drive an external command without source hooks | [Black-box testing](black-box-testing.md): finite authored actions, waits, failure evidence and process ownership |
| Retain or read step history | [Observable feedback](../../scenarios/observable-feedback.md): three layers and original run identity |

Select a prepared tool from the installed release inventory and read its own usage and declarations. Native terminal runners and offline evidence formats may differ. A release carries method resources without activating every tool; a missing native dependency leaves that execution unverified. Do not substitute a fixed author demo for the consumer's scenario.

Use stable business identity for actions. Component IDs and cell rectangles locate observations within a run; they do not authorize effects. Read-only inspection provides no input authority. Prefer existing renderers and seams, retain original failed attempts, and select a real terminal window only when its input/font/compositor is part of the claim.

For a standalone Skill entry use [observable-evidence](../../skills/observable-evidence/SKILL.md). In harness-kit, the `observable-evidence` component owns shipped checkpoint/bundle, terminal and Ink/Code adapter documentation; select its manifest resource under `docs/tools/observable-evidence`. It is a method component with explicit public bindings, not a presumed generic command.
