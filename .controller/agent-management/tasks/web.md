---
schema: task
description: Design and implement a compact embedded React management console from the native HTTP contract.
status: completed
---

# Web console

Read team/design/native contract and shared/local interface guidance. Own `web/management/` except generated `api/native.ts`. First record a short UX model and SVG in this directory; then implement a compact light console with Agent list/status, selection, message/history/transcript, launch form, stop/resume, pending/unknown operation lookup and errors. React and Vite are already installed. Main export contracts use required JSDoc tags. Return expresses structure; use named calculations. Use CSS tokens and local disclosure, no console-like technical fields cluttering the core experience. Integrator owns build.rs and Cargo, but frontend typecheck is allowed. Backend URLs and contract are in design; do not invent DTOs. Use fixture data only for your design/state tests, never imply live support.

Source handoff, strict TypeScript, production bundle and isolated desktop/mobile browser cases passed. [Owned UX/evidence](../../../web/management/design.md) records exact commands and fixture limits. Actual daemon/browser integration remains the Delivery acceptance owner's responsibility.
