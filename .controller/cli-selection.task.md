---
schema: task
status: completed
owner: tara
description: Add explicit searchable choices for finite CLI catalogs and omitted lifecycle targets.
---

# Outcome and bounded design

## Explicit kill help mismatch (2026-11-24)

During idle Agent cleanup, the local AgentX 0.7.31 `kill --help` advertised `@NAME`, but `kill --json @NAME` returned “Agent not found” for live local Agents. The same batch succeeded using bare names. [Kill help](../src/cli/commands/kill.rs) and [explicit target lookup](../src/cli/commands/stop.rs) disagree; align the advertised syntax with target resolution in CLI maintenance. The cleanup did not change command behavior.

## Target filter follow-up finding

`attach --workspace .` fails at argument parsing: [AttachArgs](../src/cli/commands/attach.rs) accepts only a target name. The [shared picker module](../src/cli/commands/target_selection.rs) composes command-specific inventories and shares terminal interaction, but does not expose common target filters. `list` has workspace discovery and label predicates; `recover` has the label predicates and a workspace default for interactive choices, but no explicit workspace or directory selector. `resume/fork --dir` selects the new process working directory, so it cannot also serve as a target filter.

The user accepted the [target selection proposal](../.docs/proposals/agent-target-selection.md) and authorized implementation. The first slice shares the filter between list, attach, and the default TUI, including the new terminal route. Reuse exact native label selection and saved workspace facts. Use additive native query variants for filtered TUI and terminal reads so an earlier daemon cannot silently answer an explicitly filtered request with a full snapshot. The timeline stays scope-wide; scoped Agent inventories exclude unclassified orphan actions and historical rows when live labels are requested. Compile and executable acceptance are deferred to the shared build.

Source handoff: the shared parser is in [CLI workspace selection](../src/cli/workspace.rs); [native query types](../src/service/queries.rs) cover filtered terminal candidates, dashboard snapshots and stopped views. Use the authored native cross-consumer case in [queries_store.rs](../src/service/queries_store.rs) and the [CLI picker fixture](../tests/cli_target_selection.py) against the later candidate. The existing bare dashboard and named attach path remain available without newer filtered queries. Do not infer live adoption from the source pass.

The user continued the same work with recover and kill. The shared argument group now carries only directory, workspace and label, so each command retains the meaning and conflicts of its own bulk options. Recover retains native workspace/label rechecks at submission; kill filters the read-only picker via native label snapshots and sends the exact confirmed rows to the existing kill path. Existing explicit kill names, PIDs and `kill all` remain separate. The picker fixture now covers noninteractive/bulk rejection and a same-label Agent from another workspace. Historical resume/fork filters remain outside this slice.

## Source-only follow-up

The user's latest refinement adds `a` as the `attach` shortcut and displays every attach/kill/resume/fork Agent choice as `name [role]`, including stopped snapshots and kill review. The role comes from the adopted launch context; missing role data displays `[-]`. Search includes the display label, while effects still receive the exact name and retain existing fences. Root/attach help and [CLI contracts](../src/cli/AGENTS.md) reflect the refinement. These source changes are newer than the installed artifact below. The user explicitly assigned later aggregate packaging to the labels work and requested no coordination messages; this follow-up does not build/package/install or operate live services. Verification is limited to source parsing/format and whitespace checks, without a new binary/PTY acceptance claim.

## Dashboard transcript reader

Scenario: highlight an Agent row and press `t` to switch the dashboard content area from events to its latest ten model exchanges. Press `t` again (or `Esc`) to return to the same event view. Show the reader above the Agent strip in stacked layout and in the event pane in side-by-side layout. The Agent list remains visible, and the reader owns its scroll position and explicit refresh.

The dashboard fixes the target when it opens the reader. Use the exact live participant with the native transcript client, the displayed historical row's saved Provider source for stopped Agents, and the existing device-qualified transcript command for remote Agents. The existing RPC worker owns loading; only the active reader can accept its response. Missing transcripts and read errors stay visible in the reader. Verify both dashboard modes at the shared build; no local compilation or live changes were requested.

Source: [reader and regression cases](../src/tui/transcript.rs), [view](../src/tui/render/transcript.rs) and [terminal lifecycle](../src/tui/mod.rs). The stacked event feed is native terminal scrollback; draw the stacked Transcript in a temporary buffer so returning restores those events and appends new ones through the existing ejector. Test the two layouts, late-response rejection, and preservation of the event filter, selection, scroll, and draft. At the shared build, also exercise both the stacked return and resize in a real PTY.

## Prior accepted artifact and installation

The corrected selectors are implemented, verified and installed as the exact accepted artifact at project `.bin/agentx`. [Adoption receipt](/tmp/agentx-target-selection-rc_xota2/adoption.json) records the rollback and passive installation checks. This closes the authorized source/package/project-install boundary. The existing live daemon remains on protocol epoch 2 while the installed candidate uses epoch 3; native operations against that daemon require a separately authorized transition owned by maho/the operator. No live restart was performed. See the explicit runtime limit below before using lifecycle commands in that scope.

The user authorized launch catalog selection and the lifecycle-target extension below. `launch` and `l` with a bare `--role`, `--profile` or `--tool` request a searchable single selection; supplied names and omitted options retain ordinary resolution. An empty value also requests selection. Escape/Ctrl+C cancels with exit 130 before launch acquisition. Non-TTY, JSON, stdio and remote selection require explicit values. Other omitted inputs retain their current behavior.

Keep native catalogs authoritative: [roles](../src/config/roles.rs), [profiles](../src/config/profiles.rs), [integrations](../src/integrations/registry.rs). Resolve choices in role → profile → tool order, respecting workspace/inheritance/helper discovery, explicit tool/profile compatibility and role-associated profiles. Never ask for an omitted selector or substitute a choice for an invalid explicit name.

Structure: `cli/commands/launch/selection.rs` owns early argument normalization and launch-specific candidate composition; `tui/select.rs` owns a small searchable selector, input state and terminal restoration. Existing launch clap declarations expose optional values only on fresh launch. Normalization runs before environment/help scanners so adjacent flags and opaque native arguments remain intact. Resolve after environment composition and before launch Session acquisition. Reuse installed ratatui/crossterm; no dependencies, global picker policy, subagents or live-service changes.

## Accepted lifecycle-target extension

The user selected `attach`, `resume`, `fork` and `kill` as additional selector consumers. Open a picker only when their target is omitted and both stdin/stderr are terminals. Explicit names, IDs, PIDs, `kill all`, JSON output and noninteractive invocations keep their existing direct/error behavior. Escape/Ctrl+C cancels with exit 130 and performs no operation.

Candidate ownership and stable values:

| Command | Candidate rule | Value passed to the existing operation |
| --- | --- | --- |
| `attach` | Current local terminal endpoints whose process is alive and whose recorded tmux pane/server can be validated | The exact `ParticipantRef`; revalidate it in the existing attach endpoint path |
| `resume` | Latest stopped tracked sessions with a supported saved tool and session ID | The native stop-event ID and session ID; compare the existing prepared plan's fence before execute and reject a changed same-name snapshot |
| `fork` | Eligible current local records and latest stopped tracked sessions; hide tools whose registry spec has no fork operation | Current: name plus exact `ParticipantRef` and session; stopped: native stop-event ID and session |
| `kill` | Local Agent records with tracked processes and status/tool/directory context | Multi-selected names plus frozen exact rows; `all` remains its separate existing operation |

The common TUI selector owns multi-selection and a final review state that lists all selected kill targets before confirmation. Selection is keyed by choice identity, survives filtering, and cancellation from either state has no effect. Orphan PIDs remain available through explicit `kill <pid>`; targetless selection does not silently broaden into `kill all` or an orphan sweep. Existing native kill fencing and resume/fork reservation remain authoritative after selection.

Implementation boundary: a command-target selection policy module composes candidates from the existing `Terminals`, `Terminal`, `Events::Query`, `Participants::List/Lookup` and terminal validation contracts; the CLI continues invoking existing attach/kill/resume/fork effects. Stop history uses one bounded native page (up to 1,000 events); explicit targets remain available outside that inventory. No service/protocol changes, candidate snapshot persistence or new dependency is required. A tiny `commands/mod.rs` registration exposes the private shared policy module.

The prepared extension was applied after lore released source/native execution in AgentX 42003. Its original isolated copy and diff remain at `/tmp/agentx-target-selection-rc_xota2`; current repository source is authoritative. It adds searchable multi-selection with hidden-selected count and complete final review; native stop-event/session checks before execute; and a finite `tests/cli_target_selection.py` fixture using the existing disposable management Provider/process owner. Scoped `just test --bin agentx selection -- --nocapture` passed 20/20; its complete log is `/tmp/agentx-target-selection-rc_xota2/native-selection.log`.

Initial candidate `.bin/agentx-metadata` version 0.7.31, SHA256 `f3237d940e692408f37b848fb9b16792ea4ad02d901802ef4420dcdc6d6082a3`, is rejected for project adoption. lore passed its CLI shape and isolated native/browser acceptance but the original launch PTY exposed a real stdout leak: Ratatui core 0.1.2 `Terminal::clear` snapshots the cursor through Crossterm, which writes its DSR to process stdout even with a stderr backend. Fullscreen `Terminal::new` and resize do not need that snapshot. The owner repair clears the alternate screen and positions its origin directly through stderr, then constructs the normal fullscreen terminal; `Terminal::clear` is removed. Original stdout-isolation/cancellation/restoration assertions remain intact. The earlier DSR-responder fixture update alone could not fix a query sent to the separate stdout pipe. No initial candidate overwrite, duplicate full suite or live restart is authorized or performed.

Corrected development executable build passed. Original launch PTY passed 10/10 (`development-launch-pty-r1`), including stdout isolation, sequential selection, cancellation/restoration and zero daemon acquisition. Target PTY passed 9/9 (`development-target-pty-r4`), including active fork/stopped resume previews, kill-review cancellation, actual selected kill, two-of-three multi-select retaining the unselected live Agent, and local tmux endpoint/cancellation. All used disposable fixture scopes and exact owned cleanup. Earlier target attempts retained fixture defects: r1 expected a complete review title in incremental ANSI output; r2 expected stopped records to disappear instead of retaining `status=stopped`; r3 exposed tmux missing only from the fixture's deliberately private PATH. The runner now waits for the review's unique new hint, checks native stopped/live identities and exposes only the installed tmux through its owned bin directory.

The corrected release command `just package .bin/agentx-selection` passed with receipt/log `selection-package-r2` under the same evidence root. `selection-source-inputs-r2.json` binds the original input inventory plus new target-selection source/runner; all 1,241 inputs were checked after packaging with zero drift. The owner stopped only package-r1 descendants before artifact creation to fix a concrete operand boundary: an Agent selected as `all` must remain one exact row, so bulk expansion now requires an explicit invocation. This does not change explicit `kill all`. Initial metadata acceptance remains historical evidence bound to its different candidate; the corrected artifact's checks and actual installation are recorded below.

# Verification and current state

| Check | Result and evidence |
| --- | --- |
| Scoped native selector/state/precedence checks | 20 passed; `/tmp/agentx-target-selection-rc_xota2/native-selection.log` |
| Corrected development executable | Build passed; original launch PTY 10 passed; target PTY 9 passed; receipts under the same evidence root |
| Corrected release build | `just package .bin/agentx-selection` passed in 956.33s; `selection-package-r2-receipt.json` |
| Exact artifact binding | Version 0.7.31, mode 0755, 32,747,704 bytes, SHA256 `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`; `selection-artifact.json` |
| Build input binding | `selection-source-inputs-r2.json`, SHA256 `40dc581638191e44945f8397b0cac7e699706586727ae5234fadb82920fdf09d`, 1,241 inputs, zero drift after package |
| Exact release target PTY | 9 passed; `release-target-pty-r1-receipt.json`; isolated fixture `/tmp/agentx-target-acceptance-d1txtu8o/run` reports no cleanup errors and unchanged candidate |
| Independent exact release original launch PTY and CLI | 10/10 original unadapted PTY and 11 CLI checks passed; lore's final release is AgentX 42673 |
| Independent exact release native management/browser | M01–M11 plus transcript and B01–B03 passed; fixture cleanup has no errors and candidate bytes remain unchanged; [independent receipt](agent-metadata/verification.md#corrected-fixed-artifact-and-final-independent-result) |
| Project `.bin/agentx` adoption | Atomic replacement completed after lore 42673/maho 42682 release; installed hash matches the accepted candidate; version and launch bare-role/attach optional-target/resume help checks passed; `adoption.json` |
| Live runtime compatibility | Post-install handoff attempt observed existing daemon PID 3279247 on epoch 2 rejecting the epoch-3 client. Service left running; successful handoff used only the preserved rollback executable with invocation-local environment overrides |

The user confirmed project `.bin/agentx` is the actual executable and authorized its update. Installed version is `agentx 0.7.31`, SHA256 `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`. Rollback is `.bin/agentx.before-selection-20261116-200321`, SHA256 `9cdc66606ef4e2880868adcfc5e81638a3ab0ca54ef010e6f7ee804e5c8b2cf0`. The [adoption receipt](/tmp/agentx-target-selection-rc_xota2/adoption.json) identifies the exact paths and installed checks.

## Live runtime limit and handoff

The metadata cleanup changes the native protocol to epoch 3; selector policy itself adds no protocol. The installed CLI cannot connect to the still-running epoch-2 daemon in `/home/long/works/longwork/agentx/.agentx`. Isolated acceptance used matching candidate-owned disposable daemons and does not establish live-scope adoption. No live/default daemon, PTY host, Relay worker or preview was restarted. The current daemon and data must remain untouched until an operator-authorized transition; maho owns this remaining rollout disposition with the metadata cleanup. This is an explicit installation limit, not a pending selector source/build check.

The first post-install AgentX handoff failed during protocol admission before delivery. The successful directed handoff to maho/lore used the retained epoch-2 rollback executable, with a temporary `rollback-client/target/debug/agentx` symlink and invocation-local `AGENTX_DEV_ROOT`/`CARGO_TARGET_DIR` overrides to prevent re-execution into the installed incompatible client. No shared configuration was changed. `/tmp/agentx-target-selection-rc_xota2/adoption-handoff.json` retains the exact command, environment overrides and successful result.

Source and Cargo lanes were released to nezi's separate labels work after the exact corrected artifact/input freeze; subsequent source changes belong to that work and do not alter this immutable selector candidate. Preserve `.bin/agentx-metadata`, `.bin/agentx-selection`, the rollback and their independently bound evidence.
