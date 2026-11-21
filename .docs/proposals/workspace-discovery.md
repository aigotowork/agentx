---
schema: proposal
description: Resolve project workspaces from explicit selectors, inherited membership and project roots.
status: accepted
models: ["../roles-workspaces.md#归属目录与发现"]
links: ["../../.controller/roles-workspaces/tasks/workspace-discovery.task.md", "../../.sketches/workspace-directory/README.md"]
---

# Project workspace defaults

The user selected this priority and authorized design, documentation and implementation on 2026-11-17. The [Task](../../.controller/roles-workspaces/tasks/workspace-discovery.task.md) owns executable evidence. This refines workspace selection from the [original proposal](roles-workspaces.md); role sources, daemon scope, persisted membership and directory endpoint ownership retain their existing contracts.

## Accepted Design

| Priority | Input | Result |
| --- | --- | --- |
| 1 | Explicit `--workspace PATH` or `global` | Exact choice, including explicitly unassigned membership |
| 2 | Explicit `--dir PATH` | Closest ancestor Git checkout/worktree root; if none, closest project `.agentx/` or existing guidance-role boundary |
| 3 | Inherited `AGENTX_WORKSPACE` | Exact inherited path or `global`; bound callers replace this default with their persisted membership first |
| 4 | Invocation cwd | Same project-root discovery as priority 2 |
| 5 | No earlier result | Personal workspace `HOME/.agentx` (`USERPROFILE` on Windows) |

An unsuccessful explicit-directory discovery continues to inheritance and invocation-cwd discovery. An invalid explicit directory or selected workspace is an error. Empty inherited values are absent. Explicit/inherited `global` ends selection and is distinct from an absent default.

Git priority is across the ancestor chain: search for the nearest `.git` directory or valid `gitdir:` file first; only if no Git marker exists search AgentX markers. Linked worktrees and submodules use their own roots, not the shared Git metadata directory. Nested `.agentx` configuration remains available for explicit `--workspace`; it does not silently split a Git workspace. Existing `.agent-guides/roles` boundaries remain supported for non-Git projects. Recognition does not run Git, inspect branch state or change files.

The personal `HOME/.agentx` folder does not make all of HOME a project workspace merely by existing. It is a personal fallback anchor. `AGENTX_DIR` still selects runtime data/daemon scope and does not replace the personal workspace default. A project `.agentx/` marker selects its parent project directory. No workspace selection starts a daemon or creates directories. A not-yet-created personal anchor can be normalized through its existing ancestor and used for passive role lookup without creation.

Resolve cwd independently: explicit `--dir` selects the Provider working directory; otherwise preserve invocation cwd. An explicit workspace changes membership and role lookup without moving cwd. Existing resume/fork adopted membership and prior directory rules remain unchanged.

The launcher saves the resolved workspace in the native collaboration snapshot and exports the same value to each child as `AGENTX_WORKSPACE`, using `global` for an explicit unassigned choice. Descendant operations with a native identity use the saved membership as their inherited default. Changing cwd or mutable environment does not move an existing Agent between workspaces.

## Scope And Cases

Use the existing `roles::select_workspace` contract for CLI launch, roles, list, send and management routing. Keep filesystem recognition separate from role namespace discovery, preserve exact global role origins and default-endpoint declaring-workspace fallback, and do not migrate old participants or endpoint leases. Public argument grammar, native collaboration schema and daemon protocol do not change.

Required cases: shell inside Git; explicit directory in another Git overriding inherited membership; linked worktree `.git` file; Git ancestor winning over nested `.agentx`; non-Git `.agentx` project; existing Markdown-only marker; no target root continuing to inherited membership or invocation cwd; explicit global; blank env; personal fallback before directory creation; independent cwd; and child environment matching adopted membership. Build and checks use disposable resources. Existing installed binaries, daemons and Agent processes are not replaced by design acceptance.
