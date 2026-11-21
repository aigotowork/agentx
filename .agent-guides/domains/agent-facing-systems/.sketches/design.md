---
schema: document
status: active
owner: agent-facing-systems
description: Design basis, routing cases and validation limits for the reusable domain
links: ["../AGENTS.md", "../../harness-authoring/AGENTS.md"]
---

# Agent-facing systems guidance design

## Situation and outcome

The user wants future projects to reuse the model and experience behind this project's CLI/API/SDK/MCP, agent documents and Skills. The original outcome was an actionable domain method in Browser Environments' local guidance. It is now promoted into the reusable Harness domain while retaining the same method and evidence boundaries. It applies when exposing a real capability to agents or maintaining an existing agent-facing surface. A private utility, a prose correction or an ordinary browser task does not require this full method.

The originating Browser Environments project retains its task and implementation evidence in `.controller/agent-experience.md`; the [worked example](../browser-environments.md) names those external source paths explicitly. This design records guidance decisions only. No new delivery tracker, global policy, release authorization or mandatory delegation is introduced.

## Model and ownership

| Layer | Owns | Consequential boundary |
| --- | --- | --- |
| Domain | Identity, facts, invariants, use cases and lifecycle | An adapter cannot create a new domain object or change operation meaning |
| Native contract | Public schemas, operation semantics, stable failures and deliberate projection metadata | Native declarations remain authoritative; generated outputs are derived |
| Projection | CLI grammar, HTTP/RPC binding, typed client and MCP tool/result mapping | Shared meaning does not require identical syntax or lossless representation |
| Agent experience | Task selection, executable recipes, discovery and Skill packaging | Documentation and hints never grant authority or enforce admission |
| Evidence and maintenance | Compatibility, reproducibility, real-consumer observations and change routing | Typechecking or a generated schema cannot prove an external effect |

Shared guidance already owns general domain design, dependency inversion, API/SDK design, CLI grammar and guidance authoring. This domain composes those sources and adds the missing agent-system model: how semantics travel across surfaces and how an agent discovers and uses them without hidden context.

## Reader perspectives

| Perspective | Starting input | Useful result | Friction to prevent |
| --- | --- | --- | --- |
| Product owner | “Make this capability usable by agents” | A proportionate set of surfaces with working task paths | Building every possible integration before identifying a consumer |
| Maintainer | A new field, effect, operation or provider version | A clear owning source and affected projections/docs/checks | Editing generated output or updating a second schema registry |
| Fresh project agent | Project AGENTS and a bounded task | A recognizable route and a small usable next step | Loading the whole domain, inferring hidden contracts or following source-only references |
| External caller | Published entry, installed Skill or configured MCP | Exact inputs, results, effects and recovery | Confusing a documentation index with installation, permission or protocol discovery |

## Composition and tree

The domain entry selects all matching routes; the workflow is sequential only where outputs are dependencies. Model/contract sources guide implementers. Discovery and templates can be used directly for an established capability. The worked example is optional and links this repository's sources rather than copying operation definitions.

| File | Responsibility | Reason for separate selection |
| --- | --- | --- |
| `AGENTS.md` | Applicability, boundaries and routes | Keep initial context small |
| `model.md` | Concepts, source ownership and proportionality | Establish meaning before choosing a carrier |
| `workflow.md` | Inputs, decisions, artifacts and end-to-end outcome | Connect engineering and caller experience |
| `contracts.md` | Native contracts, effects/errors, projections and provider choice | Resolve semantic/technical boundaries together |
| `discovery.md` | Documentation, llms, Skills and MCP configuration | Different consumer and packaging concerns |
| `maintenance.md` | Change routing, evidence, recovery and done criteria | Make continued correctness actionable |
| `templates.md` | Compact adaptable scaffolds | Let a new project adopt without reconstructing the method |
| `browser-environments.md` | Source-linked worked example | Demonstrate choices and limits with current native artifacts |

## Adoption choices and limits

- Reuse native oRPC + Zod for the TypeScript scaffold because it is the user's concrete stack and supplies runtime schemas and type derivation. The domain model also applies to another maintained native contract; no universal migration requirement is added.
- Preserve provider/version selection as an explicit compatibility decision. This project chose MCP SDK1 because available oRPC MCP integrations require the oRPC2 beta line; the example must not recommend a forced beta upgrade.
- Treat `resource` in operation metadata as domain ownership, distinct from an MCP Resource with a URI and resource protocol methods. No generic resource framework follows from either term.
- Keep `llms.txt` as an optional proposal-based documentation entry and `llm.txt` as a requested alias. Keep Skills and MCP as separately installed/configured capabilities, with supported host paths verified before claiming discovery.
- Templates contain placeholders and illustrative signatures. They do not claim to implement a complete service or install a platform integration. No speculative DSL or generator framework is introduced.
- Do not copy upstream instructions or depend on retained research files for a portable method. Link official sources; local research is evidence of this case only.

## Route and scenario walkthroughs

| Input | Selected path | Expected action / exit | Failure or excluded boundary |
| --- | --- | --- | --- |
| New project wants agents to manage a stateful object | Local entry → domain entry → model + workflow; contracts/discovery as implementation begins | Choose the object, scenarios, owned contract and minimum useful caller path; deliver through one actual consumer | No requirement to expose CLI, HTTP, SDK and MCP together |
| Existing native operation adds an optional clearing patch | Contracts + maintenance | Preserve omitted/null/false semantics in native input, map CLI representation explicitly, regenerate and check affected consumers | Do not infer patch defaults from create defaults |
| Existing API wants a Skill and llms index | Discovery + relevant templates | Package usable recipes and self-contained references, register in a supported host path, verify the distributed layout | No required API or domain rewrite |
| MCP call validates success but rejects structured error | Contracts + maintenance | Identify provider output validation behavior, correct the projection envelope and check a real client | Do not hide the error in unvalidated text or modify domain errors to fit transport |
| Shared schema is requested across CLI/OpenAPI/SDK/MCP | Model + contracts | Keep one schema/semantics chain and explicit mapping losses/policies | Identical syntax, universal serialization or generated business behavior is not promised |
| Caller needs only a browser connection | Project operating Skill and caller recipes | Use the product; this authoring domain is unnecessary | Domain entry must not hijack normal browser work |
| Private helper function changes | Existing module guidance | Keep a native declaration and local verification | No protocol, capability manifest, Skill or generator required |

## Basis and source use

| Source | Inspected for | Adoption choice |
| --- | --- | --- |
| Shared harness-authoring scenario design, authoring, writing, adaptation, composition and validation | Usable local method, reader perspectives, ownership and evidence limits | Apply the authoring method here; link shared engineering methods from consuming guides |
| Shared native-contract, API/SDK and CLI guidance | Domain semantics, dependency direction, protocol choice, caller results and recovery | Compose rather than copy their detailed procedures |
| Project native models/service/contract, generator and package declarations | Actual canonical chain, effect metadata, generated outputs and pinned providers | Link exact sources in the worked example; keep generic scaffolds independent of its layout |
| Project llms entry, canonical Skill, aliases and caller/discovery/MCP docs | Actual discovery and packaging choices | Extract distinct roles and portable reference requirements, without requiring this project's paths |
| Official llms/Agent Skills/host/MCP sources linked from discovery and contracts | Format/platform/protocol ownership | Retain proposal and host/version boundaries; avoid automatic-discovery guarantees |
| Lead's executed integration and retained receipts | Actual selected-client lifetime behavior and the structured-error correction | Link observations by version and scope; do not generalize shutdown support |

No upstream instruction text was copied into the domain. Native code in the template is an illustrative scaffold using the selected libraries, not a redistributed external Skill or a claim of a working application.

## Validation and uncertainty

Static user/Agent walkthroughs completed against the authored route tree:

| Case | Visible path and finding | Disposition |
| --- | --- | --- |
| New project asks for agent support | Local route → domain entry → model/workflow selects one useful caller; contracts/discovery become conditional implementation routes | User obtains a complete scope without an implied requirement to build every surface |
| Existing contract adds patch clearing | Direct contracts + maintenance route identifies native schema first, then presence/null mapping and affected consumers | No second schema or creation-default leakage is prescribed |
| Product already works and needs a Skill | Discovery route + packaging/Skill scaffold can be used without loading the workflow or rewriting the domain | Standalone references and supported host registration are explicit prerequisites |
| MCP success-only output schema rejects errors | Contract result policy and maintenance failure table identify the actual client boundary and native error preservation | Worked example links the corrected policy and observed SDK case |
| Ordinary browser task or private helper edit | Local exclusion routes operating work to the product Skill and keeps private code on module contracts | Domain does not add authoring overhead to unrelated work |
| Multiple matching conditions | New operation plus Skill changes compose contracts, discovery and maintenance; no first-match exclusion | Each topic has one owner and a concrete intermediate result |

These walkthroughs establish visible routing and a plausible method, not observed fresh-agent adoption. No delegated reviewer or broad test run was used. The lead completed the scoped link/anchor and Skill-materialization check; the originating project receipt `.artifacts/agent-delivery-checks.json` records the checked scope. Product generation/build, actual consumer execution and publication checks are recorded in the owning task. Placeholder links inside template code fences are illustrative, not actual document references.

A substantive authoring defect was found in the design's initial relative paths: the `.sketches` depth was counted one parent too high. It was corrected before handoff and the final integration path check passed. The independent Skill copy also exposed source-only references in document frontmatter; the affected metadata was corrected, the packaging check passed, and the discovery method now explicitly covers metadata as well as body links. No blocking scope ambiguity remains. The principal recurring workflow friction is that similar words (`resource`, `schema`, `discovery`, `idempotent`) can hide different owners or guarantees; the model assigns each one a concrete boundary instead of adding more registries.

## Flexible derivation methods and object-oriented responsibilities

The consuming DancingAI user reviewed the Contract role and requested stronger methods for constructing the oRPC + Zod chain and connecting OOP design, while explicitly preferring general principles over prescribed tools, patterns or a bundled runnable reference. The affected situation is designing or changing a domain capability and its requested consumer surfaces. Existing public effect, projection and provider-compatibility policies remain applicable.

The missing distinction was between compile-time inference, runtime binding and build-time generation. Describing all three as generation could encourage unnecessary generated files, duplicated Controllers or a second operation registry. The contract method now identifies source ownership and Service mappings, then selects a mechanism for each actual surface, separates authored policy and records the usable source-to-consumer path. Tool choice remains contextual to consumers and installed versions; official documentation and project tooling supply exact APIs. The optional Browser Environments example remains optional. No new external checkout, reference application, generator framework or all-surface delivery requirement is introduced.

Object-oriented reasoning belongs in shared data/state modeling, beside domain objects and behavior. Its addition covers cohesion, mutation encapsulation, valid construction/reconstruction, behavioral subtyping, composition ownership, interfaces/abstract classes and actual variation points. It connects to existing Service dependency inversion and representation mappings rather than creating a parallel OOP method. Class, function and record choices remain based on behavior. The project-owned Contract role adds focused routes to both methods; reusable ownership remains in Harness source.

For the user, the intended benefit is a domain design and usable selected surfaces without a predetermined implementation architecture. For a fresh Agent, the role and domain routes expose concrete decisions while preserving selective reading. The effectiveness hypothesis is that naming inference/binding/generation and the remaining authored responsibilities prevents false automation claims, while the OOP questions make behavioral ownership assessable beyond field shape. This is method reasoning, not an observed Agent reliability result.

Static walkthrough cases:

| Visible task | Expected choice and boundary |
| --- | --- |
| Existing oRPC contract needs a native RPC Client and HTTP endpoint | Inspect installed providers and wire protocols; reuse supported links/Handlers and own Service/context mappings. Generate only artifacts the consumers need. |
| Existing contract needs MCP on an incompatible convenience integration | Compare compatibility and evaluate an adequate protocol SDK adapter; retain native schema/error meaning. No forced upgrade or external reference checkout. |
| An internal Service returns entities with methods and rich values | Own public value encoding and reverse construction where applicable; share types only when meaning/lifecycle agree. No claim that JSON restores class instances. |
| A subtype refuses inputs promised by its base capability | Identify the strengthened precondition and revise the subtype relationship or separate the capability; structural assignability alone is insufficient. |
| Independent collaborators share state or resources | Define ownership, borrowing, delegation and cleanup before selecting composition or another mechanism. No mandatory strategy, factory or DI framework. |
| A private pure function or bounded presentation change | Keep its direct native contract and existing route; do not invoke a full generation scheme or require a Class. |

Validation covers focused links/anchors, release contents and the installed consumer paths. Walkthroughs support interpretability and scope, while structural checks support delivery integrity. Actual multi-surface runtime behavior and future Agent effectiveness remain outside this documentation change.

## Promotion into reusable Harness guidance

The user requested a complete migration from Browser Environments `.agent-guides/local/domains/agent-facing-systems/` to the shared domain in `long@192.168.2.178:~/works/longwork/my-harness`. The original nine Markdown files, including this authoring design, are retained. The receiving owner is `harness/guidance/domains/agent-facing-systems/`; the consuming layout is `.agent-guides/domains/agent-facing-systems/`, produced by the existing release installer.

The method, template bodies, topic boundaries and original evidence limits remain intact. Adaptations are limited to shared ownership wording, relative paths after removing the `local/` level, discovery routing and portable treatment of the optional worked example. Browser Environments application files and raw receipts remain external to Harness; their paths are explicit provenance references, not links into the receiving repository. The example itself contains the relevant choices and observations, so the generic method does not require that external checkout. No application support or fresh browser execution is inferred from this documentation migration.

For the user, completion means the whole topic is available in the requested installed domain and remains part of future Harness releases. For a fresh Agent, the shared scenario index selects this domain for agent-facing contract/discovery work; the domain routes to focused topics, while ordinary browser use and private helpers retain their existing entries. Source, release and installed layouts must have the complete topic tree, and executable references must resolve in the consuming layout.
