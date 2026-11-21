---
schema: task
status: completed
owner: fire
description: Implement native TUI pty support.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"hera","incarnation_bits":4745316779277462458}}]
---

Implement and run isolated PTY terminal input/capture/resize/inline/fullscreen/exit smoke verification including tmux as available. Own tests/tui_terminal/ and focused docs there. Use built binary or dedicated test fixture and isolated runtime scope. Distinguish emulated vs actual screenshots. Coordinate with main.

The [terminal verification guide](../../tests/tui_terminal/README.md) contains the reproducer. Direct inline and fullscreen PTY checks and the attached tmux mouse workflow passed. See the [delivery entry](AGENTS.md) for the candidate evidence.
