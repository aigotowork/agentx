---
schema: task
status: completed
owner: fire
description: Implement native TUI selector support.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"lili","incarnation_bits":4745316779277989529}}]
---

Implement complete mouse selection in src/tui/select.rs using its actual stderr writer, stable visible list offset, single and multi review, clickable footer, press release/resize/outside behavior and focused tests. Standalone from main App.

The [selector](../../src/tui/select.rs) owns terminal capture, the viewport, and its hit targets. A click on a single-choice row picks it; a click on a multi-choice row toggles it. Review rows only move the cursor, and the final action requires a separate click or Enter. Edit and Cancel mirror the keyboard actions. Only a left press and release on the same rendered target activate it; leaving the target during a drag, resizing, or changing the view clears the pending press. Wheel input over the list scrolls its viewport. Blank rows and outside clicks have no action. Tests use rendered cells to exercise choice identity and the review gate without starting a daemon.

Integrated Linux native TUI tests passed, including this module. See the [delivery entry](AGENTS.md) for the command and current evidence.
