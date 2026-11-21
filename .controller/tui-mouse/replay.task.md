---
schema: task
status: completed
owner: fire
description: Implement native TUI replay support.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"muna","incarnation_bits":4745316779277045036}}]
---

Implement offline production render and input replay, text and styled HTML step-by-step viewer incl cursor/target overlay and PNG via Playwright, just tui-replay <scenario>. Own replay.rs (test-only), justfile recipe and tests/tui_replay/ artifacts/scripts. Main declares mod replay in tui/mod.rs. Use app.handle_mouse(MouseEvent), app.mouse.regions() -> Vec<(Rect,Target)>. Hook offline recording of side effects as needed (coordinate with main). Provide mixed input, dynamic geometry and modal scenarios. Test real assertions. No real runtime effects. Write usage.

## Implementation boundary

[Native replay](../../src/tui/replay.rs) drives the production renderer and input handlers with fixed fixture data and keeps attempted side effects in a test-only App sink. The [viewer and usage](../../tests/tui_replay/README.md) document the replay artifacts, their browser requirements, and the difference from PTY evidence.

The required geometry has two points in time: dispatch resolves against the last painted map, while the result frame may move or remove the target. The trace therefore records the hit against the before frame and the text caret against the native frame in which it was measured. The viewer displays before/after on the same step and exports both for pointer events. Inline frames preserve their absolute terminal origin; the viewer offsets visual markers relative to that origin.

Resize and fixture snapshot refresh are separate replay inputs: resize cancels a pending press, while refresh replaces the in-memory source and calls the production reload path so focus follows identity and a replacement row cannot receive the release. A thread-local clock makes synthetic agent timestamps stable.

## Local verification

Plain `just tui-replay all` exited successfully with the documented local Playwright package. All four native scenarios passed and the offline browser checks generated 73 PNGs: 23 mixed, 17 geometry, 21 modal, and 12 inline. The geometry text trace includes the reordered snapshot and a release over `@ravo` while focus remains on `@nova`. The browser exercised before/after navigation, verified pointer overlays against frame origins, and checked navigation and terminal overflow at a narrow viewport. Artifacts are in `target/tui-replay/`; [usage](../../tests/tui_replay/README.md) explains how to reproduce them. Terminal capture and daemon behavior are outside this evidence.
