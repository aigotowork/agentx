---
schema: document
status: active
description: File-backed Task and Delivery assignment consumer implementation.
links: [./delivery.md, ./task.md]
---

# File task bindings

Read [Delivery](delivery.md) for the user-authorized outcome and [implementation Task](task.md) for responsibility, decisions, runtime assignment and evidence. Task/Delivery files own assignments and work state; AgentX maintains only a rebuildable in-memory projection. This capability is independent of `agentx work`.
