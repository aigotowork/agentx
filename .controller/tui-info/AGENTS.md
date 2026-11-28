# Native Agent Info

## User workflow

Use `i` or the dashboard's Info action on a single Agent. A single check takes precedence over the list cursor. In a transcript the unchecked target is the exact participant originally opened. The reader owns its own navigation history; Enter or a click follows a visible link, Esc goes back one visit and i returns to the original view. Input fields retain ordinary text entry. The inline controls stay below the active reader.

## Ownership

| Boundary | Source |
| --- | --- |
| Visit identity, history, request fencing and scrolling | [Navigation](../../src/tui/info.rs) |
| Exact snapshots, role/profile provenance, task bindings and launch relationships | [Read model](../../src/tui/info_data.rs), [native read contract](../../src/service/queries_info.rs), [assignment](reads.task.md) |
| Ratatui layout, wrapped rows and hit regions | [Renderer](../../src/tui/render/mod.rs), [assignment](render.task.md) |
| Synthetic visual workflows | [Replay guide](../../tests/tui_replay/README.md), [assignment](replay.task.md) |

Role and Profile are the adopted launch facts. Task and Delivery show observed file state, including read diagnostics. Relationships carry exact identities. Long text is requested on entry. Keep the inspection read-only and avoid interpreting provider arguments as a verified tool or skill inventory.

## Native terminal check

Run `python3 tests/tui_terminal/info.py target/debug/agentx --output target/tui-terminal/info` after building the candidate. This drives a real tmux pane, isolated daemon and fake Codex provider through Role instructions, Profile, frozen Bootstrap and Task body. A second Agent is launched by the first for direct Child and From navigation. See the [root pane](../../target/tui-terminal/info/parent.txt) and [child pane](../../target/tui-terminal/info/child.txt).

## Upper reader and exact references

Info and transcript share the region above the Agent strip in the inline layout. The terminal host enters the same temporary buffer for either reader and restores event scrollback on return. In the side-by-side layout both use the message pane. Exact Info requires a daemon-supplied reference; [local read convergence](../cloud-namespace/local-contract-retirement.md) supersedes the [historical mixed-daemon evidence](compat.task.md).

For the Tab prompt, Role/Profile selection and advanced launch options, see [dashboard quick launch](quick-launch.md).
