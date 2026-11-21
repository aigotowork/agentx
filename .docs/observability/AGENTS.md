---
schema: document
description: Entry for planned AgentX runtime observability data, contracts and engineering governance.
---

# Observability design context

This directory owns the selected observability/management foundation design. Start at [README](README.md), then follow the domain owner and native contract links for the question. Current production support remains in the [accepted daemon outcome](../../.controller/domain-daemon/delivery.md#accepted-local-outcome); these documents describe a new target until implementation evidence establishes otherwise.

Read the [design team contract](../../.controller/observability/AGENTS.md), [selected proposal](../proposals/agentx-observability.md) and your [original Task](../../.controller/observability/controller.md#runtime-bindings) before edits. Define entities/value objects, ownership, concrete types/units, valid state transitions, commands/queries/errors and lifetime obligations before representation or UI choices. Maintain one canonical definition per concept; link current native IDs rather than recreating identity or authority.

Architecture owns shared model/contracts and the artifact catalog. Runtime, configuration and events owners retain their own offered/required domain contracts and rules. Integration owns diagnosis scenarios, consumer boundaries, design acceptance and the implementation task system. The controller owns execution metadata and root navigation. Coordinate bounded shared changes through current AgentX participant identities; no production-source changes or live probes belong to this design batch.
