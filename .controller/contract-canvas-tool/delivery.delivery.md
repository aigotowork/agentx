---
schema: delivery
status: completed
description: Extract a reusable Contract Canvas tool and ground the next Space and Management UI designs in the domain SDK and project models.
stages:
  tool: {description: "Independent tool, separate AgentX data, and unrelated consumer proof", tasks: ["./tasks/tool.task.md"]}
  web: {description: "Scenario-to-domain-to-view design and implementation handoff", tasks: ["./tasks/web.task.md"]}
---

# Reusable Contract Canvas and Web domain design

The user wants a tool other projects could adopt and wants to design Web UI and its canvas objects as maintainable domain consumers. Contract Canvas reads design declarations; the Space canvas presents instances and captures user intent. They must not depend on each other's runtime. AgentX graph facts and evidence stay in the project; tool code must run with an unrelated project fixture without needing AgentX files or identities. Use an explicit documented portable model/view input and maintain the original consumer experience.

Parallel professional responsibilities are justified by the independent engineering and product/model design work. Tool owner `hizare` (`coder`) handles extraction, the original AgentX consumer and unrelated fixture; experience owner `muzabe` (`product-experience`) handles related but separate Web design and source trace. Kula coordinates scope and accepts the two results. Ownership is file-separated; they share the existing SDK and model references, without requiring any reader or scene implementation to depend on the new tool. No production Space/Management migration, service rollout, generalized framework or new AgentX capabilities are selected in this delivery.

Acceptance requires the unrelated project to use the same generic entry as AgentX: no copied renderer or rename-to-AgentX trick. In that example, follow a typed field through a foreign layer and return, and visit a scenario without any AgentX assets. The original consumer must still open at its existing HTTP URL and from a local file. Cross-layer disclosure, per-project scenarios and optional presentation metadata must remain distinct from authored semantic facts. Review the owner's bounded evidence instead of rerunning unaffected original checks.

For Web, review one end-to-end trace per product from user intent to authoritative SDK/native result to local view. A Space avatar/office-room instance carries a reference to the application object, not its own conflicting copy of Agent lifecycle. List and Office views share one conversation selection/draft owner. Management retains query/coverage state and can use the same lower handle without sharing Space's objects. Distinguish declaration walk-through from real objects and state in Web runtime; the two resulting artifacts have different consumers.

## Accepted tool extraction

[Contract Canvas](../../tools/contract-canvas/README.md) owns the viewer, portable model/view schema, validation and static builder. The [AgentX consumer](../../.docs/object-atlas-v2/README.md) owns the unchanged declaration facts, project-specific source checks and authored view settings. The two consumers use a single renderer. A relative asset link keeps the existing :18988 HTTP root and the checkout-local file entry usable without restarting the listener. New consumers can copy only the tool and provide their own model/view inputs; the built-in library example is unrelated to AgentX.

The [focused report](verification/report.json) and [tool owner handoff](tasks/tool.task.md#delivered-and-verified) establish the copied-out offline example and exact rebuild, minimal consumer, invalid reference/ownership/view rejection and inert display of authored text. They also cover actual AgentX HTTP, local file and narrow touch use, with typed membership/operation paths, walkthrough, peek/pin and return. The Controller inspected the [library walkthrough](verification/library-walkthrough.png) and [AgentX narrow view](verification/agentx-narrow.png) and accepts the portability and compatibility scope. This is a standalone source tool and static viewer; no repository split, registry publication or production SDK/Web deployment was performed.

## Accepted Web design handoff

The [Web design](../../.docs/web-product-design/domain-sdk-scenarios.md) and [owner handoff](tasks/web.task.md#design-handoff) satisfy the selected design scope. Space reuses one session/roster and exact conversation owner across Office and List; its scene has room, actor and prop nodes, one presented frame and a selection-intent port. Management has a separate workspace inspection path and distinguishes a current Role definition from saved adoption. Each scenario names existing receiving modules and observable sparse-paging, stale identity, return and read-only criteria. The only configured host precondition is an independently trusted listener and expected scope; current embedded initialization alone does not select the new domain path. The bounded first reads do not promise complete workspace membership or a latest/backward timeline, and existing write/receipt ownership is preserved. This is design acceptance; Web adapters and runtime behavior remain a separate implementation.
