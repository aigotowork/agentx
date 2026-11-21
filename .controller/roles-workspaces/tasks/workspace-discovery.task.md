---
schema: task
status: completed
agent: toga
description: Implement the accepted project-root workspace priority and stable child inheritance.
---

# Project workspace discovery

Authority: direct user instruction to complete design/docs and advance implementation on 2026-11-17. [Accepted design](../../../.docs/proposals/workspace-discovery.md) owns decisions. Current sources: `config::roles::select_workspace`, `runtime::launch_profile::prepare_with_role`, `cli::commands::launch`, `runtime::launcher`, `agents::workspaces` and [usage](../../../.docs/roles-workspaces.md). Root and module guides apply. This is one bounded shared-selection correction performed inline; no subagents or separate review cycle are justified.

## Structure And Scope

Extract filesystem selection into a private `config::roles::workspace` module, retaining the public `roles::select_workspace` entry for all current callers. That module owns project marker discovery, inherited/default path normalization and selection order; role namespace lookup stays in roles. CLI and launcher remove the implicit workspace-to-cwd assignment. Child `AGENTX_WORKSPACE` emission and persisted membership already exist and remain authoritative.

Update project/module discovery links, usage/help and the existing discussion view to the selected semantics. Preserve default-directory endpoint fallback, global role origins, existing participants and resume/fork history. Candidate binaries may be built separately; do not replace live services or installed programs. No protocol, dependency, schema migration or independent workspace ID is required.

## Cases And Verification

Write meaningful selection regressions before behavior edits: Git versus AgentX markers, nearest/nested Git, worktree marker file, non-Git project, explicit selectors versus inherited membership, failed target discovery continuation, personal fallback without creation and no HOME-wide capture. Verify cwd and exported membership through focused composition/CLI cases. Run the existing relevant suites once after coherent edits, build a separate candidate and exercise actual CLI dry-run plus a disposable fake-Provider launch if compilation permits. Report unrelated compile blockers rather than altering other active work.

## State

Design and source implementation are complete, including shared CLI/runtime selection, independent launch cwd, management directory preview and bounded known-office discovery. Usage, native help and discovery links are updated. The first scoped config/roles suite passed 27 cases. Runtime membership composition passed one case; launch-profile composition passed four. Management office cases exposed one old assertion: its synthetic root contains `.git` and `.agentx`, so under the selected semantics it is now a known workspace. After correcting the assertion, all nine management cases passed. Following the Git-file syntax guard, the six affected selector tests were rebuilt and passed; the actual linked-worktree CLI check also passed. There are 41 distinct passing Rust regression cases, with six affected cases repeated after the guard change.

The initial default full-LTO package was stopped before publication to use command-local fast release settings (`CARGO_PROFILE_RELEASE_LTO=false`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`, `CARGO_PROFILE_RELEASE_OPT_LEVEL=1`). Only its exact own Cargo/rustc PIDs were terminated after checking parent and command identity. The final recipe `just package .bin/agentx-workspace-discovery` passed in 4m25s and its version smoke returned `agentx 0.7.31`. Candidate SHA-256: `45479ebb4bec2256582cb22ef58980ea55cfdb42922bd2bd33180f94e11d5cc4`. No repository release profile or live artifact changed. Build/test warnings concern existing labels work outside this assignment.

## Completed Local Outcome And Evidence

The standalone candidate is `.bin/agentx-workspace-discovery`. [Native receipts](../evidence/workspace-discovery/native.json) record three passing groups: project discovery with a real linked Git worktree; fallback/inheritance and passive lookups; two actual fake-Codex Provider launches. The parent retained its subdirectory cwd and Git-root membership; bound identity defeated an edited environment value; explicit target Git selected the child's new workspace; both Provider environments matched native saved collaboration. Each fixture used its own temporary HOME/AGENTX_DIR, and cleanup targeted only processes carrying that exact scope. This verifies local Linux native/CLI composition and Provider delivery, not paid-model behavior or other platforms.

[Browser receipts](../evidence/workspace-discovery/browser.json) retain six selected-rule cases, no page errors and no horizontal overflow at 390×844. The interactive page is a finite teaching model, independently of executable behavior. [Verification summary](../evidence/workspace-discovery/verification.json) records commands and support limits. `python3 -m py_compile tests/workspace_discovery.py`, scoped Rust formatting and diff whitespace checks passed.

Existing installed/default programs, running daemons, Agents and LAN services were not replaced. Existing global memberships and resume/fork history were not migrated. Remote role/workspace launches and shared Codex app-server stdio workspace launches retain their existing rejection contracts; because defaults now normally select a directory workspace, an unbound shared app-server launch does not silently downgrade to global. That host's workspace support is outside this bounded selector correction. Local terminal/headless consumers are the verified usable scope.

Current CBM daemon cannot accept clients; direct source reads provide the evidence. Earlier exploratory AgentX-before-Git and implicit-global fallback are superseded. Preview: `http://192.168.2.178:18976/`, exec session `50668`; retain its static server for discussion and preserve unrelated services.
