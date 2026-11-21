---
schema: proposal
description: Compose configurable AgentX context through shared generation hooks for startup and compaction recovery.
status: accepted
models: [../model/README.md, ../architecture/runtime-hosts.md]
deliveries: [../../.controller/agent-context/delivery.md]
---

# Configurable agent context

Scope: a proposed context composition capability, investigated on 2026-11-14. Owner: AgentX participant/runtime composition. The user authorized implementation on 2026-11-14. [Usage and native contract entry](../agent-context.md) and [Delivery](../../.controller/agent-context/delivery.md) distinguish implemented behavior from Provider acceptance. [Discovery and earlier uncertainties](../../.controller/domain-daemon/feedback.md#compaction-context-recovery-2026-11-14) remain source-linked background.

## Need and current behavior

AgentX needs one way to generate participant-scoped prompt context at startup and after compaction. Users need to append their own instructions, replace or omit the built-in guide, and eventually include current team, role, task and skill information. Future domain modules should contribute through explicit generation hooks rather than duplicating text assembly in each Provider adapter.

Today [bootstrap rendering](../../src/runtime/bootstrap.rs) combines built-in instructions, participant/environment facts and configured `notes`; `bootstrap_exclude` can omit sections. [Lifecycle preparation](../../src/agents/lifecycle_service.rs) issues bootstrap receipts, while [HookClient](../../src/integrations/hooks/client.rs) renders against a daemon snapshot and confirms output after stdout flush. [Codex launch preparation](../../src/integrations/launch_args/codex.rs) combines generated bootstrap with developer instruction overrides supplied in argv. This code does not by itself establish preservation of instructions originally supplied only by a native Codex config file. Claude, Qoder and OpenCode have different compaction recovery paths; other adapters have different native capabilities.

## Recommended responsibility model

| Concept | Responsibility | Boundary |
| --- | --- | --- |
| Context request | Exact participant/session attachment, generation reason (`startup`, `resume`, `compact`) and effective originating configuration | A process-default participant cannot identify a sibling shared thread |
| Context facts | Current authorized participant facts and selected domain projections | Native daemon capabilities own managed state; configuration text cannot grant identity, permissions or membership |
| Context source | A named built-in generation hook, configured literal text, or a configured file | Internal hooks are typed module callbacks; this slice does not execute user commands |
| Context block | Source ID and generated text | Source provenance supports diagnostics; text is not an executable authority claim |
| Prepared context | An ordered, bounded set of blocks for this request | Preparing context does not acknowledge mailbox messages or prove Provider delivery |
| Provider adapter | Native event timing, prompt role/envelope, rendering and exact available output confirmation | The kernel does not depend on Codex/Claude JSON; XML labels cannot upgrade native prompt priority |

Dependency direction: domain modules expose typed facts or implement the required generation port; the application composition supplies them to the context composer. Provider adapters consume the prepared result. File reads, effective caller configuration and eventual subprocess execution remain originating-host effects; the daemon never reads a user's arbitrary template file or executes commands in its global environment.

## Shared composition flow

1. Resolve the actual participant and its current session/attachment through existing native fences.
2. Capture effective source selection from the originating invocation; obtain the domain facts required by those selected hooks through native capabilities.
3. Invoke selected sources sequentially in declared order. A source returns its blocks or no content. Hooks receive the generation reason and typed facts. Do not let a source mutate lifecycle, send messages or start another model turn.
4. Render and bound the result with source-aware diagnostics. Configured text and files are literal in the first slice; only the existing built-in renderer substitutes its known values. A general template language is unnecessary.
5. Adapt the result to the Provider's supported injection path; confirm only the exact effect supported by that path. Hook stdout flush is not proof of model consumption. Startup argv/config preparation needs its own honest evidence boundary rather than a synthetic hook-flush claim.

The common operation is conceptually `prepare_context(request, facts, sources) -> prepared_context`. An internal source has the conceptual contract `generate_context(reason, facts) -> blocks`. These are design signatures, not duplicate declarations; actual Rust contracts must be established beside their owning modules during implementation. Prefer a statically composed ordered callback list over a dynamic plugin loader, dependency graph or scripting runtime.

Startup and compact use the same mechanism. A compact call regenerates selected dynamic facts; it must not replay a cached startup snapshot or be suppressed by the existing once-per-session `name_announced` flag. Compaction can occur mid-turn, so restoration must not mark the participant idle or create an extra user turn. Native session identity and attachment rules remain unchanged.

## Configurable source selection

Recommended semantics: an absent source list preserves the existing bootstrap default; an explicitly supplied list replaces the default list; an empty list disables AgentX-owned prompt context. Users can therefore append content by retaining the built-in source, replace it with a file, or omit it altogether. Native Provider instructions remain separately owned and must not be erased by disabling AgentX context.

The selected TOML shape is shown below; current configuration and verification status are maintained in [usage](../agent-context.md):

```toml
[[context.sources]]
id = "agentx"
builtin = "agentx"

[[context.sources]]
id = "workspace-guidance"
file = "./agent-guidance.md"

[[context.sources]]
id = "preferences"
text = "Report progress in Chinese."
```

Each source has a unique ID and exactly one of `builtin`, `text`, or `file`. Order is significant for presentation but does not establish instruction authority. Relative file paths resolve against the explicit project/caller context, never the shared daemon cwd. Files are read at generation time so the compact result can reflect edits. Explicit missing/invalid sources produce a diagnostic; silently restoring a disabled built-in guide would violate user intent. Reject oversized content before injection rather than silently dropping identity or arbitrary final blocks. Exact finite limits and native transport-specific preview behavior belong in the implementation contract.

Initially the `agentx` built-in hook wraps the existing guide renderer and applies existing `notes`/`bootstrap_exclude` behavior once. Preserve their current defaults and exclusion semantics; new explicit source selection must not append `notes` a second time. Existing `hints` remain per-message guidance, outside context generation. Subsequent section overrides can be introduced when replacing a whole guide is insufficient; they are not required for this slice.

Separate native developer instructions from AgentX source selection. Codex `developer_instructions` / thread `developerInstructions` are adapter-owned native inputs, not additional user sources implicitly read and re-added after every compact. Their merge precedence and config-file-only preservation require a focused native configuration case before changing launch composition.

## Internal hooks and future domains

Register the built-in AgentX source at application composition. When team/role/task/skill features supply actual domain projections, register their generation hooks through the same port and expose stable built-in source IDs. No placeholder team or skill model is needed now. Source selection determines which hooks run; hooks use current authorized projections instead of interpreting prior model prose as facts.

For future skills, provide selected skill names, purposes and authoritative entry paths by default; do not dump every skill body into every prompt. For team/task context, prefer current role, assignment, goal, blocker and next action plus references. Compact is a refresh trigger, not an instruction to resume completed work or revive stale membership. Cross-domain snapshots must retain their source revisions when needed; do not claim atomic consistency across independently fetched projections.

External command hooks are deferred as requested. Adding them later requires host execution, explicit structured input/output, timeout/output bounds and failure semantics; it must not require replacing the composer or moving execution into the daemon.

## Provider mapping and Codex decision

Keep the kernel reason separate from native event names. Claude/Qoder compact SessionStart and OpenCode compaction hooks map into the same preparation operation, but their timing and native output placement differ. An observation-only precompact hook cannot be advertised as successful postcompact injection. A fallback on the next supported model-visible boundary must be documented with its delay; unsupported paths remain unsupported.

For Codex, [official Hooks documentation](https://developers.openai.com/codex/hooks/) identifies `SessionStart(source=compact)` as a developer-context injection path before the next model request. `PreCompact`/`PostCompact` are distinct events and not equivalent text injection channels. Existing static configuration and hook-injected history need separate treatment: native manual/automatic compaction must establish what remains model-visible before selecting full guide reinjection versus a compact refresh. The common composer must support fresh postcompact context regardless of whether static launch instructions survive.

First runnable integration should cover ordinary Codex and Claude, including compact during a turn; shared Codex needs thread-specific qualification. Other adapters can follow the same contracts, with support documented individually. Do not infer universal support from successful composition unit tests.

## Implementation structure and deciding cases

The current renderer remains the built-in source; a cohesive runtime context module owns source loading/composition/rendering. Participant lifecycle retains identity/receipt operations, domain modules own their projections, config owns effective declarations, and integrations own native delivery. Prefer extracting this seam over copying `bootstrap.rs` into a second renderer. No source relocation is required merely to introduce the contract.

| Case | Required outcome |
| --- | --- |
| Existing configuration without new source declarations | Existing built-in content, exclusions and notes retain meaning |
| Explicit file-only or empty sources | Built-in guide is replaced or omitted; native user instructions remain intact |
| Multiple hooks, empty output, repeated compact | Stable order; no duplicated blocks within one preparation; fresh generation for each real compact |
| A file changes or assignment changes between startup and compact | New content comes from current file/domain state |
| File failure, unknown source, oversized block | Bounded source-specific error; no silent fallback to disabled guide |
| Provider output fails or callback targets a sibling session | No false delivery confirmation or foreign identity injection |
| Ordinary/shared Codex with argv and native-config developer instructions | Exact merge/retention behavior established across manual and automatic compact |
| Native event only permits observation | No claim of postcompact model injection |

The bounded composer and Provider consumers are implemented; [Delivery](../../.controller/agent-context/delivery.md) owns current verification and remaining limits. This design does not add external command execution or require a service restart or deployment.

## Follow-up: instruction lifetime and runtime snapshots

Status: user-authorized lifecycle refinement, implemented in the [follow-up Delivery](../../.controller/agent-context-lifecycle/delivery.md). The earlier completed Delivery records the combined-source baseline. The user distinguishes fixed identity/rules from changing task/team state. Keep that distinction independent of native message roles.

### Contribution and lifecycle model

Use two contribution ports and three orchestration entry points. Fixed instructions are generated once for the logical Agent identity; runtime context is a fresh snapshot used at initialization and after compact. A module may implement either or both ports. Team membership/assignment changes are runtime facts; only a deliberately fixed role belongs in instructions. If identity or fixed role changes, treat that as an explicit reconfiguration instead of quietly refreshing fixed instructions.

Conceptual signatures below are design notation, not exported Rust declarations:

```rust
trait InstructionContributor {
    fn id(&self) -> &str;
    fn instructions(&self, agent: &AgentIdentity) -> Result<Vec<ContextBlock>>;
}
trait ContextContributor {
    fn id(&self) -> &str;
    fn snapshot(&self, request: &SnapshotRequest) -> Result<Vec<ContextBlock>>;
}
enum SnapshotReason { Initial, AfterCompact }

prepare_instructions(agent) -> FrozenInstructions;
prepare_initial_context(session) -> PreparedContext;
prepare_compact_context(session) -> PreparedContext;
```

The last two entry points call the same context contributors with different reasons. Contributors should return the complete current state by default, not different histories for each event. A later state-change trigger can reuse this port without creating another contribution interface. `SnapshotRequest` carries the exact native-qualified Agent/session reference and originating host context; domain implementations capture typed service clients rather than receive a generic database or expose Provider JSON. Production declarations must reuse existing native identity/session types.

### Entry guarantees

| Entry | Selection and timing | Required outcome |
| --- | --- | --- |
| Fixed instructions | Resolve fixed identity and selected instruction contributors before the first model request | Preserve native base instructions; retain the frozen result across native resume; do not re-read changing files at compact and call the result fixed |
| Initial runtime context | Native session is bound, before its first model request | Fresh task/team snapshot accompanies the first business prompt through a separate native context channel; context alone does not synthesize a user turn |
| Postcompact runtime context | Native compaction recovery boundary, before the next model request | Re-read current facts, check the same attachment, and supply a fresh snapshot; preserve active status and do not synthesize a user turn |

The user's CLI initial prompt remains the business request. It is not the transport envelope for hidden AgentX context. Hooks/plugins consume the same prepared context as a CLI-originating launch; they do not build an independent template or append application context to visible user text.

### Native delivery responsibility

Provider adapters select the supported message role and timing; contributors do not request a raw `system` or `developer` role. Codex fixed additions use native developer instructions without replacing its base prompt. Its initial and postcompact context should use native additional developer context at the corresponding supported SessionStart/first-request boundary. The current ordinary Codex startup path must be changed before claiming separate initial dynamic delivery. Claude uses its native additional-context channel. Pi/OMP use their custom model-context mechanisms; hidden display does not establish developer priority. OpenCode/Kilo compaction prompt contributions may participate in summary generation and cannot automatically be treated as verbatim postcompact developer messages.

Adapters must explicitly report unsupported delivery rather than claim developer equivalence or silently turn a context refresh into a user message. If the backend cannot preserve fixed additions across compact, restore the already frozen text through its supported channel; do not regenerate it from current files. Exact supported paths remain subject to per-Provider execution evidence.

### Snapshot and receipt semantics

Keep stable source/block IDs, deterministic source ordering, bounded output and explicit errors. Each dynamic result is a complete current snapshot labelled as superseding earlier AgentX snapshots for those sources. This is semantic supersession unless a Provider supports native replacement: do not claim an old message was physically deleted. Empty current state must explicitly clear prior task/team facts rather than silently omit a removed assignment. Domain-owned projections supply facts; arbitrary user/file/tool text must not acquire instruction authority solely through the rendering interface.

Initial and compact receipt gates must be distinct and session-qualified. An earlier instruction/bootstrap receipt cannot suppress the first dynamic snapshot or a later compact refresh. A repeated callback for the same native boundary should not append another copy; a new compact boundary must generate fresh content. Check the binding again before publishing asynchronous results, retain pending output on failure, and confirm only the native effect actually observed. Preserve the existing distinction between flushed hook output and model consumption.

Follow-up acceptance must cover: first request contains both fixed instructions and separate runtime context; changed task state appears after compact while fixed instruction text remains unchanged; resume preserves fixed instructions while refreshing runtime facts; removed assignments clear previous facts; stale-session callbacks are ignored; no extra user message/turn is created; unsupported Provider channels remain explicit. The [native declarations](../../src/runtime/agent_context.rs) now own the exact contracts; the follow-up Delivery owns actual verification and remaining limits.
