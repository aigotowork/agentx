---
schema: document
description: Design management products and workbenches through user tasks, object relationships, contextual actions and complete work paths
---

# Management product design

First read [product and interface design requirements](../../scenarios/interface-design.md). Their layout, information efficiency, disclosure, visual language and semantic implementation constraints apply to management pages and workbenches, including bounded fixes and narrow screens.

Use for management consoles, operational workbenches and products where people organize multiple subjects, act on them and inspect their outcomes. This domain owns the product experience connecting those subjects; it does not define a universal application shell or replace domain contracts. A bounded visual or component fix can use its existing Web guide directly.

| Current question | Read and produce |
| --- | --- |
| What makes this a useful management product? | [Principles and scope](README.md): user decisions, object relationships and complete work paths |
| How should a product or substantial redesign proceed? | [Workflow](workflow.md): evidence, model, navigation, prototype, implementation and bounded acceptance; start at the unresolved stage |
| How should a page organize information and actions? | [Page design](page-design.md): page responsibility, density, representations, state and continuity |
| How do these decisions connect in practice? | [Worked example](worked-example.md): one fictional product, including branches, history and recovery |
| Which methods were borrowed, and with what limits? | [References](references.md): primary sources and specific adaptations; optional further reading |

The explanatory methods are in Chinese. Apply their decisions to the consumer's language and terminology. Do not require consumers to read the author's research, reproduce the example's schema, or install an external Skill.

Before prescribing a layout, establish the actor's task, the selected subjects and relationships, the action's actual owner, and the visible outcome. Reuse existing facts and scenarios. Keep source history, live state, declarations and observed outcomes distinguishable. Presentation geometry must preserve relationship meaning. Preserve drafts and in-flight operations across view changes according to their actual lifetime.

For broad work, demonstrate a complete path from finding the right object through an authorized action to its result and return. For a read-only product, demonstrate the decision or investigation it supports. Read-only scope is legitimate; never add unsupported management actions just to make the interface look complete. Use actual rendered and interaction evidence for claims about the experience; label simulated behavior and distinguish an Agent walkthrough from user acceptance.

## Compose existing owners

| Concern | Owner |
| --- | --- |
| General connected product design and the canonical scenario source | [Product design](../../scenarios/product-design.md); [professional methods](../product-design/AGENTS.md) for a selected question |
| Maintaining connected design knowledge over time | [Design management](../design-management/AGENTS.md) |
| A bounded inspector or tool page | [Tool interfaces](../webdevelopment/tool-interfaces.md); broader navigation and product organization use this domain |
| Complete tool-task, cognition, precision and interruption review | [Tool experience review](../product-design/scenarios/tool-experience-review.md); bind its findings to the existing scenario and native responsibility |
| Domain meaning, service ownership and integration | [Engineering design](../../scenarios/engineering-design.md) and [modeling](../modeling/AGENTS.md) |
| Page/component structure, visual system, interaction and rendered review | [Web development](../webdevelopment/AGENTS.md), selecting only the affected guides |
| Selected delivery outcomes and task coordination | [Feature planning](../../scenarios/feature-planning.md); existing project records and actual delegation authority |

Keep the consumer's decisions, evidence and unresolved work with that product. These methods add no approval gate, mandatory reviewer, required document set or permission to modify external systems.
