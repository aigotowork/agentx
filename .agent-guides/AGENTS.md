---
schema: document
status: active
owner: harness
last_updated: 2026-09-24
description: Scenario guidance and roles for independent and delegated work
links: [ ./principles.md, ./guidance-authoring.md ]
---

# Agent guides

Read [common principles](principles.md). In a consuming project, also read `.agent-guides/local/AGENTS.md` for project-specific guidance. Source authors follow the enclosing harness maintenance entry; edit source and rebuild instead of modifying installed copies.

Project additions: read [local guidance](local/AGENTS.md). Apply its relevant routes together with the shared guidance.

Before the corresponding work, read the sections selected by all matching scenario rows below. Scenario guidance applies to independent and delegated work. Select any authorized collaboration role separately; a role is not a prerequisite for reading scenario guidance. Read newly applicable sections when the task changes, rather than loading every guide at startup. User instructions and actual tool authorization take precedence; this index does not authorize delegation.

## Read by role and situation

### Professional role entries

Select a role when the task calls for its cohesive responsibility; these entries
compose the scenario methods below and work for direct or authorized delegated
assignments. Role documentation does not register a runtime or grant delegation.

| Responsibility | Entry |
| --- | --- |
| Design and deliver code changes through the actual consumer | [Coder](roles/coder.md), composing the implementation role |
| Diagnose persistent, recurring or cross-boundary failures and challenge unsupported assumptions | [Debugger](roles/debugger.md), using system diagnosis and evidence-driven observation |
| Domain/data models, Service contracts, Mock supply and requested client/transport surfaces | [Domain and contract designer and builder](roles/domain-contracts.md) |
| Complete prototypes with named compositions, local fixtures and operable paths | [Product prototyper](roles/prototyper.md) |
| Semantic design systems, shared UI, styles and prototype reconstruction | [Design-system designer and builder](roles/design-system.md) |

### Scenarios

| Situation | Read/action |
| --- | --- |
| Producing a static poster, visual composition or PNG/PDF artwork | [Canvas-design Skill](skills/canvas-design/SKILL.md), with [source and scope](skills/canvas-design/README.md); retain product interaction and information requirements outside the artwork. |
| Exploring unfamiliar code, symbols, callers or change impact with a code graph | [Codebase-memory Skill](skills/codebase-memory/SKILL.md); select the available wrapper/native interface and verify source coverage. |
| Explaining a new model, complex relationships, interacting states or multiple dimensions | [Visual model communication](./scenarios/interactive-design-communication.md): for new or complex models, compose professional visual forms such as Mermaid, SVG, Canvas, chart libraries or 3D renderers into a polished componentized HTML file; serve that file through LAN HTTP when shared access is needed |
| Designing or implementing any frontend page, including a bounded fix | [Continuous frontend preview](./domains/webdevelopment/continuous-preview.md): expose the actual page early, keep it available and current, and prefer stable use with manual refresh loading new code |
| Designing or reviewing logos, wordmarks, emblems, visual identities, concept alternatives, craft or multi-round design feedback | [Logo and visual identity](./domains/logo/AGENTS.md): meaning, references, construction, presentation and honest delivery; existing asset exports retain their owner. |
| Planning, producing or substantially reviewing technical articles, visual explanations, interactive experiments, small teaching applications or related demonstration materials | [Technical explanations](./domains/technical-blog/AGENTS.md): reader outcome, evidence, primary form, production and editorial judgment; a spelling fix needs no new brief. |
| Deriving native domain schemas/procedures, reusing existing types or checking complete caller data and result flows | [Consumable contracts](./scenarios/contract-modeling.md): one native source chain, representation mapping, caller supply and explicit enforcement obligations. |
| Providing a runnable Mock to a frontend through native contracts, or changing its fixtures, reset, host lifetime or consumer integration | [Mock provisioning](./scenarios/mock-provisioning.md): usable default supply, contract-derived bindings, owned domain behavior and bounded evidence. |
| Connecting scenario criteria, domain objects/operations and UI actions; checking binding, completeness, finite paths or recoverable guided demos | [Interaction modeling](./domains/interaction-modeling/AGENTS.md): source-linked three-axis slices, independent coverage and honest model/UI evidence. |
| Reviewing a tool/workbench's complete task, object scope, discoverability, precision, interruption or return experience | [Tool experience review](./domains/product-design/scenarios/tool-experience-review.md): task and cognition, native responsibilities, continuity and scoped findings; combine interface requirements below. |
| Reviewing sustained collaboration, proposed reassignment, goal drift, fragmentation, repeated checks or process cost | [Collaboration effectiveness](./scenarios/collaboration-effectiveness.md): trace the original outcome and consumer, test disputed requirements and change the owning work; routine small fixes need no review ritual. |
| Designing, implementing or reviewing a product interface, page, prototype, management console, tool or mobile/small-screen experience, including bounded UI changes | Required: [Product and interface design requirements](./scenarios/interface-design.md), before the work: layout first, useful information/action density, local disclosure, visual expression, semantic composition and token-based CSS. Read it directly; do not defer it behind a domain workflow. |
| Understanding or steering an evolving product/module through connected scenarios, architecture and interfaces; maintaining a feature across changes or reconstructing current versus intended behavior | [Design management](./domains/design-management/AGENTS.md): maintain current feature/model entries and owned design relationships, identify affected updates, and link native contracts and support; reuse the existing model for an unchanged bounded edit. |
| Reviewing the architecture or engineering quality of a project/module, including reuse choices, structure, classes, interfaces, encapsulation, abstractions or dependencies | [Architecture and engineering quality review](./scenarios/architecture-quality-review.md): primary review entry; select depth, collect evidence and propose proportionate improvements. Follow its targeted links as needed; reading it does not authorize implementation or delegation. |
| Responsibilities/contracts contradict one another, or sustained uncertainty, repeated cross-consumer symptoms or accumulating exceptions require diagnosis | [System diagnosis](./scenarios/systematic-troubleshooting.md): pause affected execution and establish a causal model before a coherent correction; independent work can continue. |
| Reviewing product capabilities/direction, proposing features, maintaining a roadmap or planning a selected delivery outcome | [Feature planning and delivery](./scenarios/feature-planning.md): use Roadmap → Proposal → Delivery → Task for planned capability changes; keep planning choices, design decisions and delivered support distinct, with owned design/contract/model updates. Small direct fixes can use their existing Task. |
| Applying the product design methodology (how-to-design), or turning a product idea, substantial feature or redesign into connected needs, scenarios, concepts, experience and contracts | [Product design](./scenarios/product-design.md): enter at the current question, keep one scenario source and a complete outcome slice, then select professional methods and checks as needed |
| Deepening product discovery, feature/interaction promises, requirement relationships, prototyping, evaluation or delivery quality | [Product design methods](./domains/product-design/AGENTS.md): select professional depth, including [feature and interaction promises](./domains/product-design/requirements.md#define-feature-and-interaction-promises); the root product-design scenario retains the connected method |
| Designing or substantially reorganizing a management console or workbench with multiple subjects, relationships and work paths | [Management product design](./domains/management-product-design/AGENTS.md): derive object navigation, contextual actions and page hierarchy from tasks, then implement and verify complete paths; bounded page/component fixes retain their existing Web route |
| Designing or writing unit/integration tests, including interacting states | [Scenario and state case design](./scenarios/feature-scenarios.md#design-cases-before-writing-tests): derive meaningful coverage before test code; apply the [execution limits](./principles.md#quality-tests-and-images) separately. |
| Maintaining a feature's user goal, meaningful steps, expected results or regression evidence | [Feature scenarios](./scenarios/feature-scenarios.md): one consumer-owned source, useful checks and explicit coverage |
| Capturing or interpreting UI state, rendering, temporal behavior or retained feedback | [Observable feedback](./scenarios/observable-feedback.md) |
| Adapting finite models, retaining model/source basis, reading check reports or selecting contract tools | [Modeling](./domains/modeling/AGENTS.md): checked meaning, supported tools and implementation obligations |
| Explaining downstream read/write or processing fan-out in a tool, API or workflow, comparing work strategies, or establishing inputs for capacity estimates | [Workload modeling](./skills/workload-modeling/SKILL.md): scoped quantities, step triggers, local work and shared accounting |
| Explaining data/query performance, join expansion, SQL lineage or prediction versus actual query work | [Data/query methods](./domains/data-query/AGENTS.md): query-specific grain, strategy, identity and coverage; combine workload modeling for cross-layer work |
| Developing or observing a terminal UI | [TUI](./domains/tui/AGENTS.md): interaction, framework/PTY boundaries, retained output and owned cleanup |
| Developing, debugging, packaging or accepting a VSCode extension | [VSCode](./domains/vscode/AGENTS.md): actual public host contracts, native/Webview boundaries and disposable-host evidence |
| Discovering or comparing Skills for a concrete task | [Skill research](./scenarios/skill-research.md): actual source/resources, supported selection and adaptation |
| Preparing a package release, Web/Worker deployment, service rollout or recovery | [Release and recovery](./scenarios/release-and-recovery.md): fixed artifacts, exact targets, receipts and meaningful verification |
| Deriving a selected module or cross-module contract from scenarios, reconstructing its model, or changing behavior against a baseline | [Collaborative modeling](./scenarios/collaborative-modeling.md): connect semantics, responsibilities, state, data supply and optional checks; keep pure presentation work on its existing UI route |
| Complex or ambiguous work; delivering a non-trivial feature; establishing a project; changing project structure, workflow, or context | [Project context](./scenarios/project-context.md): model the problem, maintain task state, and use [progressive derivation](./scenarios/project-context.md#progressive-derivation-and-externalized-reasoning) with local sketches when it reduces uncertainty |
| Applying a non-trivial design, review, planning or implementation method | [Method working records](./scenarios/project-context.md#make-method-execution-explicit): retain useful intermediate results, confirmed conclusions, open questions and the next action in the existing work context |
| Creating or maintaining managed document, directory or Task metadata | [Shared structures](./refs/xschema/common-schema.md); for Roadmap, Proposal, Delivery or coordination records, read [planning and Controller structures](./refs/xschema/controller-schema.md). Record authors need these structures regardless of collaboration role. |
| Recording a design, task state, operating rule, or reusable lesson | Use [placement rules](./documentation.md#artifact-placement-rules) to choose the owner and form, then consult the relevant local Module or [project Module](./scenarios/project-context.md#keep-context-discoverable) for concrete destinations; update an existing artifact before creating another |
| Choosing a technical approach, toolchain, library, framework or infrastructure solution, or preparing to implement a general-purpose capability | [Technology selection design](./scenarios/engineering-design.md#technology-selection-design): research reuse options after identifying architectural requirements and before substantial implementation |
| Designing, implementing or materially changing a command-line tool, its commands, arguments, options, configuration, results, help or usage documentation | [Command-line tool design](./scenarios/command-line-design.md): coherent grammar and configuration plus [results, effects and recovery](./scenarios/command-line-design.md#results-effects-and-recovery), including streams, exit status and interruption |
| Designing an end-to-end product, service, or cross-layer architecture | [Domain, module, and product composition](./scenarios/engineering-design.md#domain-module-and-product-composition), then [Service contracts and dependency inversion](./scenarios/engineering-design.md#service-contracts-and-dependency-inversion); use [Contract boundaries and integration](./scenarios/engineering-design.md#contract-boundaries-and-integration) for the relevant integration boundary |
| Designing or materially changing project structure, domain boundaries, contracts, or a non-trivial feature | [Types and contracts before implementation](./scenarios/engineering-design.md#types-and-contracts-before-implementation), [Domain-oriented structure](./scenarios/engineering-design.md#domain-oriented-structure), and [Directory hierarchy](./code-structure.md#directory-hierarchy-as-a-module-map) |
| Defining interaction between domains, modules, frontend/backend, services, APIs, RPC, or events | [Service contracts and dependency inversion](./scenarios/engineering-design.md#service-contracts-and-dependency-inversion), then [Contract boundaries and integration](./scenarios/engineering-design.md#contract-boundaries-and-integration) |
| Designing or reviewing a module's public semantics, Service operations, domain types or protocol/package surface | [Public contract design](./scenarios/engineering-design.md#public-contracts-and-domain-meaning), or [semantic contract review](./scenarios/architecture-quality-review.md#public-contracts-and-domain-semantics) for existing work; add [SDK/package guidance](./scenarios/engineering-design.md#protocol-derived-packages-and-sdks) when bindings or public packages are involved |
| Defining a bounded function, value type or small module interface | [Small module and value contracts](./scenarios/engineering-design.md#small-module-and-value-contracts): use native declarations and nearby behavioral meaning; deepen only unresolved boundaries |
| Designing, exposing or maintaining agent-facing CLI/API/SDK/MCP contracts, llms documentation, Agent Skills or caller discovery | [Agent-facing systems](./domains/agent-facing-systems/AGENTS.md): coherent domain meaning, native contracts, owned projections, progressive discovery and actual-consumer evidence; select only needed surfaces. |
| Checking API definition coverage, maintaining OpenAPI/IDL or generating and validating client SDKs | [API contracts and SDKs](./scenarios/api-contracts-and-sdks.md): authoritative definitions, data schemas, suitable tools, repeatable generation and compatibility/integration checks |
| Designing or reviewing module relationships, interfaces, dependencies or interaction protocols | [Architecture/interface diagrams](./documentation.md#architecture-and-interface-diagrams): create or update focused model views as working artifacts; use [relationship review](./scenarios/architecture-quality-review.md#module-relationship-maps) when checking actual dependencies |
| Designing or reorganizing directory hierarchy, filenames, submodules, public interfaces or implementation homes | [Directory hierarchy](./code-structure.md#directory-hierarchy-as-a-module-map) and [module implementation patterns](./scenarios/engineering-design.md#module-contracts-and-implementation-patterns); when assessing existing work, also use [directory/contract/implementation review](./scenarios/architecture-quality-review.md#directory-contract-and-implementation-alignment) |
| Understanding a domain or designing a solution through its objects; creating or changing domain resources, work items, design records, data/value types, relationships, invariants or operations | [Object models as the design basis](./scenarios/data-state-modeling.md#object-models-as-the-design-basis), then the affected [data and types](./scenarios/data-state-modeling.md#data-and-types) or [state transitions](./scenarios/data-state-modeling.md#state-transitions); reuse an unchanged model and use [representation mapping](./scenarios/data-state-modeling.md#domain-storage-and-transport-representations) when storage or protocols are involved |
| Designing or changing lifecycles, workflows, protocols, asynchronous UI behavior, resource lifetimes, or fixing state-related bugs | [Data and state modeling](./scenarios/data-state-modeling.md), including [when to use a state machine](./scenarios/data-state-modeling.md#when-to-use-a-state-machine), progressive modeling, tools, transitions, and invariants; apply to the affected behavior even for a bounded fix |
| Writing or refactoring code, markup, or structured data in any language or format, including small edits | [Structure and compact expression](./code-structure.md): preserve meaningful structure, isolate cohesive implementation details, and keep simple expressions compact |
| Designing, implementing, debugging or reviewing a web app, page, prototype or component, including UI/UX, design systems, component registries, motion and visual resources | [Web development](./domains/webdevelopment/AGENTS.md): select the relevant frontend scenarios; use its workflow for broader work and direct guides for bounded changes |
| Selecting, implementing or reviewing Stripe payments/subscriptions, looking up Stripe CLI/API behavior or upgrading API/SDK versions | [Stripe](./domains/stripe/AGENTS.md): select the fixed official Skill and current version/account contracts; keep project authentication and consumers local. |
| Selecting, building on, deploying to or operating Cloudflare; configuring Workers, storage, AI agents, email, Turnstile, DNS or Zero Trust | [Cloudflare](./domains/cloudflare/AGENTS.md): select official Skills by task and use project CLI/API workflows |
| Implementing or modifying `.ts` or `.tsx` modules, including small fixes and non-web code | [TypeScript and React contracts](./domains/webdevelopment/implementation.md#typescript-and-react-contracts); non-web work does not require the other frontend scenarios |
| Writing or changing `AGENTS.md`, guides, or context routing | [Agent entry points and context](./documentation.md#agent-entry-points-and-context) |
| Designing or maintaining principles, Role/Scenario/Skill guidance, entry routes or supporting content, including adapting external methods | [Governance content authoring](./guidance-authoring.md): select scenario design, writing, adaptation, composition and validation; new or materially changed content needs an owned sketch/design and user/Agent perspective analysis |
| Moving or renaming Rust modules/files while preserving behavior | [Rust module migration](./scenarios/rust-module-migration.md): complete relocation/wiring change set, then bounded checks |
| Producing a static composition, poster or PNG/PDF artwork | [Canvas-design Skill](./skills/canvas-design/SKILL.md); product prototypes retain interaction/usability requirements |
| Progressively refining actors/needs or mapping experience promises into executable flows and evidence | [Product-experience method](./domains/product-experience/AGENTS.md); canonical requirements and native runners keep their existing owners |
| Designing documentation layout or choosing the owner or location of sketches, designs, or local context, including within a page or component | [Layered context and local artifacts](./documentation.md#layered-module-context-and-local-artifacts), then [placement rules](./documentation.md#artifact-placement-rules): use concrete defaults for new Roadmap, Proposal, model and Delivery/Task content; preserve valid existing homes through actual artifact catalogs. |
| Creating or reorganizing a concrete artifact inventory | [Module artifacts](./documentation.md#module-artifacts): keep local resource descriptions in the owning directory's Module carrier; keep general templates in placement guidance |
| Creating or interpreting empty directories, or example filenames | [Structural placeholders](./documentation.md#structural-placeholders): use location and naming cues without inferring a form or active state |
| Writing document content, including designs, requirements, explanations, decisions, or task records | [Structured and model-based expression](./documentation.md#structured-and-model-based-expression) and [documentation conventions](./documentation.md#documentation-and-code-comments) |
| Writing comments for complex code | [Documentation and code comments](./documentation.md#documentation-and-code-comments) |
| Writing Markdown/plain text or YAML/YML | [Markdown and plain text](./documentation.md#markdown-and-plain-text) or [YAML/YML](./documentation.md#yaml), according to the output format |
| Matching a task to a profession, missing specialist capability, or constructing a reusable professional Agent | [Professional Agents](./scenarios/professional-agents.md): distinguish profile, method/tool prerequisites and runtime readiness; select Skills and publish scoped guidance before dependent work. |
| Considering task decomposition or parallel Agents, noticing fragmented dispatch/execution, adjusting coordinated work, or deciding verification ownership and stopping conditions | [Workflow management](./scenarios/workflow-management.md): choose useful coordination, connect contributions to the governing model and consumers, and bound verification; revise affected responsibilities before dispatching more local work. |
| Considering delegation or entering a significantly different domain | [Delegation criteria](./roles/controller.md#when-to-delegate); other controller sections apply only to authorized orchestration |
| Routine implementation or fixes within an existing design | Follow canonical contracts and the affected module guide; skip unrelated architecture guidance while still applying matching language, tooling, and documentation rows above |
| Repeated friction, unclear ownership or wasted work | Find matching evidence in the project’s local lesson/task records through its entry; update the owning rule within the authorized scope |
| Need history, evidence or a technical decision | Follow the relevant task or design link; skip unrelated history |

### Professional responsibilities

Select these focused entries when their responsibility fits the actual request, independently or within an authorized assignment. A role reference does not authorize spawning an Agent or installing its tools.

| Responsibility | Read/action |
| --- | --- |
| Deliver source changes through existing contracts and actual consumers | [Coder](roles/coder.md) |
| Diagnose persistent failures, model/contract contradictions or disputed task assumptions | [Debugger](roles/debugger.md), applying [systematic troubleshooting](scenarios/systematic-troubleshooting.md) |
| Design/build domain contracts, projections and matching Mock supply | [Domain and contracts](roles/domain-contracts.md) |
| Define product experience, progressively refine actors/needs or evaluate complete paths | [Product and experience](roles/product-experience.md) and its [focused method](domains/product-experience/AGENTS.md) |
| Produce a complete operable prototype for an experience question | [Prototyper](roles/prototyper.md) |
| Maintain semantic Layouts, components, tokens, styles and discovery | [Design system](roles/design-system.md) |
| Construct or adapt professional roles, methods, Skills or collaboration agreements | [Role and method steward](roles/role-method-steward.md) and [construction method](domains/role-methods/AGENTS.md) |

### Delegated roles

Use these routes only for authorized collaboration. At startup or after compaction, restore the applicable project rules, current assignment, role guide, bound task, and owned module context; add only matching scenario sections above.

| Situation | Read/action |
| --- | --- |
| Controller decomposing a selected change, assigning work, resolving a blocker or accepting delivery | [Controller](./roles/controller.md): own the original outcome and contribution model; assign professional responsibilities and accept both scoped results and their composition |
| Controller scheduling ready Tasks, handling dependency changes or reassigning an available worker | [Ready-work scheduling](./roles/controller.md#schedule-ready-work), then [closure and context reuse](./roles/controller.md#close-rotate-and-remember) |
| Worker implementing a bounded feature | [Implementation](./roles/implementation.md) |
| Create or materially revise an assignment | [Task records](./scenarios/task-records.md), then [task handoff](./scenarios/task-handoff.md): make the parent contribution, governing model, real consumer, owned design and receiving/integration responsibility recoverable. |
| Sending or reacting to a message | [Communication](./scenarios/communication.md): notify only for an action, real blocker or completion; ordinary progress stays in the task |

## Single sources of truth

| Information | Owner |
| --- | --- |
| Reusable behavior and role authority | The relevant scenario and role guides |
| Runtime, compatibility, permissions and resource limits | Project working rules and explicit task authority |
| Accepted product direction, capability priorities and decision links | The owning product/project roadmap |
| A selected change, its current-to-target design, internal decisions and linked Deliveries | The owning Proposal, with detailed design in the affected model topics |
| Delivery outcomes and stage dependencies | The owning Delivery plan |
| Task dependencies and next ready work | Canonical decomposition/dependency records; the ready queue is derived |
| Scoped professional capability and selected methods | Owning domain/profile entry and existing artifact map; method/tool readiness is separate from runtime availability |
| Workflow responsibilities, structural results and decision/recovery conditions | Existing Delivery or coordination model; domain detail stays with professional owners |
| Live agents, assignments and shared write coordination | Actual runtime backend plus the responsible Controller's direct binding records |
| Execution status, exact scope, current blocker and handoff | The relevant task record, with status recorded once in its chosen format |
| Public contracts and module responsibilities | Module guides and canonical declarations, linked from the task |
| Current domain meaning and explicit target/support differences | The owning model entry and linked topic/module designs; native declarations own exact interfaces |
| Why a rule changed | Trigger-based lesson with evidence and rule link |

During delegated work, keep ordinary progress in the bound task. Apply [communication eligibility](./scenarios/communication.md) before sending even a direct message to the controller; notify only an owner who must act. Use the project's authorized tooling and available tool or skill help for messaging syntax. Explicit recipient and communication constraints take precedence over generic broadcast examples.

Maintain this directory as working guidance. When a reusable failure appears, fix the relevant scenario or role rule and update the project’s relevant lesson record; link evidence instead of copying transcripts. Do not reread every lesson on every turn or add a review ritual.
