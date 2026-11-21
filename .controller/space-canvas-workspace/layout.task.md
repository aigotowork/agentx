---
schema: task
status: completed
title: Canvas pane layout and visual style exploration
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"ruvuma","incarnation_bits":4745316436499810499}}]
---

Read [controller design](controller.md), [Space brief](../../.ui-craft/surfaces/space.md), [UI Craft](../../.codex/skills/ui-craft/SKILL.md), Shape, layout, responsive, dashboard, minimal/editorial. Own `src/workspace/WorkspaceWindows.tsx` and `workspace.css` (self import), and your task/design artifacts. Explore two style directions on the same scene/records, compare compact field desk versus quieter reading room; choose a common treatment that respects existing Space skins. Implement semantic window layout for overlay over the scene and normal region alongside roster in chat presentation. API: `WorkspaceWindows({ panes, activeId, focused, overlay, onActivate, onFocus, onMinimize, onClose }: Props)`; `panes: readonly { id:string; title:string; minimized:boolean; visible:boolean; reader?:ReactNode }[]`; callbacks take id, `onFocus` toggles focus. Pane shell owns header controls; reader still owns exact identity/channel/control layout. Shelf shows all open IDs, makes non-visible panes reachable and preserves focus; desktop two, narrow one (CSS container or media, show only active narrow without duplicate mounts for visible pane). Expose stable accessible labels and data hooks, coordinate CSS feedback with companion Task via design record. Leave navigation/SDK/App to controller. Record concrete handoff and trade-offs.

The [pane shape, style directions and feedback contract](layout.design.md) govern this slice.

Implemented the [shared window shell](../../web/agent-space/src/workspace/WorkspaceWindows.tsx) and [its styling](../../web/agent-space/src/workspace/workspace.css). The production reader supplies a render function to put its exact window controls in the existing channel strip. The controller owns width-based reader leases, route and last-close focus return. The [browser evidence and matched visual comparisons](layout.design.md#browser-evidence) close this slice; they use offline Studio supply and leave live adoption with the controller.
