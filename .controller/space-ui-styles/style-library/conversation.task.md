---
schema: task
status: completed
owner: kula
description: Keep one real conversation contract in every Space appearance.
---

# Space conversation contract

The selected LAN bundle contains the old note labels, but those notes are page-only synthetic records, outside the AgentX domain. Remove the third channel and the action that turns drafts into synthetic records. Message history and deliberate send remain with Space and the public SDK. Transcript is a separate read-only channel with an explicit Provider acquisition action and exact receipt handling. Keep the historical `channel=model` link; invalid/obsolete `channel=notes` falls back to messages.

The same controller owns exact Agent selection, drafts, attachments and receipts across pixel and illustrated styles. Style owns presentation only. Design Canvas supplies sample histories and editable drafts without adding records or making native calls. Existing service adoption stays separate; verify the isolated browser path with the real native fixture where available.

Recovery: event #62081 was queued for the Citrus, Reverie and Wash owners. Each original session was at a ready, empty prompt after a Provider failure; a deliberate continuation in those same sessions restarted work and exposed the queued message. The completed Collage owner did not need a wake-up.

## Implemented and verified

[Conversation](../../../web/agent-space/src/conversation/Conversation.tsx) owns the two channels, pinned native operations and the exact draft boundary. [ConversationBlock](../../../web/agent-space/uiux/systems/space/blocks/ConversationBlock.tsx) renders those channels in every style. The static specimen uses the same component; it cannot submit or turn drafts into records.

A GET-only browser visit to the selected :38023 deployment confirmed the labels `消息 / 模型记录 / 本页笔记` in the real reader, served by `index-BB7s8Pgw.js`. This explains why the prototype inherited that extra concept. No POST was made during verification.

[Isolated native browser report](../../../.artifacts/space-ui-styles/style-library/native-r1/browser-report.json) establishes pixel → watercolor continuity, a successful exact AgentX send and Qoder fixture reply, explicit Transcript acquisition, and return to the pixel style with the unfinished draft intact. The report uses the public HTTP SDK against an actual AgentX daemon and its existing disposable Provider fixture; it does not exercise a paid model. [Fixture manifest](../../../.artifacts/space-ui-styles/style-library/native-r1/manifest.json) records successful cleanup. Production was not changed.
