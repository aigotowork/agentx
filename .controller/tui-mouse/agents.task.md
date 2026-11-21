---
schema: task
status: completed
owner: fire
description: Implement native TUI agents support.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"ravo","incarnation_bits":4745316779277145289}}]
---

Implement agent rows, headers, tabs, multiselect marker, list/strip scrolling preserving focus, stable identity. Own render/agents.rs and new src/tui/agent_mouse.rs, declare no mod in tui/mod.rs. Expose AgentTarget (Clone Debug PartialEq Eq), pub fn activate(app:&mut App,target:AgentTarget), pub fn scroll(app:&mut App,down:bool). Register Target::Agent(AgentTarget) and Target::AgentsArea through app.mouse.add. MouseState will have pub agent_scroll: Cell<Option<usize>>. Coordinate extra data requirements. Include tests.

The renderer and mouse dispatch share the same logical item ordering. Action targets use name/device/creation time or PID/launch time and resolve again at release. Existing keyboard-driven list layout keeps its live/section split; a manual scroll traverses one continuous list while preserving the cursor and message filter. Mouse wheel starts use the most recent render geometry; keyboard cursor movement and row clicks return the pane to cursor-following.

Source is ready in [the renderer](../../src/tui/render/agents.rs) and [dispatch](../../src/tui/agent_mouse.rs). Tests cover reordering and rejecting replacements, exact remote multiselect and inline filtering, scroll with preserved focus in both layouts, and Unicode tab hit geometry. Formatting and whitespace checks passed. The first native test compile ran while the shared controls and replay sources were being edited and could not build; fire owns the integrated native test run.

Integrated Linux native TUI tests passed, including this module. See the [delivery entry](AGENTS.md) for the command and current evidence.
