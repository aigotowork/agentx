---
scope: Scoped Cloud Space web mode
status: active
owner: Space Web
related: [../../../../.controller/cloud-space/wire.md, ../../README.md]
---

Cloud mode is selected at the scoped URL before native Space or inspection mounts. It shares Space theme, office shell and Canvas. Cloud admission, exact recordings and mutations remain cloud SDK responsibilities.

| Boundary | Contract |
| --- | --- |
| `route.ts` | Parse the canonical scope and exact host/name/incarnation, bind links to generation, and own history. URL values cannot choose a credential or a host listener. |
| `model.ts` | Bind the browser's selected bearer to the same origin; load and fence cloud inventory by scope generation; map registration facts into scene resources without inventing status, roles or workspaces. |
| `CloudSpace.tsx` | Explicit connect/disconnect and audience controls; same exact selection in scene and roster; host catalogue and original receipts through the existing Deployment control components. |
| `CloudMessages.tsx` | Read-only scope-bearer mailbox stream. Preserve native older cursor and exact endpoints, including registered retired tuples. |

The first view keeps the warm Space office on the main axis. A selectable roster sits beside it; the selected identity anchors cloud records, with host catalogues and receipts in contextual drawers. The matching roster/scene selection is the visual signature. On narrow screens the surface stacks and scrolls as one workspace. Deployment is scope discovery for admins; it and Space ask for their respective credentials explicitly on each page.

Admin exact links resolve against paged current registration; Scope links can use the durable exact message projection for retired identities. Unknown generation, unavailable or unauthenticated data, empty result and absent identity have different states. Scope bearer never implies Agent sender identity. The native deployment/SDK owner owns the shared wire and asset admission; Space owns presentation only.

Browser verification: build the public SDK first, then run `node web/agent-space/build.mjs <new-absolute-output>` and `node web/agent-space/tests/cloud-space.browser.mjs <output> [evidence-directory]`. The fixture serves the built assets and the coordinated JSON contract on loopback, with no user credentials, host or deployed scope.
