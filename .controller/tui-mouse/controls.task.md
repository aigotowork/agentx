---
schema: task
status: completed
owner: fire
description: Implement native TUI controls support.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"dino","incarnation_bits":4745316779277946591}}]
---

Implement all visible controls, overlays, footer, launch/relay, confirm, reader close, empty state, palette, overflow for narrow screens and compose/text cursor placement using production geometry. Own render/mod.rs, render/launch.rs, new src/tui/control_mouse.rs. Expose ControlTarget(Clone Debug Eq PartialEq), pub fn activate(app:&mut App,target:ControlTarget), pub fn scroll(app:&mut App,target:ControlTarget,down:bool). Register Target::Control(ControlTarget) via app.mouse.add. Register Target::Reader and Target::Messages on scroll areas, Target::AgentsArea in agents renderer handled by another. Use app.mouse.clear_targets() for modal masks; main clears on each frame. Main handles scoped stale input, ask as needed. Include tests.

### Control interaction model

The renderer owns terminal-cell geometry and registers targets as it paints. The [control dispatcher](../../src/tui/control_mouse.rs) maps clicks to existing keyboard or state transitions and carries the semantic identity needed to reject a stale press. Dialogs mask lower targets; the reader is a pane and keeps its adjacent agent area scrollable. Compose opens the reader without discarding text, and clicking the draft closes the reader and positions the caret.

A narrow action row keeps complete controls and folds the remainder into a scrollable More menu. A palette row selects an exact suggestion; Fill and Run preserve the existing keyboard distinction. Single-line fields scroll horizontally to the caret, and compose uses the same wrapped graphemes for drawing, hit-testing and caret placement. The assignment did not specify these interference cases, so they are recorded here for replay and handoff.

### Implementation and evidence

[Dashboard rendering](../../src/tui/render/mod.rs) and [launch rendering](../../src/tui/render/launch.rs) register the painted controls and scoped wheel regions. The compose footer uses the same Ctrl+T shortcut as the keyboard handler; clicking the draft returns from the reader to its clicked grapheme. Confirmation dialogs include an explicit mouse cancel control, and wheel events over launch settings do not change configuration.

All six [native control tests](../../src/tui/control_mouse.rs) passed from the freshly built `agentx` test binary. They cover compact launch fields, palette and confirmation masking, wide and combining compose graphemes at a nonzero viewport origin, horizontal prompt placement, scrolling and dismissal in the narrow More menu, and masked relay token editing. Integrated replay and installed adoption belong to the parent delivery.
