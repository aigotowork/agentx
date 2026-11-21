---
scope: Workspace and execution-directory concept discussion
status: selected
owner: toga
last_updated: 2026-11-17
related: ["../role-workspace-team.md", "../../.docs/roles-workspaces.md", "../../.inbox/workspace-directory.feedback.md"]
---

# Workspace and directory

The user selected the workspace priority on 2026-11-17 and authorized design, documentation and implementation. The [accepted Proposal](../../.docs/proposals/workspace-discovery.md) owns decisions; the [Task](../../.controller/roles-workspaces/tasks/workspace-discovery.task.md) owns execution/evidence; [usage](../../.docs/roles-workspaces.md#归属目录与发现) owns current rules. The [interactive view](index.html) explains a finite fixture and contrasts the selected design with earlier behavior. It never launches an Agent or reads live services.

## Working model

| Concept | Responsibility | Current representation |
| --- | --- | --- |
| Workspace | Stable collaboration membership, team discovery and directory-default recipient context | Canonical directory path (personal fallback may not exist yet), or global |
| Execution directory | Filesystem location used to start a process | cwd; launch `--dir` |
| Role definition namespace | Origin and precedence of reusable role definitions | Local/ancestor/global role source |
| AgentX scope | Daemon, database and shared runtime/configuration ownership | Canonical `AGENTX_DIR` |

One workspace may contain Agents executing in several directories. An explicit workspace and execution directory may be outside each other's filesystem trees. A runtime Agent has one adopted membership; changing cwd does not change it. A global role definition can produce a local workspace member. Filesystem nesting supplies lookup precedence, not recursive team membership or process isolation.

## Historical behavior before this change

The earlier resolver selected explicit `--workspace`, then discovery from explicit `--dir`, then inherited `AGENTX_WORKSPACE`, then discovery from caller cwd. Discovery recognized the closest `.agentx/roles/` or `.agent-guides/roles/` ancestor. It did not detect Git roots or a generic project `.agentx/` directory. Therefore an explicit dir outside a roles marker resolved to global, even for a bound Agent. For a bound caller, launch command setup first replaced the inherited environment value with persisted native membership; this protected against a stale or edited environment value.

There was a second coupling: launch defaulted cwd to the selected workspace path when `--dir` was omitted. Selecting or inheriting a team root could move a launch from the caller's current subdirectory to the workspace root.

The launcher already places the selected workspace path, or `global`, in each new Agent's `AGENTX_WORKSPACE`. This is a transport default for descendant commands. The native launch snapshot remains authoritative for a bound Agent; the environment is not identity or permission proof. `AGENTX_DIR` independently selects daemon/database scope.

## Selected resolution model

Keep `workspace` and `dir` separate. Let repository/project discovery supply a workspace only when the operation needs a default. Resolve in this order:

| Priority | Condition | Selected workspace |
| --- | --- | --- |
| 1 | Explicit `--workspace PATH` or `--workspace global` | Exact explicit choice |
| 2 | Explicit `--dir`, with a recognized project boundary | Nearest Git root; if none, nearest AgentX project root |
| 3 | Inherited `AGENTX_WORKSPACE` | Path or `global`; bound callers use persisted membership |
| 4 | Invocation cwd | Same Git-first project discovery |
| 5 | No project boundary and no inherited membership | Personal workspace `~/.agentx` |

For a target or shell cwd inside a Git checkout, use its Git worktree root. This makes all subdirectories in one checkout share a default workspace; a different linked worktree gets a different path and therefore a separate default team. An explicit workspace can intentionally join them. A nested AgentX marker does not override an enclosing Git root. `.git` directories and valid `gitdir:` worktree/submodule files are recognized. Only when no Git root exists does discovery use a project `.agentx/` or existing `.agent-guides/roles/` boundary.

This answers the child-launch cases: no `--dir` inherits the parent's persisted workspace; explicit `--workspace` always wins; explicit `--dir` inside another Git project joins that target project's workspace; explicit `--dir` without a project boundary continues to inheritance and invocation cwd. The parent Agent's own membership never changes. A normal shell with no matching project root falls back to personal `~/.agentx`, rather than implicit global.

Keep cwd resolution separate from workspace resolution: explicit `--dir` sets cwd; when omitted, retain the caller's current cwd. In particular, `--workspace` should select team membership without silently changing the working directory. This preserves the user's chosen work location while allowing automatic workspace discovery at a project root.

Pass the resolved workspace (path or `global`) in `AGENTX_WORKSPACE` for every newly launched Agent, as the current launcher already does. On AgentX operations from a bound participant, resolve membership from native state and overwrite any ambient value. Persist the same result in the launch collaboration snapshot. This keeps inheritance stable without making environment state authoritative.

Project `.agentx/` marks its parent directory. Personal `~/.agentx` is a fallback anchor and does not mark all of HOME as a project. `$AGENTX_DIR` remains the runtime storage/daemon selector; changing it alone does not change the personal default. Passive discovery and role lookup do not create the personal directory. Existing saved global memberships and resume/fork history are not migrated. These choices supersede the earlier exploratory AgentX-before-Git priority and implicit-global fallback.

## View and verification

The single static document uses native HTML, CSS and JavaScript, following the existing CLI explorer's dependency-free approach. Controls compare current and proposed resolution for a bound Agent or shell, explicit dir/workspace, Git roots, linked worktrees and AgentX markers. All paths are illustrative fixtures.

Start from this directory: `python3 -m http.server 18976 --bind 0.0.0.0`. Stop the owned server with Ctrl-C in its exec session. LAN URL: `http://192.168.2.178:18976/`; reload deliberately for updates. Server identity and executed browser checks are recorded after startup below. Existing AgentX services are unrelated and must be preserved.

Active preview: exec session `50668`, port `18976`, deliberately retained for the discussion. Earlier checks below concern the first view revision; the Task records current-view and native checks separately. Numeric LAN HTTP GET succeeded. Local headless Chrome exercised the initial directory discovery, nested marker and explicit override. A complementary narrow-view check exercised an actual cwd change to the external worktree while preserving adopted membership, a fresh explicit local membership with a global role, absence of page errors and no horizontal overflow at 390×844. These are finite teaching-view checks, not live AgentX runtime acceptance or proof of access from another physical device.
