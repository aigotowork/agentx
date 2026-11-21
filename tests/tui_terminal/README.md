# Native TUI terminal smoke

Run from the repository root with Python 3.11+ on Linux and an exact built AgentX candidate. The [PTY Task](../../.controller/tui-mouse/pty.task.md) owns the assignment; [offline replay](../tui_replay/) covers interactive scenarios against the production renderer separately.

```sh
python3 tests/tui_terminal/smoke.py target/debug/agentx \
  --require-tmux --output "$(mktemp -d /tmp/axtui-evidence-XXXXXX)/capture"
```

The script starts and verifies its own daemon in a disposable `AGENTX_DIR` and uses a fresh home, working directory and tmux socket. It launches no Agent or provider and does not connect to a running user tmux server. Omit `--require-tmux` to run just the direct PTY checks when tmux is unavailable. `--keyboard-only` is a baseline mode for older executables without inline mouse support; it does not verify click or wheel interactions.

| Check | Evidence |
| --- | --- |
| Direct operating system PTY: inline and fullscreen startup, SGR capture and bracketed paste negotiation, keyboard input and paste, resize and redraw in each mode without an inline saved-lines erase, layout transitions, Ctrl+C/Ctrl+D and exact TTY attribute restoration | `direct-*.ansi` are raw process output. The driver supplies a synthetic cursor position report for Ratatui's inline queries; the stream is not a rendered screen. |
| Attached tmux: click visible Help in inline and fullscreen, scroll the overlay with the wheel, click the layout toggle, resize the attached client in both layouts, preserve a pre-dashboard shell line in scrollback, and verify dashboard exit and client detach | The named pane snapshots have text and styled ANSI forms. `tmux-client.ansi` is the raw stream from the attached client; `tmux-pane-output.ansi` is the dashboard's raw output from tmux's pipe. The driver sends xterm SGR bytes through the client's PTY. |

None of these artifacts is a desktop screenshot. Offline replay PNGs show synthetic renderer frames; tmux pane captures show the running binary's terminal cells without GUI font rendering, pixels or physical mouse input. `result.json` records the executable's SHA-256 and the checks that completed. A failing run keeps the available captures and ends nonzero.

For a window-pixel capture and a native selection check on Linux with GTK3, VTE 2.91, XTest and Xvfb installed, run the following command. It clicks Help, verifies the text selected by a Shift-drag while the TUI owns the terminal, exits to the fixture shell, and verifies plain drag selection. It then filters and clicks a tool in the CLI picker and checks that redirected stdout contains a clean dry-run preview for that tool. The images and selected strings are written under the output directory.

```sh
GDK_BACKEND=x11 xvfb-run -a /usr/bin/python3 tests/tui_terminal/vte_capture.py target/debug/agentx --output target/tui-terminal/vte
```

For Agent Info, the Linux integration check runs a disposable daemon, launches an Agent with the existing fake Codex fixture and reads its adopted role, profile and bound Task through a real tmux dashboard. The captures are plain text.

```sh
python3 tests/tui_terminal/info.py target/debug/agentx --output target/tui-terminal/info
```
