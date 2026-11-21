---
schema: proposal
description: Make AgentX context embedding usable through a LAN floating demo and packaged Chrome/Edge sidebar extension.
status: accepted
deliveries: [../../.controller/context-clients/delivery.md]
models: [../agent-management/web-context.md]
---

# Usable context clients

The accepted iframe/addContext/attachment contract is implemented and adopted. The user now needs concrete applications: a business-style page reachable on LAN with a bottom-right expandable AI overlay, and a Chrome/Edge extension ZIP they can install. Both select an existing exact Agent conversation, capture contextual text/files/images, and preserve deliberate send in the embedded UI.

The demo should show a coherent real-world task with selectable page information and contextual ask actions, rather than another protocol/debug harness. The extension should provide a browser side panel with service/target selection, page selection/text, visible-tab screenshot and attachment intake. Both reuse the existing context envelope and current native reads. No new-Agent launch, fake AI replies, paid model fixture disguised as live support, store publishing or signed CRX distribution. ZIP contains an unpacked MV3 extension with concise Chinese load/configure/use instructions. Existing service and user Agent lifetimes remain unchanged.

Detailed client design belongs to each module README/design, with exact semantics linked to the native contract instead of duplicated. Actual LAN startup and package/runtime evidence belong to the [Delivery](../../.controller/context-clients/delivery.md) and Tasks.
