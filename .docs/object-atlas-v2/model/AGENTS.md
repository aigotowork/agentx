# Canonical AgentX contract projection

Read the [model and renderer handoff](README.md) and [layer concept](../layers.md) for declaration ownership and exact anchors. `contract-model.json` owns the curated facts; SDK/native declarations own actual behavior. Keep typed dependencies navigable and source-linked. Preserve exact field and method edges, declaration-layer ownership, visibility and source/proposal provenance. Inheritance, implementation, wrapping and delegation are distinct relations; SDK Agent remains a union.

The [generic tool](../../../tools/contract-canvas/AGENTS.md) owns the portable schema and graph checks. This directory owns checks against AgentX source, status values and decisive domain routes. The sibling [AgentX view](../view.json) owns presentation. Regenerate the offline data script and page together with `node .docs/object-atlas-v2/model/check.mjs --write-wrapper`. This verifies graph/source correspondence, not compiled or running product behavior.
