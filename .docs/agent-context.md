# Agent instructions and runtime context

Status: lifecycle split implemented; current verification is tracked in [Delivery](../.controller/agent-context-lifecycle/delivery.md). The [proposal](proposals/agent-context.md#follow-up-instruction-lifetime-and-runtime-snapshots) explains the lifetime model. Native interfaces live in [agent_context.rs](../src/runtime/agent_context.rs); persistence contracts live in [context_service.rs](../src/agents/context_service.rs).

For the complete source survey, replacement examples and command/message guidance inventory, read the [Prompt system reference](prompt-system/README.md), [Provider channels](prompt-system/providers.md) and [Hint inventory](prompt-system/hints.md). This page remains the concise configuration/lifecycle entry.

## Two lifetimes, three entry points

| Content | Entry | Lifetime |
| --- | --- | --- |
| Fixed identity, role, principles, methods and tool guide | `prepare_instructions` | Generated once, stored, then reused verbatim for the same Agent/native-session resume |
| Selected current facts: builtin peer list and explicit text/file sources | `prepare_initial_context` | Fresh snapshot before the first model request |
| Selected current facts: builtin peer list and explicit text/file sources | `prepare_compact_context` | Fresh snapshot after compact or at the next supported model boundary |

`InstructionContributor` and `ContextContributor` are typed internal callbacks, not shell hooks. Register contributors in application composition. They receive native participant identity and originating host facts; snapshot callbacks additionally receive the bound native session and reason. Domain implementations capture their own typed clients, never a generic database handle. Contributors do not select model roles, mutate lifecycle, send mail or trigger model turns. No external command execution or dynamic plugin loader is implemented.

Provider adapters choose a supported native channel. The first user prompt remains the actual business request; AgentX context does not synthesize another user turn. A fresh runtime snapshot explicitly supersedes earlier AgentX runtime snapshots, including removed assignments. This is semantic supersession unless a Provider offers actual message replacement; old history is not claimed to be deleted.

## Configuration

Use independent source lists:

```toml
[instructions]
sources = [
  { id = "guide", builtin = "agentx" },
  { id = "principles", file = "./agent-principles.md" },
]

[context]
sources = [
  { id = "runtime", builtin = "agentx-runtime" },
  { id = "assignment", file = "./current-task.md" },
]
```

Sources have unique nonempty IDs and exactly one of `builtin`, `file`, or `text`. Files resolve against the originating caller directory. Fixed files are read once; editing or deleting them does not change already frozen instructions. Runtime files are read again for each snapshot. Literal files/text are not interpolated or executed. The builtin guide alone uses existing template substitutions, `notes` and `bootstrap_exclude`. Its startup peer list is omitted from frozen instructions; `agentx-runtime` supplies the current peer list as dynamic facts. `hints` remains a per-message feature.

The builtin guide teaches [roles and directory teams](roles-workspaces.md#从哪里发现这些能力), including role discovery, inherited workspace launches, scoped/global member lists and lazy directory-default sends. The full startup and native subagent guides share that concise content. `agentx roles --help` owns shipped role-file setup; `agentx docs --cli` and `--help-all` consume the same command help. Updating the executable changes newly generated instructions; existing frozen sessions keep their adopted text and can read current help explicitly.

Launch guidance teaches one direct delegation command: `agentx launch --go --headless --title "Task title" --bind ".controller/xx/xx.task.md" --agentx-prompt "Assignment"`. Add `--role NAME` when needed; configured defaults are selected automatically. The builtin identity block shows the adopted workspace, Role and initial working directory beside the name and launcher. The [file binding contract](file-task-bindings.md) explains when bound documents are included in the first task message. Bootstrap does not teach profile discovery or native-argument forwarding. A missing default is handled through the launch diagnostic, rather than routine configuration preflight. The [guidance Task](../.controller/roles-workspaces/tasks/guidance.md#routine-delegation-guidance-correction-2026-11-17) distinguishes source changes from executable adoption.

CLI/environment equivalents use JSON arrays: `instruction_sources` / `AGENTX_INSTRUCTION_SOURCES`, and `context_sources` / `AGENTX_CONTEXT_SOURCES`. An explicit empty instruction string selects the builtin default; `[]` disables that list. With an explicit instruction selector, absent/empty runtime selection means no initial runtime content. A later empty snapshot explicitly clears prior runtime facts.

Compatibility: without the new instruction selector, the previous `context.sources` list is partitioned: builtin entries become fixed instructions, text/file entries become runtime context. Thus an absent/empty old selector retains the builtin guide; old `[]` disables both; a file-only selector still replaces the guide and now refreshes only runtime context. Use `instructions.sources` to freeze a file. Source ordering is deterministic within each lifetime; it does not change model instruction priority.

At most 64 sources per list and 64 KiB of rendered context per channel are accepted. Missing/unreadable runtime files, invalid UTF-8, unknown generators and invalid selections fail with diagnostics and do not silently restore defaults. Existing frozen instructions are loaded before changed instruction configuration/files are evaluated.

The earlier [custom bootstrap selectors and Bash hook proposal](proposals/bootstrap-prompt-customization.md) is withdrawn: the user selected existing instruction sources for static replacement and requested documentation only. `AGENTX_BOOTSTRAP_PROMPT`, `AGENTX_BOOTSTRAP_PROMPT_FILE`, `AGENTX_BOOTSTRAP_PROMPT_HOOK`, the proposed JSON module protocol and five-second external hook execution are not implemented. `--agentx-system-prompt` remains additive; replacing the builtin while retaining Role/additions uses an explicit instruction list without `builtin: "agentx"`, as shown in the [replacement reference](prompt-system/README.md#配置与替换).

## Native delivery and supported boundaries

| Provider | Fixed instructions | Runtime snapshot |
| --- | --- | --- |
| Codex ordinary | Native developer instructions, merged with user home/profile/argv instructions; never replaces Codex base instructions | SessionStart/first-input fallback and compact SessionStart use a separate developer additional-context message |
| Codex shared | Thread-qualified frozen instructions restored through the thread context channel | Exact thread attachment and first-input/compact hooks; no shared-process identity fallback |
| Claude | Frozen text restored through native additional context | SessionStart initial/resume/compact context, using existing actor/output receipts |
| Qoder | Frozen text restored through additional context | SessionStart including compact/clear; no compact idle transition or wake |
| Pi | Native append-system section when available; hidden-context fallback preserves a forced host prompt | Request-local hidden context; session_compact fetches fresh snapshot; stale replies ignored |
| OMP | Frozen text in hidden custom model context; never replaces the host system prompt | Main-agent request-local context and session_compact refresh |
| OpenCode/Kilo | v1 native system transform; v2 system-array additions | v1 synthetic part on existing input plus compaction context; v2 request/compaction context. Synthetic/hidden text is not claimed to have developer priority |
| Gemini/Antigravity | Existing additional-context channel restores frozen text | Initial supported boundary and next BeforeAgent boundary refresh; no new postcompact native event claimed |
| Cursor | Existing SessionStart additional context | BeforeSubmitPrompt refresh; no independent postcompact event claimed |
| Kimi | First-input hook restores frozen text | UserPromptSubmit hook refresh; no independent postcompact event claimed |
| Copilot | Existing SessionStart additionalContext | UserPromptSubmit refresh; no independent postcompact event claimed |
| Hermes | Existing pre_llm_call context restores frozen text | Refresh at pre_llm_call, including the next call after compression/session rebinding |
| Grok | Existing launch rules channel | No supported independent runtime channel: selecting runtime sources rejects launch explicitly |

Each adapter retains its native identity, output and mailbox boundaries. Plugin context refresh does not claim or acknowledge mailbox messages. Snapshot generation/host output is not evidence that the model followed instructions. Fixed data uses participant-incarnation storage and a tool/session/name-qualified resume alias; a fork with a different identity does not inherit another Agent's identity text. Initial-context receipts are independent of the legacy bootstrap marker and permit one live outstanding preparation. Failed generation releases its preparation; a dead hook can be recovered by a later native boundary.

Codex native home/profile preservation is bounded to `CODEX_HOME/config.toml` plus explicit argv; full project/system configuration discovery is not implemented. External Provider support and native execution evidence must be read from the current Delivery, not inferred from the table's source-level integration.
