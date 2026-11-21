---
schema: document
status: active
owner: project_maintainers
description: Architecture and engineering-quality reviews of projects, modules, interfaces and implementations
links: [ ../AGENTS.md, ./engineering-design.md, ../code-structure.md, ./data-state-modeling.md ]
---

# Architecture and Engineering Quality Review

Use this guide when asked to assess a project's or module's engineering quality, architecture, structure, dependencies, encapsulation, abstractions, interfaces or classes, or to propose a structural improvement. It is the review entry point for both independent and authorized delegated work. Apply it to the requested scope; a module review does not authorize a project-wide audit, implementation, dependency installation or live-system testing.

The outcome is a supported judgment and a proportionate improvement proposal: what is sound, what causes a concrete problem, what should change, and what evidence would validate that change. Review protects correctness and maintainability; it is not a default gate for every edit. A negative initial assessment is a question to investigate, not a conclusion to confirm.

## 1. Establish scope and review depth

When reconstructing scattered architecture or explaining a proposed change across scenarios and modules, use [design management](../domains/design-management/AGENTS.md) for the connected current/target views. This review still owns its assessment; a bounded local review reuses the existing model without a whole-product reconstruction.

Read the relevant project/module AGENTS.md, current assignment, canonical contracts and applicable design records. Identify the capability, consumers, owned state, public entry points, dependencies and compatibility constraints. Separate current source behavior from historical acceptance and future plans. Record material ambiguity and evidence limits in the existing task/report using the [Module artifact destinations](./project-context.md#keep-context-discoverable); a bounded review needs no new control system.

| Situation | Approach |
| --- | --- |
| One class, interface or bounded module | Work inline. Establish its public meaning and relevant caller/provider contracts; inspect internals and tests to the depth needed for the requested judgment. |
| Several modules with disputed ownership or dependencies | Build a compact responsibility/dependency map and trace representative cross-module use cases before recommending boundaries. |
| Whole project or broad engineering-quality review | Account for every maintained source area and every dimension below. Declare depth and sampling; an inventory is not a line-by-line audit. |
| Delegation explicitly authorized and independent workload justifies it | Follow [delegation criteria](../roles/controller.md#when-to-delegate), assign nonoverlapping scopes through files, and retain one synthesis owner for cross-module decisions. Do not create a second reviewer wave by default. |
| A review discovers a bounded implementation request already authorized by the user | Follow the applicable implementation/design guidance and verify the affected behavior. This guide adds no approval requirement. |

For complex or ambiguous reviews, use [project context guidance](./project-context.md#model-before-acting) to state the domain, stakeholders, boundaries, constraints, success criteria, assumptions and trade-offs before analysis.

### Time and quality

- Respect an explicit time or resource budget. Otherwise choose depth by scope, change impact and risk; do not invent a deadline or exhaustively inspect everything because tools are available.
- Prioritize boundaries with effects, shared state, broad consumers, untrusted input or frequent change. Sample a concrete extension and a failure path to test whether the design remains understandable.
- Gather only evidence that can change the conclusion. Do not start repeated full builds, broad live-provider trials or independent review cycles for a design-only question.
- Stop when material conclusions are supported, boundary conflicts are reconciled and the proposal is actionable. A time limit reduces coverage, not the standard for calling a claim verified. Record unexamined areas and remaining uncertainty.
- File counts, line counts, fan-in/fan-out and graph centrality locate candidates; they are not quality scores. Do not prescribe arbitrary file-size targets or a universal metric threshold.

## Persistent or recurring problems

Use [systematic troubleshooting](systematic-troubleshooting.md) when causes remain unclear, symptoms recur across consumers, exceptions grow, or responsibilities/contracts contradict one another. It owns outcome and constraint correction, model selection, discriminating observation, coherent correction and bounded verification. Reuse a sound existing model for a known local defect; do not infer a structural defect merely from attempt count.

Pause execution that depends on the disputed explanation and keep independent work ready. Retain source-linked views and the supported cause or explicit distinguishing hypothesis in the existing task/design before resuming. Use this review's [dimensions](#2-review-dimensions) and [semantic contract review](#public-contracts-and-domain-semantics) when that diagnosis needs an engineering assessment. A review-only assignment retains its scope; an already-authorized correction proceeds through implementation and focused acceptance.

## 2. Review dimensions

Use this matrix to select evidence. In a broad review, give each row a concise disposition: sound, material issue, insufficient evidence, or not applicable with a reason. A focused review may group or omit irrelevant rows with a stated scope. Do not manufacture a finding for every row.

| Dimension | Inspect | Judge against |
| --- | --- | --- |
| Solution selection and reuse | Required capabilities; custom infrastructure; platform APIs, existing dependencies and suitable maintained open-source options; compatibility and replacement cost | [Technology selection design](./engineering-design.md#technology-selection-design); use the comparison evidence below |
| Domain boundaries and layers | Responsibilities, vocabulary, state ownership and consistency boundaries; domain rules versus use-case orchestration, dependency wiring, presentation and infrastructure | [Domain-oriented structure](./engineering-design.md#domain-oriented-structure); each rule and contract has a coherent owner |
| Directory hierarchy and file granularity | Whether the physical tree expresses responsibilities, sub-capabilities and contract/implementation homes; crowded flat levels, unnecessary depth, navigation needed for one operation and files that change together | [Directory hierarchy as a module map](../code-structure.md#directory-hierarchy-as-a-module-map); assess comprehension and change locality, not file count or facade names alone |
| Encapsulation and exports | Whether intended directory/module boundaries match actual exports and imports; accidental helper exports, implementation/resource leakage, deep imports, mutable state and internal use of the public barrel | Deliberate consumer-facing contracts and enforced dependency rules; a folder or index alone does not establish encapsulation |
| Public contracts and type design | Responsibility, domain vocabulary, scenario completeness, protocol guarantees and precise input/result/failure types | [Semantic contract review](#public-contracts-and-domain-semantics), then implementation/type checks when needed |
| Service and Entity models | Service operations and their domain types; entity identity, relationships, methods, invariants and storage/transport mappings | [Public domain meaning](./engineering-design.md#public-contracts-and-domain-meaning) and [entity representations](./data-state-modeling.md#domain-storage-and-transport-representations); interfaces express owned domain capabilities |
| Protocols and SDK/package boundaries | Canonical schema/service definition, exported package, generated bindings, adapters and actual consumers | [Protocol-derived packages](./engineering-design.md#protocol-derived-packages-and-sdks); generation preserves a contract but does not prove its semantic quality |
| Classes, factories and implementation | Actual stateful objects and closure owners; constructor/acquisition effects, invariants, lifetime, mutation visibility, inheritance, composition and cleanup | One cohesive responsibility and explicit ownership; choose classes, factories or pure functions by behavior, not an OOP quota |
| Dependency relationships | Runtime, type-only, build/test and generated dependencies separately; cycles, reversed layer edges, globals, hidden I/O, public-boundary bypass and change propagation | [Relationship diagrams](#module-relationship-maps) with explicit edge meanings, permitted direction and source-checked discrepancies |
| Abstraction and reuse | Existing variation points; duplicated policy versus coincidental similarity; pass-through wrappers; oversized context objects; leaky abstractions and speculative extensibility | An abstraction must hide a cohesive detail or serve a present variation; preserve intentional platform/provider differences |
| State, failure and resource contracts | Legal transitions; cancellation and late results; atomicity; known/unknown commits; retry authority; concurrency; handles, disposal and partial acquisition | [State-machine selection and workflow](./data-state-modeling.md#when-to-use-a-state-machine); success, accepted work and completed effects remain distinct |
| Verification and evolution | Tests of public behavior and boundary failures; fixtures/fakes; dependency guards; discovery docs; files touched by a concrete feature or provider addition | Tests and guidance support safe changes; passing types or architectural allowlists do not prove runtime correctness or good ownership |

For TS/TSX public component contracts and structure, consult [TypeScript and React contracts](../domains/webdevelopment/implementation.md#typescript-and-react-contracts). For component-system quality, also use [component and system review](../domains/webdevelopment/review.md#component-and-system-review); add its experience section when assessing user-visible flows. Do not load frontend scenarios for unrelated server code beyond applicable language sections.

### Build versus reuse

Review against [technology selection design](./engineering-design.md#technology-selection-design). Inspect whether architectural requirements drove the selection, which standard mechanisms were researched for reuse, and why remaining custom infrastructure is justified. Compare the current implementation with suitable options using the evidence below; include migration cost when assessing replacement.

| Comparison field | Required evidence for a consequential recommendation |
| --- | --- |
| Functional and semantic fit | Exact supported behavior, runtime/platform, cancellation, concurrency, data precision, resource lifetime and failure/commit guarantees |
| Operational fit | Maintenance, compatibility, license constraints, dependency/native footprint and security/performance requirements relevant to this project |
| Integration cost | Public API changes, deployment implications, migration and regression surface, code actually removed versus new adapter complexity |
| Decision | Keep, simplify using platform/existing dependency, adopt an external solution, or investigate a candidate; explain the specific gap or benefit |

Apply the same [research and evidence standard](./engineering-design.md#research-and-decision-evidence) to replacement proposals. Missing selection rationale is an evidence gap, not proof that existing code must be replaced. Do not claim measured performance without measurements or recommend a framework merely because it exists.

### Public contracts and domain semantics

First assess what the public declarations, protocol and model promise. A module's responsibility and collaboration should be understandable in domain terms without reading its private algorithms. Keep this judgment distinct from whether the implementation satisfies the promise; inspect internals only when that further judgment or an unresolved source question is in scope.

When a finding requires reconstructing a missing model or deriving a changed contract, use [collaborative modeling](collaborative-modeling.md) within the review's scope. Its [check selection](collaborative-modeling.md#select-and-interpret-checks) helps trace necessary data and finite behavior without treating incomplete coverage as absence. Review does not authorize product implementation or create a mandatory second reviewer.

Locate the actual contract artifact: OpenAPI or the established complete HTTP contract and data definitions, the chosen RPC schema/IDL, or owned typed declarations for a same-process public interface. Confirm its authority, coverage and currency using [contract discovery](./api-contracts-and-sdks.md#locate-the-contract-before-judging-it). Missing/stale definitions are findings; private helpers need no protocol file merely because they call one another.

| Review question | Useful intermediate result to confirm |
| --- | --- |
| Is the responsibility clear and correctly bounded? | A concise capability/owner statement and its intended consumers; identify mixed or misleading responsibilities |
| Is domain expression complete for the declared scenarios? | Map required actions/outcomes to named operations and domain concepts; identify missing semantics or exposed implementation detail |
| Are Services and types coherent? | Connect Service operations, inputs/results/errors and referenced entity/value types; assess real capability boundaries rather than assuming one Service per Entity |
| Does the protocol express valid interactions? | Trace relevant accepted and rejected actions, state preconditions, results and effects; check identifiers, units, optionality and correlated variants |
| Do entities own the right behavior? | Identify identity/value rules, relationships, meaningful methods and invariant owners; distinguish domain models, database records and DTOs |
| For frontend boundaries, are state and actions a coherent protocol? | Connect authoritative inputs, events/requests, transitions and visible outcomes using [component/page protocols](../domains/webdevelopment/interaction-and-state.md#component-and-page-protocols); a local UI protocol does not require RPC |

Create or update the relevant [interface/type or interaction diagram](../documentation.md#architecture-and-interface-diagrams), linking canonical declarations rather than writing a second interface specification. Record conclusions and unresolved questions for the relevant concerns, including sound boundaries. These are confirmation prompts for the Agent, not a required report table or user sign-off at each row.

For a generated SDK or public package, trace the declared Service/protocol and data model through generation/adaptation, package exports and a real caller. Check the relevant [generation and compatibility results](./api-contracts-and-sdks.md#make-generation-repeatable); distinguish an ambiguous source contract from stale generated output, a mapping defect or incorrect consumer use.

### Interface and implementation checks

After establishing the public semantics, trace at least one real consumer and provider for a disputed implementation or substitutability claim. Check the selected [module implementation pattern](./engineering-design.md#module-contracts-and-implementation-patterns) against the present requirement and physical implementation homes. Split an interface when consumers need independent capabilities; do not create one interface per concrete class by habit. Keep DTOs at their owning boundary, and distinguish domain types from transport/storage representations where semantics differ.

For [dependency inversion and IoC](./engineering-design.md#service-contracts-and-dependency-inversion), trace source dependencies separately from runtime calls and wiring. Check whether policy imports or constructs a concrete provider, who owns the required port, and whether a conforming replacement preserves consumer behavior. An interface declaration or injection container alone does not demonstrate separation.

Treat external input as untrusted data until validated. `unknown` at a decoded boundary is appropriate; `as T` without validation, broad records used for known protocols, or correlated fields modeled as unrelated optionals require scrutiny. An absence of explicit `any` does not establish type safety. Inspect runtime validation, assertions and conversions, and use discriminated unions where valid states demand them.

Review [domain objects](./data-state-modeling.md#domain-objects-and-behavior) for coherent identity or value semantics, behavior and protected invariants. Review classes and closure factories by the same behavioral criteria. Where they own resources, make acquisition, borrowed/owned resources, legal calls, disposal and partial initialization failures visible. Check superclass assumptions only where inheritance exists; prefer composition when it preserves the contract more clearly. Stateless transformations may remain functions, and a factory returning a typed object may provide sufficient encapsulation.

Current interfaces do not prove that design preceded implementation. Use design history only if available and relevant; otherwise assess present consistency. Proposals should define ownership, types and signatures before recommending implementation movement. Analysis-only work does not require creating a skeleton in production source.

### Module relationship maps

Create or update a focused diagram comparing intended domain collaboration with actual module use. Choose node granularity deliberately: a module dependency view, a Service/type view and a file import graph answer different questions. Use the shared [diagram guidance](../documentation.md#architecture-and-interface-diagrams); label edges such as `imports/requires`, `calls`, `implements`, or `publishes/subscribes`, and distinguish composition/wiring from source dependencies.

Derive applicable relationships with existing import/dependency tooling, package declarations, call traces or focused source inspection. Record the inspected scope and relevant tool coverage/limitations. Static imports alone cannot establish all runtime interactions; resolve material dynamic/wiring questions from actual composition or traces rather than treating missing edges as absent behavior.

For dependency findings, distinguish module graphs from file graphs and type-only edges from runtime edges. A type cycle is coupling, not proof of runtime initialization failure. A broad dependency allowlist can accept a poor ownership model. A barrel that re-exports implementation while implementations import its types can conceal internal cycles; inspect declarations and exports before drawing a conclusion. Generated artifacts and parser omissions must not silently become source coverage.

Confirm whether edges respect the public domain contracts, intended callers and dependency direction. Retain the diagram or its reproducible generated source, conclusion and unresolved edges in the existing working record. A graph without semantic interpretation is only an inventory.

### Directory, contract and implementation alignment

Review the physical hierarchy and actual module behavior together. Follow [the directory design rules](../code-structure.md#directory-hierarchy-as-a-module-map); this section defines the evidence expected during a module design or implementation review.

| Review question | Evidence to inspect |
| --- | --- |
| Can a fresh reader explain the module and find a relevant sub-capability without memorizing filenames? | Actual tree, concise module guide and one public entry; compare named groups with the responsibilities of their files |
| Do filenames identify their subject and purpose under project conventions? | Domain names, meaningful purpose suffixes and public/private cues; flag misleading names or generic dumping grounds |
| Are role-specific directories placed at the correct scope? | Ownership and local descriptions for controller, documentation, component or adapter areas where present; check nested responsibilities without requiring every scope to contain every area |
| Do contracts and implementation have coherent owners and useful locality? | Trace a public operation from contract to rule/use case, concrete adapter and composition; identify needless jumps across global types/services/helpers collections |
| Is each child an actual public module or only a private grouping? | Declared exports and real consumers, including deep imports; do not infer public status from a directory or automatically demand an index everywhere |
| Is the intended encapsulation enforced? | Visibility, exact public exports, import/dependency rules, runtime and type edges; check whether a facade still exposes private machinery or leaves all implementation scattered |
| Is the chosen pattern proportional and the dependency direction correct? | Actual implementation variants, state/lifetime needs and consumer interfaces; compare against simpler function/module/factory alternatives |
| Does the structure support a realistic change? | Walk through adding one supported variant or changing one local rule; identify affected contracts/files and explain any unrelated spread |

Report a structural issue by naming the mixed responsibilities, misleading boundary or concrete navigation/change cost. Propose the target responsibility tree, public/private boundary and necessary contract/dependency changes before a move list. Distinguish a useful private regrouping from a true module split. A large count is a signal to investigate; a small count, deeper tree or added barrel is not proof of improvement. Do not prescribe one directory pattern for every domain or claim encapsulation is fixed solely because files were relocated.

## 3. Gather and qualify evidence

1. Map the requested scope: maintained directories, responsibilities, entry points and public contracts. Follow relevant AGENTS.md rather than loading all historical material.
2. Build the relevant [relationship diagrams](#module-relationship-maps); verify freshness, parse coverage, filters and truncation before using negative or exhaustive claims. Fall back to source reads when tools are unavailable.
3. Trace representative normal, failure and extension scenarios through consumers, contracts and providers. Read relevant tests to identify protected semantics and gaps; do not claim a test was run merely because it was inspected.
4. Verify material findings against exact source locations. Distinguish a demonstrated defect from a reachable risk, maintenance debt, an unverified hypothesis and a style preference.
5. Synthesize a target model that preserves required behavior and resolves cross-module ownership. Independent module proposals must not assign the same contract/state to different owners or introduce a new dependency cycle.

Use targeted static evidence for design judgments. A concrete uncertainty can justify a bounded observation, but does not by itself authorize tests or image reading; follow the [execution limits](../principles.md#quality-tests-and-images). Existing test failures and tool limitations should be reported with their actual scope, not used as a pretext for unrelated fixes or hidden behind a generic “checks passed.”

As each selected concern is examined, retain its actual conclusion and remaining question in the [method working record](./project-context.md#make-method-execution-explicit). Interface/Service/entity diagrams, interpreted dependency graphs and annotated directory trees carry intermediate results. Distinguish resolved/sound areas from unexamined or inconclusive ones instead of relying on a final generic “reviewed” statement.

## 4. Produce an actionable proposal

Scale the output to the task. A bounded review may be a short finding and signature sketch; a broad review should provide:

- A clear conclusion, review scope and coverage limits, including healthy boundaries worth preserving.
- A responsibility/dependency map and dimension coverage, with links to canonical source/contracts and detailed evidence.
- Prioritized material findings: source/symbol, actual behavior or change scenario, consequence, violated design principle, proposed owner/boundary/interface, and relevant trade-off. Label uncertain claims explicitly; do not bury correctness risks under stylistic observations.
- A target structure or dependency diagram where it improves understanding. Mark proposed interfaces as proposals; explain what moves, what consolidates, what remains public/internal, and why.
- A small sequence of independently verifiable improvements, preserving public CLI/API, wire/storage compatibility and lifetime/error guarantees where required. Name meaningful checks for each stage and which prior result each depends on.
- Alternatives considered, build/reuse decisions, unresolved questions and the next useful action. Do not assign calendar estimates without an explicit basis.

Recommended priority meanings: P1 for material correctness, safety, production usability or high-impact boundary risk; P2 for concrete maintenance/extension burden; P3 for optional localized improvement. Priority is not an automatic implementation blocker, and a structural P1 does not imply a proven production incident. Adjust terminology to the project's existing issue system instead of creating a competing one.

Keep current behavior and future recommendations distinct. Prefer a modular single deployment and targeted boundary repair when they meet the need; justify additional packages, layers, protocols, inheritance hierarchies or frameworks against present requirements. A directory move without changed ownership, contracts or useful locality is not an architectural improvement by itself.

When a finding proposes a new capability, product direction or delivery commitment, follow [feature planning](./feature-planning.md#confirm-direction-and-maintain-the-roadmap) for product fit and the user's decision before adding it to the committed roadmap. A review recommendation is not approval; already-authorized fixes keep their existing scope and authority.

## 5. Record and close

Write conclusions and proposals to the destinations in the [project Module artifacts](./project-context.md#keep-context-discoverable), linking existing records instead of duplicating facts. Keep execution state in the task, design reasoning in the design/report, and reusable behavior in this guide or its linked owner. Make the result discoverable through the relevant AGENTS.md.

Before delivery, check that source claims have evidence, the target dependency direction is coherent, material findings have a concrete remedy, unknowns remain explicit, and the reported verification matches actions actually taken. Use one proportionate check; do not start a separate review workflow for the review itself. Analysis completion means the requested assessment and proposal are delivered, not that proposed refactoring has been implemented or accepted.
