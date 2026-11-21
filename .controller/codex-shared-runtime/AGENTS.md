# Shared Codex runtime Delivery

Read root [AGENTS.md](../../AGENTS.md), the [Delivery](./delivery.md), [controller record](./controller.md), and your assigned Task. Recover only the relevant canonical design and native declarations linked there. This is authorized implementation work; the earlier design-only task remains historical evidence.

## Roles and professional basis

| Assignment | Professional entry and method | Decision scope |
| --- | --- | --- |
| architecture | [Rust contract design](../../.inbox/rust-contract-design/domain/AGENTS.md), [engineering design](../../.agent-guides/scenarios/engineering-design.md), [architecture review](../../.agent-guides/scenarios/architecture-quality-review.md) | Layering, native production contracts, cohesive file/module skeleton and cross-domain contract decisions |
| native-service | [implementation](../../.agent-guides/roles/implementation.md), [service contracts](../../.agent-guides/scenarios/engineering-design.md#service-contracts-and-dependency-inversion), [state modeling](../../.agent-guides/scenarios/data-state-modeling.md); use the available openai-docs skill for native product facts | Native daemon lifecycle, discovery, compatibility, transport and backend generation |
| identity-messaging | [implementation](../../.agent-guides/roles/implementation.md), [data/state modeling](../../.agent-guides/scenarios/data-state-modeling.md), [collaborative modeling](../../.agent-guides/scenarios/collaborative-modeling.md) | Identity attachment, persistence transactions, claim/delivery/recovery semantics and generic activation policy |
| bridge-composition | [implementation](../../.agent-guides/roles/implementation.md), [contract boundaries](../../.agent-guides/scenarios/engineering-design.md#contract-boundaries-and-integration), [CLI design](../../.agent-guides/scenarios/command-line-design.md) | Native bidirectional routing, lifecycle arbitration, bootstrap/hooks and selected launcher wiring |
| integration | [feature scenarios](../../.agent-guides/scenarios/feature-scenarios.md), [contract boundaries](../../.agent-guides/scenarios/engineering-design.md#contract-boundaries-and-integration) | Independent scenario/test authorship, fixture composition, scoped execution, build evidence and acceptance gaps |

These routes reuse existing professional methods. They do not authorize speculative framework construction, additional workers, or work outside this Delivery. Spawned workers may not delegate further. The controller is zomi; use your actual runtime name for every agentx command.

## Shared implementation rules

- Design and expose native Rust contracts before behavior. Consumers depend on owned contracts, concrete adapters implement them, and the launcher composes them. Reuse existing types and storage where their semantics match.
- The architecture owner publishes file ownership and the usable contract/skeleton handoff. Implementation workers may investigate existing code immediately, but must not write behavior against guessed sibling interfaces.
- Each worker owns its Task and assigned source/design leaves. Coordinate a real overlapping export/module edit directly with the current writer. Do not overwrite concurrent work, invent global default identity, or broaden all-provider launch behavior to accommodate Codex.
- Keep interface definitions singular in production source once materialized; update the design sketch to reference the implemented source rather than allowing it to drift into a second source of truth.
- Domain workers author meaningful cases. The integration owner runs tests/builds, except a native-service investigation may execute a clearly bounded disposable protocol probe to resolve a specific external-runtime fact. Do not run broad checks per worker or repeat passed checks without a relevant change.
- Keep installed tools, credentials, project environment declarations and user daemons unchanged. Isolated native subprocesses may use disposable homes/projects and fixed loopback responses. Do not invoke daemon management until its service paths and effects are demonstrably isolated.
- No Git metadata is present. The controller saved the pre-implementation source archive and hash manifest under `.agentx/.tmp/deliveries/codex-shared-runtime/`. Use that basis for bounded baseline comparisons; do not initialize Git or restore unrelated files.
- Completion, a contract that releases a waiting consumer, a real blocker or an actual ownership conflict warrants a targeted agentx message. Routine progress belongs in the Task. Requests require a reply; include the reply id. Completion must name the usable result, evidence limits, model updates and resource release.

Follow [communication](../../.agent-guides/scenarios/communication.md) and [handoff](../../.agent-guides/scenarios/task-handoff.md). While awaiting an agentx response, finish your turn; do not poll or sleep.
