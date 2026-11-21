---
schema: proposal
description: Embed existing conversations and add page, image and file context through a shared composer path.
status: accepted
deliveries: [../../.controller/web-context/delivery.md]
links: [../agent-management/README.md, ../../.controller/live-ui-integration/design.md]
---

# Embedded conversation context

## Selected outcome

An existing AgentX conversation can be opened in an iframe or browser-sidebar wrapper, focused through query parameters, and supplied with page text, screenshots or files. The user can review/remove pending context and deliberately send it with a message. Native attachments persist on the selected AgentX host and are referenced by readable filesystem paths in Agent messages; Web consumers use attachment IDs and preview/download URLs.

## Accepted boundaries

Iframe embedding is allowed without an origin allowlist, accounts or an authorization handshake, per the user's explicit personal-host usage decision. A simple parent-window check and payload validation remain appropriate. URL parameters only navigate/prefill; they never send, upload or launch. Context targets an exact conversation instance, never a namesake selected later. New-Agent postMessage operations are deferred. Direct Provider multimodal injection, OCR, document conversion and a packaged browser extension are outside this first slice; a documented extension wrapper bridge can consume the same protocol.

## Current and target

Current management messages are text based; current response CSP prevents framing, and Agent Space lacks a complete attachment upload/delivery path. Preserve the existing durable operation receipts, scope/generation fencing and native recipient rules. Extend the existing conversation and management boundary rather than adding another UI or business daemon.

The implementation owner refines exact protocol, upload/storage contracts, state behavior and finite limits in [the feature design](../agent-management/web-context.md) before behavioral implementation. The [Delivery](../../.controller/web-context/delivery.md) owns actual support and acceptance; acceptance of this Proposal is not implementation evidence.
