---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"salu","incarnation_bits":4745317318819039043}}]
---

Own web/agent-space only, including browser tests. Replace CloudSpace separate workbench with shared local App, shared conversation reader/composer, agent info and launch/stop components, same role picker. Keep authentication gate and host-specific data adapter. Use existing cloud SDK for inventory/control; new admin SDK methods client.listMessages(scope,query,opts) and client.sendHumanMessage(scope,{generation,request_id,to:{host,name,incarnation},body},opts) return MessageHistoryPage and MessageView respectively. Scope bearer supports read-only history. Preserve exact instance identity incl host and generation, shared links, session retention, disabled actions, real receipts. Implement in shared checkout. Test against fixtures covering actual entry, role launch/chat/reload/stop. Parent owns SDK and native owner owns wire. Report completed changes and remaining gaps. Do not publish/deploy.

## Workbench model

- The scoped entry authenticates and mounts App with a single admitted cloud adapter. App, workbench navigation, conversation windows, Agent facts, command dialog and launch role picker own interaction for both hosts. The local adapter retains existing Domain SDK behaviors.
- The cloud adapter owns paged host/registration inventory, exact readers, cloud history and write receipts. A cloud subject is addressed by scope, generation, host, name and incarnation. Host groups are placement, not native workspace membership. Shared links keep the cloud URL tuple, accept old list links and pin generationless links after successful restoration.
- Cloud host + installation selects the write authority. Launch uses the selected host catalogue's workspace alias and role; stop requires a fresh match for the captured installation and registration. Scope login is read-only; cloud does not claim native Composer metadata, attachments, Fork, relationship or transcript capability.
- Cloud history has independent older and live reads; write acknowledgement is the returned MessageView, not a delivery claim. Control receipts preserve the original request id and native cloud status, with lookup-only recovery. Uncertain sends can explicitly replay only the original in-memory request.

## Web implementation outcome (2026-11-23)

Implemented in the shared checkout. CloudSpace now remains only the authentication and session-retention gate; it constructs CloudWorkbench and mounts the same App used by native Space. Cloud registration rows are host-grouped while exact refs include scope, generation, host, name and incarnation. CloudHistory reads durable pages with separate older/live lanes. Admin sends use the SDK human-message method and retain uncertain request identity; the returned MessageView is the accepted receipt and remains visible after reload. Admin controls retain native control receipts and installation fences. A successful launch receipt opens the returned exact instance only if the user has not navigated since submit.

### Validation

- TypeScript check and fresh static Space build passed.
- Cloud browser fixture passed: shared App, role launch, successful receipt navigation, delayed launch/navigation fence, human send replay, Agent reply, reload, installation-checked stop, read-only Scope history, stale generation, 320px layout and native demo separation.
- Route and interaction tests passed all 6 cases. The native conversation-workbench browser regression passed on the same artifact. Browser build and screenshots: `/tmp/agentx-cloud-workbench.FXaQVO/`.

Remaining cloud limits are deliberate adapter boundaries: Scope bearer is read-only; cloud does not expose attachments, Provider Transcript, Fork, launch relations or local workspace/role membership. Native SDK/wire changes are parent-owned and were consumed through the generated methods.
