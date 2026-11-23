---
scope: Scoped Cloud Space Web
status: active
owner: Space Web
related: [../../../../.controller/cloud-space/shared/web.task.md, ../../../../.controller/cloud-space/wire.md, ../../README.md]
---

The scoped URL admits a bearer before mounting the same App, reader, composer, Agent facts, and launch/stop dialog as the native entry. Start with the [Web Task](../../../../.controller/cloud-space/shared/web.task.md) for design, checks and evidence limits. Use the [public wire](../../../../.controller/cloud-space/wire.md) for transport guarantees.

| Boundary | Contract |
| --- | --- |
| [CloudSpace.tsx](CloudSpace.tsx) | Mount only the login gate before admission; preserve the existing audience and token-duration choices. |
| [model.ts](model.ts), [route.ts](route.ts) | Bind the SDK bearer to the same origin and map nonsecret host, name, incarnation and generation into shared navigation. Treat hosts as grouping, not native workspace membership. |
| [workbench.ts](workbench.ts) | Project current registrations and independent exact readers into App. Use the host catalogue's aliases and roles for launch. Verify installation and exact registration before stop. |
| [history.ts](history.ts), [operations.ts](operations.ts) | Keep older and live pages independent. Render authenticated durable messages and real cloud receipts; retain only control identities in session storage. Never automatically resubmit a write. |

Scope bearer reads only. Admin can send text to a current exact registration and use the original receipt for control recovery. The accepted message ID is not a delivery acknowledgement. Retired registrations remain linkable when their exact histories remain available; never navigate to their replacements.

Build the public SDK, then run `node web/agent-space/build.mjs <new-absolute-output>` and `node web/agent-space/tests/cloud-space.browser.mjs <output> [evidence-directory]`. The browser uses built assets and disposable loopback routes.
