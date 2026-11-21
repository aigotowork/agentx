---
schema: task
status: completed
description: Publish a project-local role and method steward with model-oriented, traceable design workspaces.
links: [./design.md, ../../.sketches/role-method-steward.md]
---

# Establish the role and method steward

## Outcome and boundary

A discoverable AgentX role can guide capability-gap analysis, collaboration modeling, method/Skill research, role publication and evidence-linked maintenance. It maintains a recoverable workspace and uses diagrams/tables for relationships and decisions. Deliver local guidance, a reusable workspace template, a runtime role definition and bounded verification. Actual worker launch, external Skill adoption and observed business effectiveness are outside this implementation Task.

## Work and next action

| Part | Current result |
| --- | --- |
| Design | Structure and decisions recorded in the linked design before publication |
| Publication | Role, construction method, workspace template and non-default TOML published; local/root entry routes connected |
| Verification | Native role resolution and launch configuration preview passed; static walkthrough recorded below; final local link/anchor and TOML checks passed |

This implementation Task is complete. No required implementation work remains. A subsequent real role-design assignment can use the published entry and retain bounded behavioral evidence in its own topic; that observation is not claimed here.

## Completion criteria

- The actual role prompt routes to usable methods, workspace ownership and existing professional/research guidance.
- The template exposes scenario, collaboration, alternatives, source evidence, design maturity and adoption without duplicating Task status.
- Routine changes can reuse existing topics and avoid full research/review cycles.
- A real CLI resolves the role; any preview or runtime limitation is recorded separately.
- A fresh Agent can discover this topic and distinguish published files from observed behavioral support.

## Issues and evidence

| Issue | Resolution or remaining limit |
| --- | --- |
| Shared guidance is installed product content | Compose it by reference; keep all new operating instructions local |
| Work, design and adoption can be confused | Task owns work progress; the design owns maturity and exact adoption evidence |
| A diagram can imply unsupported behavior | Label current, selected, illustrative and observed claims; put guards/evidence beside transitions |
| Current scope has a usable high-reasoning profile | Reuse the existing role profile without editing shared runtime settings |
| Initial link check found two new paths with one excess parent traversal | Repaired both method links; one checker false positive treated an existing directory link as a file and was corrected |

## Static consuming-path walkthrough

These are source/interpretation checks performed by the implementer, not an independent Agent trial or proof of model compliance.

| Concrete request | Actual entry and selected path | Result and boundary |
| --- | --- | --- |
| Build a specialist in an unfamiliar field | Role's creation route → construction method → Professional Agents + research route → workspace template | Scenario, collaborators, judgments and constraints precede method selection; missing expertise remains explicit; no generic stack is prescribed |
| Repair prototype-to-component data handoff | Role's overlap route → shared scenario/model → linked existing domain owners | The design's sequence preserves prototype baseline, contract-owned view/Mock supply and design-system reconstruction; ordinary exchanges need no steward relay |
| A candidate Skill is discoverable but its tool is absent | Research route → source assumptions/prerequisites → adoption/evidence table | Catalog presence does not imply usable capability; affected work records the missing prerequisite and resolution owner |
| Correct one broken role link | Role's bounded-maintenance route → existing owner | No mandatory new topic, complete research loop, extra Agent or trial |
| Publish an updated role while an old instance is running | Native role usage + separate adoption record | Files may be published while runtime adoption stays unobserved; no existing instance is implicitly refreshed |

## Native checks and evidence scope

Executed from the repository root with `AGENTX_DIR="$PWD/.agentx"` and the existing `.bin/agentx` on 2026-11-16:

| Check | Result | Supports |
| --- | --- | --- |
| `profile list --json` | Existing `codex-gpt-6-1-sol-high` uses Codex `gpt-6.1-sol`, high reasoning | Configured prerequisite exists; no profile mutation |
| `roles show role-method-steward --dir "$PWD" --json` | Exit 0; local role, expected profile, `default: false`, absolute prompt-source header | Real role resolution and prompt-file binding |
| `launch --dir "$PWD" --role role-method-steward --dry-run --json` | Exit 0; `action: launch_preview`, selected role and project workspace | Configuration preview only; no worker launch or adopted runtime claim |
| Local document/TOML inspection after path repairs | Exit 0; 9 Markdown files checked for local links/anchors, role TOML parsed, 6 Mermaid fences inspected | Discoverable reading routes and basic document structure; no Mermaid rendering claim |

Mermaid fences and model meaning are inspected statically; no rendered-diagram acceptance or empirical role effectiveness is claimed. No external Skill installation, native source change, rebuild, shared-method edit or running-service replacement was required.
