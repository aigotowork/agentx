---
schema: task
status: completed
agent: dani
description: Implement configurable tmux headless hosting and reconnectable terminal attachment.
---

# Tmux headless hosting

The user selected the three-mode model and requested implementation, including socket/configuration ownership. [Design and usage](../../.docs/architecture/terminal-sessions.md) owns the selected contract. [Earlier discussion](../domain-daemon/feedback.md#headless-reattachment-clarification-2026-11-14) is historical input.

## Work and boundaries

One implementer owns configuration, a small runtime tmux adapter, launcher integration, CLI attachment and focused acceptance. Reuse existing PTY, process registration and terminal metadata; do not add a terminal streaming protocol or managed database tables. Preserve the current workspace's unrelated changes, aliases, daemon/workbench and all user tmux servers. Test only disposable scopes/sockets and mock provider executables. Build `.bin/agentx-tmux` separately from `.bin/agentx`; do not restart the running user daemon or workbench. The other CLI owner released source and package claims.

## Acceptance

Exercise separate window/headless defaults and per-invocation overrides; headless/window conflicts; config edit/read; native PTY fallback; two distinct tmux socket scopes; real detached launch, terminal attach/detach and continuing native PID; exact session cleanup; no user-default server mutation. Linux tmux execution establishes local support only; macOS is source-supported but requires its own execution evidence. Windows retains native PTY and explicitly rejects tmux.

## State

Implementation and local acceptance are complete. `just package .bin/agentx-tmux` built AgentX0.7.31, and `python3 tests/tmux_headless.py .bin/agentx-tmux` passed against that final artifact. SHA-256: `8bd555e2f81ac81def035e144d6482de879b26d6d3dd2a3760e87bef6d8f3940`. All nine affected tmux unit cases and the final debug real attach/input/detach fixture also passed after the scoped-diagnostic correction. The candidate is separate from `.bin/agentx`; existing user configuration, installed binaries, daemon and workbench were not replaced or restarted.

## Implementation map

| Responsibility | Owned source |
| --- | --- |
| Scoped defaults, ENV/TOML mapping and edit/read | [`config.rs`](../../src/config.rs), existing config editor, configuration disclosure |
| Mode selection and existing runner composition | [`launcher.rs`](../../src/runtime/launcher.rs) |
| Explicit socket/config, detached sessions and validated attach command | [`tmux.rs`](../../src/runtime/tmux.rs) |
| Exact participant lookup and caller terminal I/O | [`attach.rs`](../../src/cli/commands/attach.rs), [`terminal_client.rs`](../../src/runtime/terminal_client.rs) |
| Early observed server/pane capture and recapture preservation | Existing PTY shared context, terminal projection and Agent binding |
| Three-mode usage, root/recursive help, config descriptions | [Usage](../../.docs/architecture/terminal-sessions.md), README and CLI help |

## Focused evidence

| Case | Executed result |
| --- | --- |
| Catalog, validation, ordinary configuration compatibility | `just test --bin agentx config::tests -- --test-threads=1`: 105 passed and one failed because the pre-existing generic sample used an invalid value for the new enum. The sample now uses `tmux`; its exact single-test rerun passed. |
| Tmux path/attachment checks and old/new metadata projection | `just test --bin agentx tmux -- --test-threads=1`: 9 passed. |
| Config keys discoverable in ordinary help | Exact `cli::commands::help::tests::config_help_covers_registry_keys`: passed. |
| Root, attach and recursive public help | Disposable-home probes passed; help did not create its selected data scope. New socket/config key info matched documented defaults and did not create a managed database. |
| Real headless lifecycle | Final debug and packaged artifacts passed the isolated Linux/tmux3.6a fixture: two scopes, default/custom configuration, a socket path containing spaces, ENV precedence, conflicts, repeated controlling-terminal attach/input/detach, continued same native PID, changed defaults retaining actual socket, same-server neighbor preservation, independent second server, private socket-parent permissions and native PTY override. |
| Local source hygiene | Touched source `git diff --check` and selected `rustfmt --edition 2024 --config skip_children=true --check` passed. |

The fixture uses a local Qoder-shaped executable without Provider hooks or paid model calls. This proves PTY hosting/reattachment without depending on a Provider SessionStart hook; it does not prove every real Provider, macOS/iTerm, Windows execution, relay attachment or same-server client switching. Existing PTY delivery and Provider integrations are reused, not independently reaccepted here.

## Findings and disposition

Real attachment first exposed missing early server metadata: an existing empty `env` object blocks fill-only nested context merging. Runtime now stores observed TMUX in a top-level `tmux` field beside `pane_id`, preserves it during hook recapture and retains the old `env.TMUX` read fallback. The default `/dev/null` is a device, so only an explicitly selected tmux config is required to be a regular existing file.

Launching from another tmux terminal must not restore the parent's server value over the new pane's TMUX through the captured environment sidecar. The tmux background branch removes that inherited value; the fixture supplies unrelated parent socket/pane metadata and checks the actual new socket.

Startup-failure evidence previously read a pane on the invoking default server and suggested a whole-server reset. Scoped headless hosting makes that lookup incorrect. Diagnostics now shares the runtime adapter's recorded socket/server/pane validation and bounded control transport with attachment, and permission remediation names only the recorded socket. Missing metadata produces no captured evidence instead of reading an unrelated default pane.

Operator kill is asynchronous with PTY shutdown; the fixture waits for owned-session disappearance while checking a live neighboring session. Provider startup also precedes ProcessHost registration: native-PTY fallback acceptance now waits for its public terminal endpoint before operator stop, avoiding a startup PID-fence race. No kill-fence relaxation or effect replay was added.

Existing `config KEY --info` can initialize update-cache files although it bypasses daemon acquisition. The probe therefore distinguishes help's no-scope behavior from config-info's no-managed-database behavior. This is existing command metadata routing in `cli/router.rs`, outside the selected tmux change.

## Default settings and stale attach correction (2026-11-14)

The user reported `attach neko` failing with StaleFence while its managed tmux pane remained alive. The CLI reconstructed `ParticipantRef.incarnation_bits` from the `InstanceRow.created_at` float returned by List; serde_json can round that float by one ULP. Attach now obtains the exact integer fence from native Participants::Lookup and retains the subsequent endpoint validation. A deterministic native IPC test selects a rounding-sensitive timestamp, demonstrates the old mismatch and validates the returned exact fence.

The user selected larger history and mouse support as defaults. With no custom tmux_config, launch now writes private `.tmp/tmux/default.conf` containing history-limit 100000 and mouse on; Settings::resolve remains free of filesystem writes. Custom configuration remains explicit. The already-running project tmux server was updated only for these two requested options, without restarting or replacing its Agent. Verification: the deterministic native IPC precision regression passed; both runtime tmux unit tests passed; the final debug fixture passed with default history-limit 100000 and mouse on, custom-config precedence, exact socket attachment, input/detach and isolated cleanup. Scoped formatting and diff checks passed. Final artifact: `just package .bin/agentx-tmux` completed, and `python3 tests/tmux_headless.py .bin/agentx-tmux` passed. SHA-256: `b4e68d998c7ccc18372e76b6fd21ee467e12969e603974a8a25b13234e9daf47`. The primary `.bin/agentx` and live daemon were preserved; the running project tmux server received only the two user-requested setting changes.

### Follow-up launch grammar discussion

The user questioned the different behaviors of `--terminal tmux`, `--terminal tmux --no-run-here` and `--terminal tmux --headless`. Before the follow-up below, the implicit single-Agent run-here default could win over an explicit terminal, and the legacy non-headless tmux preset did not share scoped settings. The accepted follow-up below owns the replacement behavior and its current execution state.

### Accepted follow-up: unified tmux launch (2026-11-14)

Status: completed; see the accepted local outcome below. The user authorized the simplified grammar. Existing current-terminal execution remains the default for a single interactive human invocation. An explicit terminal overrides that implicit default. Both foreground and detached tmux use the scoped runtime adapter and existing PTY host. `--run-here` conflicts with an explicit non-here terminal; it is not a second way to request tmux.

| Scenario | Expected outcome / check |
| --- | --- |
| Single human TTY, explicit tmux | Create managed session, report its name, wait boundedly for its exact terminal endpoint, attach; detach leaves the same PID alive |
| tmux + headless or no-run-here | Same socket/config/runner, return without attaching; manual attach remains available |
| tmux + JSON, non-TTY, multiple Agents or inside an AI tool | Detached managed sessions; never consume caller standard streams |
| No terminal override | Existing single-human current-terminal default; configured terminal still selects the host when opening elsewhere |
| Explicit window terminal | Open that terminal without requiring no-run-here |
| Invalid run-here / terminal combination | Reject before reserving or spawning |
| Remote launch | No originating-host automatic attachment |

Runtime owns the host selection and optional exact ParticipantRef for local attachment; the field is not serialized to relay clients. CLI reports launch first and then delegates attachment to terminal_client. That client waits only for startup metadata, keeps the exact identity fence, and gives a manual attach recovery command on failure. No new daemon contract, terminal transport, or session store is needed. The existing integration fixture will cover foreground detach/input/reattach and detached variants; unit cases cover precedence and conflict behavior. Preserve the live daemon and primary binary; package only the separate candidate.

Follow-up fixture findings: a reused tmux server retains its initial environment, so per-run probe filenames must be native fixture arguments rather than ambient variables. A later cleanup exposed the same timestamp-rounding defect in the independent operator kill path (`kill_native_row`); terminal screen selection also reconstructs that float fence. These consumers are outside this launch/attach change. Record the finding here rather than weaken native fences or widen this change into all operator commands. The updated host fixture cleans up only exact panes in its private sockets (and its own native provider PID), so its new acceptance must not be described as operator-kill or screen-query acceptance. Earlier operator-kill evidence remains historical.


Follow-up debug evidence: launch precedence unit case passed (one test); the current debug executable passed the extended real-tmux fixture, including default current-terminal execution despite a configured window terminal, foreground attach/input/detach/reattach with continued provider PID, no-run-here and JSON on actual controlling TTYs, piped invocation, headless launch, scoped settings, retained socket, neighboring session survival and native-PTY fallback. Selected Rust formatting and tracked-file diff checks passed. Release candidate packaging and final-artifact execution are in progress.


### Accepted local outcome: unified tmux launch

The follow-up implementation and Linux acceptance are complete. `just package .bin/agentx-tmux` produced AgentX 0.7.31; SHA-256 `8619eafd37979d1d183252bb5b30ba8a19c2e42eb292ae884fd7d321d6f316aa`. `python3 tests/tmux_headless.py .bin/agentx-tmux` passed, including successful fixture cleanup. The primary `.bin/agentx`, installed programs and live user daemon/workbench were not replaced or restarted by this work.

The final artifact passed the same foreground/current-terminal/detached scenarios as the debug build. Existing Linux tmux socket/config defaults and explicit overrides still passed. The precedence unit test passed; scoped formatting and diff checks passed. Tests use real tmux with a local Qoder-shaped Provider fixture, not paid Provider execution. Remote attachment, macOS/iTerm execution and nested tmux client switching were not independently exercised.

Final-artifact acceptance initially completed all functional checks but failed temporary-directory removal: PTY shutdown could reacquire the daemon after cleanup had stopped it. The fixture now records its Provider parent PIDs, waits for the owned hosts to exit, then stops and waits for each owned daemon. The rerun passed with cleanup included. This changes test lifecycle ordering only; no user daemon lifecycle semantics were modified. The separate operator kill/screen float-fence finding above remains outside this completed launch/attach scope.
