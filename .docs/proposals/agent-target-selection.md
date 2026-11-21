---
schema: proposal
status: accepted
description: Share Agent target selection across the default TUI and CLI actions while retaining action eligibility.
links: ["workspace-discovery.md", "../agent-classification.md", "../../.controller/cli-selection.task.md"]
---

# Agent target selection

## Baseline and source status

The [root router](../../src/cli/router.rs#L272) opens the TUI only without arguments (or via the separate `--new-terminal` mode). It does not parse root TUI options. The [TUI source](../../src/tui/db.rs#L67) requests a scope-wide snapshot without a workspace filter; the [native projection](../../src/service/queries_store.rs#L132) includes live, remote and stopped Agents, orphans and a shared timeline. The TUI's Agent row has a working directory, which is insufficient to recover adopted membership. Accepting `--dir` in the router alone cannot deliver a workspace view.

[Attach](../../src/cli/commands/attach.rs) has only an optional exact name; its picker reads registered terminal endpoints and applies local tmux eligibility. [List](../../src/cli/commands/list.rs) and [recover](../../src/cli/commands/recover.rs) already share workspace discovery for their implicit team, but use different native matching paths. [Kill, resume and fork](../../src/cli/commands/target_selection.rs) share picker interaction and compose different inventories. Current help and native code remain authoritative until any follow-up lands.

Source implementation covers the implicit TUI, list, attach, recover and the kill picker. The [selection Task](../../.controller/cli-selection.task.md#target-filter-follow-up-finding) records verification and adoption separately. Historical-selector extensions remain a follow-up.

## Proposed CLI contract

Keep data scope, workspace membership and working directory distinct. Data scope (`AGENTX_DIR`, or `--scope` where already available) selects a daemon and its records. Workspace filters select within that scope. A process working directory controls where the Provider starts; it can differ from the Agent's workspace.

| Input | Meaning on an Agent query or selection surface |
| --- | --- |
| `--dir PATH` | Discover the workspace containing the directory using the [existing discovery order](workspace-discovery.md). A relative path uses the invoking cwd. `--dir .` can select the repo root from a subdirectory. Selecting a view does not change cwd, data scope or an Agent's membership. |
| `--workspace PATH\|global` | Select a canonical existing workspace path exactly, ahead of `--dir` discovery. A directory team view includes unassigned Agents as `list` does today; `global` selects only unassigned Agents. |
| `--label CONDITION` | The [existing label predicates](../agent-classification.md#过滤契约), ANDed across repetitions and intersected with the workspace and any explicit name. Keep the existing missing-key semantics. Label data currently covers local current participants; unavailable data does not count as a match. |
| `NAME` | An explicit exact Agent for the action commands; preserve existing command-specific lookup or `self` behavior on read commands. An explicit action name without an explicit workspace selector continues to work across workspaces. Picker text search only narrows displayed choices; it never becomes a prefix match for an action. |

| Command | Proposed behavior |
| --- | --- |
| `agentx --dir .` / `agentx --workspace PATH` | Open the default TUI with its Agent inventory scoped by the common selector; apply `--label` to current Agents once that shared query is wired. Keep bare `agentx` as the existing scope-wide overview. Root options belong to the implicit TUI entry; write `agentx attach --dir .`, not `agentx --dir . attach`. If combined with `--new-terminal`, carry the same selection into the opened TUI. |
| `agentx attach --dir . [--label CONDITION]` | Show a searchable list of matching local Agents whose tmux terminal can be attached; always let the operator select one if no name was supplied. For no-name invocations, prefer the same default workspace as `list`; provide `attach --all` for the scope-wide picker and reject combining it with `--dir` or `--workspace`. An explicit name and explicit filters must match together. |
| `list`, `recover` | Preserve existing defaults and outcome rules. `recover` accepts `--dir`/`--workspace` with explicit names or the picker; an explicit `--all` batch remains scope-wide and accepts only label filters. A filtered interactive picker does not become a batch action. |
| `kill` | Filter only the interactive candidate list; bare `kill` keeps its scope-wide picker. Explicit names, PIDs and `kill all` are separate and reject filter options. A filter alone never signals a process. |
| `launch`, `resume`, `fork` | Keep `--dir` as the new process working directory and fresh `launch --label` as assignment. If historical selection needs filters, use an explicit workspace selector for source history only; adopt membership from the saved source as today. Do not infer historical labels from a same-name current Agent. |
| `send`, `roles` | Reuse directory discovery with their existing intent: directory send addresses the workspace default recipient; roles queries definitions. Neither is a generic Agent filter. |

Use a short scope indicator in the TUI Agent list and attach picker. Treat text search, workspace/label filters, and action eligibility as separate stages. For an empty filtered attach picker, report no matching attachable Agents and show the selected scope; do not silently widen the search. Distinguish this from a missing tmux endpoint for an explicit name. Opening a picker or TUI requires a terminal.

## Engineering shape

1. **CLI owns input resolution.** Define one composable Agent-filter argument group for consumers that take the same options; commands can compose only the pieces they use. Reuse [cli::workspace](../../src/cli/workspace.rs) with the originating cwd and saved caller membership; normalize workspace choice to scope-wide, workspace plus unassigned, or unassigned-only. Keep parser-only flags and Provider args after `--` outside the filter contract. Derive flag help from the shared parser declarations.
2. **Native queries own matching.** Use the existing [workspace model](../../src/agents/workspaces.rs) and [native label query](../../src/agents/labels.rs#L74). Reuse their semantics in lightweight candidate projections; do not call `list` or use its cleanup/status/unread projection just to match Agents. Match recorded membership, never cwd. Current and historical queries can use different records while sharing membership semantics. Use exact participant references from native results; don't reconstruct them from rounded JSON timestamps.
3. **Each consumer owns eligibility and effects.** Attach intersects name/workspace/labels with terminal endpoints and revalidates the selected participant and tmux host before connecting. Recovery retains its exact failure/host checks; kill retains target review and its native fence. The TUI uses the same workspace choice in its Agent inventory and stopped Agent views. Treat orphan/relay diagnostics and timeline/search as scope-wide unless their own data can support trustworthy workspace attribution; label them separately and keep unknown-membership action rows out of a scoped Agent picker.

Exercise nested Git cwd, exact workspace paths, explicit `global`, an empty scoped picker, and same-name target replacement. Verify the scope remains stable through TUI refresh and new-terminal opening; help and invalid arguments must bypass terminal effects. Check repeated labels, explicit-name mismatch, exclusion of an out-of-workspace kill candidate and confirmation/cancellation. Keep recovery eligibility and explicit bulk semantics when applying the filter. This design does not require a parser rewrite, new database schema or a generic execution operation.
