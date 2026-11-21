# Dashboard quick launch

Tab opens the prompt. Up focuses Role, Left/Right selects Role, Profile or the advanced settings, and Enter opens the field. Click a field or choice for the same action. Escape cancels a picker. Enter in the prompt submits even when the task is empty.

[Selection and preview](../../src/tui/launch.rs) use the native caller workspace, visible role catalog and launch resolver. Only explicitly chosen selectors enter the [launch request](../../src/tui/rpc/mod.rs). Keep tool, count, headless and terminal in the advanced panel. Retain the draft when correcting a failed submission. The [quick launch replay](../../src/tui/replay.rs) exercises both layouts with synthetic configuration.

Run `tests/tui_replay/run.sh quick_launch` for both the [stacked replay](../../target/tui-replay/quick_launch/index.html) and the [side-by-side replay](../../target/tui-replay/quick_launch_vertical/index.html). The narrow-screen steps cover cancelling a filter, overriding an inherited Profile and correcting a missing Role-bound Profile without losing the task.
