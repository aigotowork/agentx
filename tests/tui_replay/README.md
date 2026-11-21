# Native TUI replay

The replay is a development fixture. Rust owns the input sequence and its assertions, calls the production renderer and input handlers on an in-memory App, and records attempted RPC operations and Info reads without starting workers. The separate, offline browser viewer reads those frames; it does not implement the TUI, perform native hit testing, or call AgentX services. The [mouse replay Task](../../.controller/tui-mouse/replay.task.md) and [Agent Info replay Task](../../.controller/tui-info/replay.task.md) define this scope. PTY smoke tests address live terminal behavior separately.

| Scenario | Evidence |
| --- | --- |
| `mixed` | Focus and compose to `nova`; open and scroll its transcript; focus `luna` without retargeting the reader; click back into the draft and submit only to `nova`. The transcript reply is synthetic. |
| `geometry` | Resize cancels a pending click even though the pointer still hits the same agent. A new click works, and the wheel scrolls the resized message pane. A data refresh then reorders the rows while a button is down: focus follows the original agent and release leaves the replacement row alone. |
| `modal` | Help masks an underlying agent row; the row is clickable after dismissal. Cancelling the kill confirmation records zero RPCs and confirming records exactly one for `nova`. |
| `inline` | Focus an agent in a viewport with a nonzero origin, open stacked transcript detail from compose with Ctrl+T, and close it with a mouse click. Focus and draft survive the return to inline. |
| `info_links` | Open an adopted Role and Profile, follow a Delivery to its declared Task, and scroll the Task body. Back reuses completed pages. |
| `info_identity` | A sole checked Agent takes precedence over focus and retains its incarnation after replacement. Follow its historical parent to a historical sibling while a current namesake is replaced. Two checked Agents cannot open Info. |
| `info_context` | Open Info for a pinned transcript while focus is elsewhere. Closing Info restores the transcript position; closing the transcript restores the exact draft, caret, and message feed position. Compose accepts a literal `i`. |
| `info_resize` | Search and the launch prompt accept a literal `i`. Shrink the upper reader to 62 × 17, keep the agent strip visible below it, restore Info cursor and scroll on Back, and discard a late reply for an earlier visit to the same page. |

```sh
just tui-replay mixed
just tui-replay info
just tui-replay all
```

Each scenario writes to `target/tui-replay/<scenario>/`: `steps.txt` for plain text, `trace.json` for frame and event data, `index.html` for the standalone viewer, and `png/` for individual steps. Open the HTML file directly. Select a step with its list entry or the Previous/Next controls; use the Before view to see where a pointer event landed and which region was topmost at that moment. After shows the resulting frame and any text cursor. All displayed coordinates start at zero.

Playwright writes an `NN-after.png` for every step, plus `NN-before.png` for pointer events so their hit geometry is visible. These images show the browser's rendering of the captured Ratatui cells. The browser uses its local monospace font and a fixed fallback palette for terminal colors; it cannot reproduce a particular terminal emulator's font, theme or scrollback.

The recipe uses the repo's normal native build dependencies, plus Node.js and Playwright with Chromium for screenshots. It checks Node's module path and `target/tui-replay-tools/` for Playwright. If you don't already have a copy, install one into ignored local output:

```sh
npm install --prefix target/tui-replay-tools playwright@1.59.1
target/tui-replay-tools/node_modules/.bin/playwright install chromium
just tui-replay all
```

If Playwright's Chromium is missing, the exporter tries system Chrome. Use `AGENTX_BROWSER_EXECUTABLE` for a different compatible Chromium executable or `AGENTX_PLAYWRIGHT_PACKAGE` for a different Playwright package path. For text and HTML only, run `AGENTX_TUI_REPLAY_NO_PNG=1 just tui-replay mixed`. The tests also run without exporting files through `cargo test --locked --bin agentx tui::replay::`.

The Info traces record page requests with exact synthetic identity and send fixtures through the production reply handler at scripted points. They verify native navigation, drawing and stale reply rejection without reading any real Agent data or bound file. The [Reads task](../../.controller/tui-info/reads.task.md) tracks evidence for the production data adapter separately.
