---
scope: Role construction, professional methods, skills and collaboration integration
status: discussion
owner: project
last_updated: 2026-11-16
related: [../.agent-guides/scenarios/professional-agents.md, ../.agent-guides/scenarios/skill-research.md, ../.agent-guides/domains/harness-authoring/AGENTS.md, ../.docs/roles-workspaces.md]
---

# Role and method steward: discussion basis

This note preserves the exploratory proposal and initial discussion for the user's requested role-building and maintenance entry. Current selected design, implementation state and support are owned by the [maintenance entry](../.controller/role-method-steward/AGENTS.md); proposed structures below are historical unless selected there. The note itself is not evidence of runtime adoption or improved Agent performance.

## Outcome and scope

Enable a user or Controller to turn a professional capability gap into a usable role that fits the project's actual workflow, stack, collaborators and tools. Maintain the professional basis through task feedback and relevant upstream changes. A role name or a list of technologies is insufficient.

The entry should support creation, extension, method research, Skill selection/adaptation, maintenance and retirement. A bounded method gap may need only an existing guide update or Skill; it does not automatically justify a new role or Agent instance.

## Inspected foundations

| Source | Existing responsibility |
| --- | --- |
| [Professional Agents](../.agent-guides/scenarios/professional-agents.md) | Match gaps, construct a professional basis, publish guidance and establish scoped usability |
| [Skill research](../.agent-guides/scenarios/skill-research.md) | Discover via skills.sh CLI, inspect source/resources, compare methods and record supported adoption choices |
| [Harness authoring](../.agent-guides/domains/harness-authoring/AGENTS.md) | Design, write, adapt, compose and validate guidance from user and Agent perspectives |
| [Local guidance](../.agent-guides/local/AGENTS.md) | Project-specific role and scenario routes |
| [Role runtime contracts](../.docs/roles-workspaces.md) | TOML role definitions, prompt paths, profile selection and runtime snapshot behavior |
| [Contract role](../.agent-guides/roles/domain-contracts.md) | Native contracts, presentation props and contract-backed Mock supply |
| [Prototype role](../.agent-guides/roles/prototyper.md) | Complete interactive prototype and presentation fixture handoff |
| [Design-system role](../.agent-guides/roles/design-system.md) | Component/Block reconstruction and shared visual resources |

The inspected files already cover substantial parts of the method. The proposed gap is an explicit operational entry and cohesive project adaptation. A comprehensive external standards or Skill catalog survey has not been performed; this note does not claim a universal standard or a best external framework.

## Responsibility model

| Concept | Owns |
| --- | --- |
| Role | Professional accountability, judgments, boundaries and completion criteria |
| Collaboration scenario | Participants, handoff inputs/outputs, shared decisions and change propagation |
| Professional method | Domain decisions, working sequence, intermediate artifacts and verification rationale |
| Skill | A reusable capability selected for a concrete gap |
| Project binding | Actual stack, canonical sources, artifact homes, commands and tool prerequisites |
| Runtime instance | Identity, assignment, actual access and adopted instruction snapshot |

Keep these concepts separate without requiring a file for each. Stable role guidance references shared methods; project adapters own local differences; live assignments remain with the Controller/runtime.

The proposed steward owns capability-gap analysis, role composition, research/adaptation records, discoverable publication and maintenance proposals. The Controller owns business priorities, staffing and assignments. Domain owners retain substantive domain decisions. If expertise is missing, research the basis and record uncertainty rather than asserting specialist authority. Installation or guide changes do not confer deployment or messaging permission.

## Proposed working loop

1. Recover a concrete desired outcome, environment, current workflow and capability gap. Decide whether to reuse, extend or create a role, or supply only a method/Skill.
2. Establish collaboration boundaries with affected roles: input sources, owned decisions, outputs, consumers, change handling and decisive completion conditions. Link shared workflow facts rather than duplicating them in every prompt.
3. Research professional methods against the needed judgments and project constraints. Prefer maintained authoritative sources and actual tool contracts; use Skills as candidate implementations of methods rather than the entire knowledge base.
4. Inspect selected Skill source and necessary resources. Record source revision, assumptions, useful mechanism, dependencies, attribution/license, conflicts and local adaptation. Upstream instructions remain research material until selected under the task's authority.
5. Publish the smallest usable role guide and project adapter; add a role TOML only when runtime discovery is needed. Keep unconditional prompt content small and route specialist details by scenario.
6. Exercise a representative handoff and a decisive boundary case at proportionate depth. Record documentation readiness, tool/runtime readiness and observed task performance separately. Claim no business effectiveness from static review alone.
7. Revisit on demonstrated failure, workflow/stack change, broken prerequisite or relevant upstream change. Inspect the affected impact before adoption; preserve source/revision and a recoverable prior version using existing mechanisms.

Small updates enter at the affected step. Do not require the whole loop, a fixed candidate count, a scorecard or independent review for every wording change.

## Representative collaboration

For an initial product UI, the prototype owner supplies the complete interaction, page/Block inventory, visible/hidden states and presentation fixture. The design-system owner supplies visual boundaries, component composition and reusable resources. The contract owner uses that baseline to define props/callbacks, data meaning and matching Mock supply; the design-system owner wires the reconstruction against the agreed supply. Shared visual foundations can proceed while contracts are clarified. Record an unresolved ownership conflict before dependent integration, and update affected owners when a baseline changes. This is a description of inspected local guidance, not evidence of an executed new collaboration.

The steward designs and maintains these role agreements; it does not become a mandatory intermediary for routine professional exchanges.

## User and Agent fit

The user should be able to request an outcome or describe a recurring failure without already knowing the correct profession, Skill name or framework. The output must explain the chosen role boundary, useful methods, local tools, handoffs and actual limits.

A fresh Agent should enter through the role's actual prompt and project AGENTS.md, find current authoritative sources, select only applicable methods and know which missing inputs block work. It should not need the author's memory, every collaborator's full prompt, the complete research history or tools that were merely mentioned in a guide. Changed role files do not retroactively refresh running instances; adoption must respect the runtime's saved instruction behavior.

## Design workspace and accountable process

The user's follow-up establishes a requirement: the steward must maintain discoverable design drafts, research/material preparation, alternatives and decision rationale, concerns, progress and state. Directly writing a role prompt without a recoverable design basis does not satisfy this outcome. The concrete layout and lifecycle below remain proposed.

Maintain one owned design workspace per coherent role-design or collaboration-design topic. When several roles share a workflow, keep their common scenario and handoff decisions together and link individual role designs to that source. Reuse an existing topic instead of opening a new workspace on every maintenance request.

### Artifact responsibilities and layout

The steward should establish and maintain the shallowest useful layout under the owning module's existing exploratory/design area. A substantial topic may use the following example; this is not a requirement to precreate every file or directory:

```text
<owning-module>/.sketches/<topic>/
  AGENTS.md       Topic scope, reading routes and authoritative source locations
  design.md       Scenario, responsibilities, alternatives, decisions and open issues
  research.md     Sources, inspected revisions, relevant mechanisms and adoption analysis
  evidence/       Retained trial inputs/results when actual trials warrant files
```

Keep active work state in the existing owning Task using the [native Task structure](../.agent-guides/refs/xschema/common-schema.md#task), with a link from the topic entry. If no Task exists and implementation starts, create one discoverable `.task.md` record in the owner's task area. Do not copy Task status into a parallel status document. A short analysis can keep design and research in one file; split them when their independent use makes navigation easier. Reference upstream materials rather than copying them unless a retained snapshot has concrete value.

Published role guidance remains in the project's actual role-guide home, with runtime TOML in `.agentx/roles/` when needed. Shared reusable methodology belongs to its canonical authoring source. Exploratory notes and trials are not automatically authoritative instructions.

### Minimum recoverable design basis

For a new or materially changed role, record:

- The initiating need, representative scenario, expected contribution and why an existing role or bounded method update is insufficient.
- Current workflow, collaborators, responsibility boundaries, input/output handoffs, project constraints and authoritative sources.
- Relevant research: actual sources/revisions, what was inspected, useful methods, required tools and material limits. Separate source facts, design assumptions and observed results.
- Meaningfully different alternatives, selected approach, reasons, rejected/deferred paths, trade-offs and known concerns. Record decisions that change responsibility, method, dependencies or adoption; do not create a separate decision record for every wording change.
- Intended guide/Skill/project bindings, representative validation, actual evidence and remaining gaps.
- Current next action and unresolved questions, linked to the owning work record; adopted guide/version and affected runtime scope when adoption occurs.

The goal is a concise, reviewable rationale and evidence trail, not a transcript of every exploratory thought or an archive of all browsing output.

### State semantics

Keep work progress, design maturity and adoption separate:

| Dimension | Proposed representation | Transition basis |
| --- | --- | --- |
| Work progress | Existing Task states: planned, active, blocked, completed, cancelled | Maintain the native Task record; blocked requires a concrete reason and what resolves it |
| Design maturity | A section in the owned design: exploring, candidate, trial-supported, superseded or retired | Candidate has coherent boundaries/methods and explicit gaps; trial-supported cites a defined task, version, environment and observed result |
| Adoption | Evidence-linked adoption section naming guide/artifact, scope and instruction snapshot where applicable | Publication, runtime use and business outcomes require their own evidence; no inferred global readiness |

These design-maturity labels are local proposal vocabulary, not additions to AgentX's native Task or Proposal enums. No formal approval step is introduced by the label candidate. A decision that requires user input stays unresolved until answered; other authorized independent work can continue.

Update records when a decision changes, evidence arrives, a dependency blocks progress, an artifact is published/adopted or a handoff occurs. Name the prior decision being superseded and its impact; preserve material provenance using existing version control and source links. Completion of a design-only Task may leave runtime adoption unperformed, provided that was its declared scope.

### Applying the process to the steward itself

Use this topic as the starting design basis for the steward. Before implementing its prompt, make its own responsibility contract, research routes, artifact ownership, state-update rules and bounded validation explicit. Then publish a role entry that directs future instances to the actual workspace rather than relying on the current conversation. A fresh instance should recover the current question, selected approach, evidence gaps and next action without redoing completed research.

## Open questions and smallest next slice

- Publication ownership: shared guidance is installed harness content. Before implementation, locate its canonical authoring source; keep project-specific changes local rather than treating installed copies as upstream.
- Distribution scope: begin with this project unless the user selects a reusable shared role; project bindings must stay explicit if the role later serves multiple projects.
- Skill availability: distinguish catalog discovery, inspected source, local files and actual host discovery. No new Skill was downloaded, installed or executed for this discussion.
- Evidence: the usefulness of this combined entry is a design hypothesis. A good first trial is to reconstruct the existing prototype/contract/design-system collaboration and address one demonstrated gap before expanding the role catalog.
- Initial implementation candidate: one local steward role with links to existing professional-construction, research and authoring methods, a compact project collaboration map and one bounded trial. Avoid a new service, registry, general workflow engine or duplicate method library.
