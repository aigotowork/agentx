---
schema: task
status: completed
agent: vali
description: Maintain the minimal agentx discovery, binding, launch and context tool.
result: Five-command resource-consumer interface implemented; 38 tests and live discover/status/context reads passed.
---

# agentx

The [native contract](../.tools/agentx/README.md) and [usage](../.tools/agentx/USAGE.md)
own current behavior. The user defined agentx as discovery, identity/binding, launch and
context recovery. It consumes original task state; agents maintain their task files directly.

The public surface is discover, launch, resume, status and mycontext. Removed the task-write
module, lifecycle/ownership/checkpoint operations and the previous session command namespace.
Status reads original Task metadata, Role/Skill source availability and supplementary agentx
observation. Mycontext adds retained Role instructions, Skill entry points, guidance and Task body.
Discovery combines effective directory Role/Skill definitions and directly bound agents.

Session YAML holds only its own bindings/settings and native identity references. agentx owns
runtime admission/activity, Codex owns history, and the Task owner defines task schema and state.
No runtime history, locks, cached task state, generated reports or task ownership claims exist.
Startup prompts are built in memory and no longer direct agents to tool-managed task writes.

The user-requested [retained experiment](../experiments/agentx/README.md) remains available.
Its task instructions and Role source/current binding were updated to direct Task maintenance.
The first-stage task remains blocked waiting for user continuation; no new agent was started.

Verification: all 38 temporary-workspace tests passed. Native `.bin/agentx` discovery found
3 Roles, 15 Skills and 1 bound agent in the experiment. Self-selected status read the original
blocked task and agentx listening observation; mycontext from `/tmp` recovered the full Role
and Task. Session, Task and Role bytes were unchanged by the three read commands. Missing or
invalid resources and unavailable agentx are covered without hiding remaining local context.
No new report files were retained.
