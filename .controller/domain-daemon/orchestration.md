---
schema: document
status: active
owner: controller
description: AgentX-native team lifecycle and communication.
---

# Team orchestration

Use AgentX itself for engineering-agent launch, assignment, communication and lifecycle. Do not require Paseo, its profiles, workspace IDs or agent IDs for development collaboration. The user's current instruction supersedes earlier platform choices.

Keep assignments, source ownership, constraints and handoff evidence in the owning Delivery/Task. For routine delegation, run `agentx launch --go --headless --title "Task title" --bind ".controller/xx/xx.task.md" --agentx-prompt "Assignment; read the bound Task"` directly with configured defaults. Add `--role NAME` when a suitable role is known. Do not inspect profiles or consult help before ordinary launches; follow the launch diagnostic if configuration is missing. Use command help only for additional options or recovery. Explicit user model/reasoning requirements still apply. Create only justified, bounded subagents; do not assign work to unrelated existing agents.

Use AgentX participant identities and messaging for actionable handoffs, blockers and completion. Keep normal progress in the Task. Preserve single-writer file ownership and release it explicitly when handing work over. Provider test fixtures use disposable scopes and must not replace installed binaries or mutate live user services.

Earlier Deliveries may retain Paseo IDs and execution receipts as historical evidence. Those records do not authorize new Paseo scheduling or reactivate archived workers.
