---
scope: AgentX contract model and view consumer
status: active
owner: domain-contracts
related: ["../../tools/contract-canvas/README.md", "./model/README.md", "./layers.md", "../../.controller/contract-canvas-tool/delivery.delivery.md"]
last_updated: 2026-11-21
---

# AgentX Contract Canvas model

Open [the checkout-local entry](index.html) or the existing [LAN preview](http://192.168.2.178:18988/). For general usage, model authoring, view configuration and tool maintenance, start at the [Contract Canvas documentation](../../tools/contract-canvas/README.md). This directory contains the AgentX consumer and its curated facts.

- **应用模型 → 会话与草稿** shows the proposed Office/Actor model, exact conversation selection, draft reopening and advisory send availability. See the [application design](layers.md); the walkthrough does not execute the proposal.
- **Domain SDK → SDK 总览** links four caller paths: scope and membership, conversation records and provenance, commands and receipts, and scoped resources. Follow a method's input or output to its fields, then preview or compare the referenced type in context.
- **Workspace.members** retains the original global page projection. **Workspace.memberPage** shows the separately advertised native filter and its own cursor. **发送与恢复** retains raw receipt lookup by address; **operation(prepared)** shows typed recovery from a retained command, without replay.

The [current SDK model](../model/typescript-domain-sdk.md) explains compatibility: an older listener can omit native member support, and a valid older launch receipt can lack exact addresses. The [package Task](../../.controller/typescript-domain-sdk/tasks/contract-completeness.task.md#completed-package-outcome) owns package verification. Canvas checks source correspondence and offline artifact integrity; live adoption is a separate claim. See the [earlier audit](../model/typescript-domain-sdk-review.md) for the pre-extension class diagram and gaps.

| File | AgentX responsibility |
| --- | --- |
| [model/contract-model.json](model/contract-model.json) | Curated declaration facts, scenarios and source evidence |
| [view.json](view.json) | AgentX labels, display order, starting subject and optional scenes |
| [model/check.mjs](model/check.mjs) | AgentX source and scenario checks plus generic validation and static assembly |
| [index.html](index.html), [model/graph.js](model/graph.js) | Generated entry and data; do not hand-edit |
| `viewer/` | Relative link to the [shared tool assets](../../tools/contract-canvas/viewer/); the existing listener serves through this link |

From the repository root, run `node .docs/object-atlas-v2/model/check.mjs`. After changing either input or the shared template/embedded layer helpers, add `--write-wrapper` to regenerate the two outputs. To reproduce the LAN preview only when the listener is absent, use `python3 -m http.server 18988 --bind 192.168.2.178 --directory .docs/object-atlas-v2`. If exporting a local AgentX page outside the repository, copy the pointed-to viewer files into its `viewer/` directory instead of copying the link alone.

The [extraction Task](../../.controller/contract-canvas-tool/tasks/tool.task.md) owns current HTTP, direct-file and independent-copy verification. The [accepted layer walkthrough](../../.controller/object-atlas-v2/layers.delivery.md) and [reference comparison follow-up](../../.controller/object-atlas-v2/tasks/reference-preview.task.md) record earlier changes and their evidence.
