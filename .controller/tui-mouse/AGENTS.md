# TUI mouse and replay

The production Rust dashboard owns both keyboard and pointer input. Mouse interaction is enabled on entry, including the inline viewport. Terminal capture belongs to the screen lifecycle. Renderers register the actual visible terminal cells for each target every frame. Modal renderers mask underlying regions. Press and release must resolve to the same logical target, and wheel events use the pane under the pointer. Terminal-native selection remains host-specific.

| Ownership | Work |
| --- | --- |
| Event routing and capture | [Mouse state and dispatch](../../src/tui/mouse.rs), [dashboard event loop](../../src/tui/app.rs), [terminal lifecycle](../../src/tui/mod.rs) |
| Agent regions and identity | [Agent assignment](agents.task.md) |
| Controls and overlays | [Controls assignment](controls.task.md) |
| CLI picker and review | [Selector assignment](selector.task.md) |
| Reproducible visual feedback | [Replay assignment](replay.task.md) |
| Escape-sequence and cleanup verification | [PTY assignment](pty.task.md) |

Use the existing action handlers for keyboard and mouse. Validate side effects through offline recording and use an isolated runtime for actual terminal input. Local source and executable verification do not imply installed adoption.

## Development and verification

Run `cargo test --locked --bin agentx tui::` for the native TUI checks. Use `just tui-replay mixed` for the combined compose/read/send path, or `just tui-replay all` to render the full suite as text, styled HTML and PNG. The [replay guide](../../tests/tui_replay/README.md) explains the fixtures and screenshot dependencies. The [terminal verification guide](../../tests/tui_terminal/README.md) covers actual binary input in an isolated PTY, an attached tmux client, and host-native selection in a GTK/VTE window.

## Local Linux evidence

The native TUI run passed 176 tests. The integrated development executable built with `cargo build --locked --bin agentx`. Its final [PTY receipt](../../target/tui-terminal/pty/result.json) records SHA-256 and the direct inline, direct full-screen and attached tmux checks. GTK/VTE window captures verified native Shift-selection during capture, plain selection after exit, a clickable Help action and filtered CLI selection with clean redirected dry-run output. See the [selected text](../../target/tui-terminal/vte/selection.txt) and [picker screenshot](../../target/tui-terminal/vte/filtered-picker.png). Generated artifacts live under ignored target directories and can be regenerated from the linked guides.

A concurrent messaging change blocked compilation by reading an inbox consumer after moving it. The integration fix evaluates the existing legacy reservation predicate before passing ownership to the inbox. It preserves the original consumer scope.
