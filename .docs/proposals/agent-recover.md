---
schema: proposal
status: accepted
owner: agentx
last_updated: 2026-11-19
description: Explicit CLI recovery for live local Agents stalled by model request errors.
models: [../model/model-request-errors.md, ../agent-classification.md]
deliveries: [../../.controller/agent-recover/delivery.delivery.md]
---

# AgentX recover

## Scope and selection

Recover asks a live local Agent to continue its existing Provider session after a stalled model request. It does not resume a stopped process, create a session, replay a user prompt or clear a model error. Managed Codex terminals report model request failures through native hooks. Both the original PTY host and the direct shared Codex terminal have a continuation adapter. Other hosts need equivalent observation and a supported continuation path before they can appear in the picker.

| Command | Target set |
| --- | --- |
| `agentx recover @luna @nova` | Exact local names, with no prefix expansion or Relay routing. Optional workspace and label filters further restrict the named targets. |
| `agentx recover --dir . --label CONDITION` | Interactive search and multiselect over matching eligible Agents. Without a directory or workspace filter, use the caller's workspace plus unassigned Agents, as with `list`. Space toggles; Enter reviews; a second Enter submits; Escape cancels. |
| `agentx recover --all [--label CONDITION …]` | Submit for all matching eligible Agents across local workspaces in the selected AgentX scope, as with `list --all`. |

`--all` cannot be combined with names, `--dir` or `--workspace`; labels may still narrow the explicit batch. `--dir` discovers the enclosing workspace; `--workspace` selects an exact workspace or `global`. Use the [existing label predicates](../agent-classification.md#过滤契约), including AND across repeated options and the existing missing-key rules. The daemon intersects label, workspace, name and recovery eligibility before returning candidates and reports a named Agent that fails the filter. An invocation with neither names nor `--all` needs an interactive terminal. Filters alone never imply execution.

## Eligibility and continuation

The daemon verifies the current generation, exact participant incarnation, Provider session, and unresolved model error episode. The reported failure must still be current, with no observed retry in progress, user cancellation, approval, exit, or indication the Agent has moved on. The host must also establish that the relevant turn stopped and the prompt can safely receive a continuation; a failed request or an old idle signal alone is insufficient. Treat the upstream error detail as diagnostic text. There is no HTTP-code whitelist: an eligible 403 can be retried manually after the operator fixes access; a 502 does not prove the host is idle. Manual recovery has no Inspector duration threshold or automatic retry schedule.

Keep the [Inspector mailbox continuation](../../src/inspection/store.rs) and its settings separate. Recovery submits one short continuation of the current task on the existing session, without draining pending mail. The PTY observes a post-failure Codex Stop; the direct host can also observe `turn/completed` on the matching failed native turn. Turn-end evidence is separate from the displayed status. A new model request, user prompt or interrupt invalidates it.

Freeze the selection by exact participant and failure observation. Admission rechecks generation, session, failure and selection filters. The PTY path checks prompt ownership and obtains a second one-use grant before pressing Enter. For the proven stopped failure, the [direct Codex adapter](../../src/integrations/codex/terminal/recovery.rs) accepts the exact thread in `idle` or `systemError`, consumes the ticket before `turn/start` and verifies the recorded input against its unique ID and content. The visible pane does not participate. Queued requests expire after 10 seconds if no host has claimed them and can then be explicitly requested again. Once claimed, an attempt cannot be issued again during the same unresolved error episode, even after a transport interruption.

## Results and evidence

Report each Agent separately. A queued request waits for the selected adapter; repeated commands can return the same request and any observed submission or uncertainty. Missing or changed targets report a skip. Distinguish request acceptance from host submission. Only a subsequent model-request success observation resolves the error; neither CLI exit nor a quiet terminal does so. An empty discovery result is a normal no-op. Missing or ineligible explicit names, execution failures, and unknown submissions produce a nonzero batch result without preventing other eligible targets from running.

Verify with a disposable managed Codex fixture: a failed model request stops, recovery submits one continuation in the original session, and a later successful request clears the error. Also cover an in-flight retry, approval or occupied prompt, process exit, a same-name successor or changed episode during selection, deduplication, unknown submits and label filtering. The [Delivery](../../.controller/agent-recover/delivery.delivery.md) records actual source and verification status.
