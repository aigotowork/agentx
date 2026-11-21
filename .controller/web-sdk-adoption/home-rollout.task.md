---
schema: task
status: completed
description: Adopt the accepted SDK-backed Web artifact on the existing home-scope daemon at port 38000.
---

# Authorized outcome

The human authorized migrating the existing `http://192.168.2.178:38000` daemon to the SDK-backed solution and assessing remaining gaps on 2026-11-18. This explicitly extends the completed source-only [Delivery](delivery.delivery.md#accepted-local-outcome) to selected live adoption. Tele owns this bounded rollout inline; no new feature team or repeated independent acceptance is needed.

# Model and boundaries

Adopt the independently accepted R3 artifact, SHA256 `935172a4ca3a5f6c7fc4d369bd65a768b0e7baef1c0d795b098dd0167afd1950`, rather than rebuilding the unrelated dirty checkout. Management, Space, Inspector, Workbench and Observation use the public SDK; daemon-native services and IPC retain their native authority. Browser extension and demo embed Space and have no second HTTP client to migrate. Three listener URLs remain independent; port 38000 owns Management/Space/Inspector, not every domain endpoint.

Preserve home scope `/home/long/.agentx`, generation, stored exact Agent references, messages, configuration and running Provider/PTY hosts. Use native exact-scope graceful restart and a persistent named executable. Keep installed/default binaries and unrelated daemons unchanged. Retain the original installed executable as rollback and take an online SQLite backup before transition. Never replay old Unknown operations or launch paid Agents as rollout probes.

# Verification and assessment

Reuse accepted R3 source/package/native/browser/extension evidence. Rollout checks cover artifact identity, native client compatibility, preserved generation and participant references/processes, actual LAN embedded pages and asset bytes, SDK read access and browser cross-origin access. Assess remaining native launch incident evidence separately from role-directory correctness; old Unknown receipts remain historical uncertainty. Platform, dependency and broad-suite limits remain owned by the Delivery, not silently reported as fixed.

Evidence and final disposition will be recorded here after execution. Private database backup and exact runtime receipts live under `.artifacts/home-sdk-rollout-20261118/`.

## Adopted live outcome

This SDK R3 adoption remains historical evidence and the direct rollback for the subsequent [runtime read and failure repair](../runtime-read-repair/implementation.task.md#accepted-selected-service-outcome), which is now the actual selected :38000 daemon. Use that repair's persistent binary for current restart commands.

The selected home-scope daemon was gracefully restarted using persistent `.bin/agentx-home-sdk-r3` on 2026-11-18. SHA256 matches the independent R3 acceptance artifact. Native version remains `0.7.31`; build identity is `native-build-v1:46020fb2-3808-42c1-9775-bb617ab61ae3`. Actual owner PID4112505, instance `161245aa-8d5c-4928-9014-9defdd506926`, generation `de88b405-d20d-486d-b330-867671858429`. Addresses remain `http://192.168.2.178:38000/`, `/space/` and `/inspector/`. The browser extension can use its original `:38000` setting again; the temporary `:38001` frontend is not required by this deployment.

The online backup was taken before restart with SQLite backup API, private mode0600 in the mode0700 evidence directory. [Before receipt](../../.artifacts/home-sdk-rollout-20261118/before.json) and [after receipt](../../.artifacts/home-sdk-rollout-20261118/after.json) establish all33 stored name/PID/created-at tuples and all33 observed process start ticks unchanged, unchanged configuration, and preserved generation. Native installed old client and new client both connect to the new daemon. Installed/default programs, project default `.bin/agentx`, other scopes and Provider/PTY hosts remain unchanged.

[Actual public SDK reads](../../.artifacts/home-sdk-rollout-20261118/sdk.json) cover descriptor, current Agents, exact Agent lookup, workspace catalog, DancingAI effective roles, Inspector snapshot and original historical receipts. Project coder resolves to `/home/long/works/longwork/dancingai/.agentx/roles/coder.toml`. [Actual Chromium LAN pages](../../.artifacts/home-sdk-rollout-20261118/browser.json) cover Management, selected DancingAI Space posts at390px and Inspector page/API loading: zero page exceptions, zero POSTs and no document horizontal overflow. Asset hashes are in the after receipt. [Cross-origin browser evidence](../../.artifacts/home-sdk-rollout-20261118/cors/browser-result.json) verifies live descriptor Fetch and actual OPTIONS preflight from a separate localhost origin. Inspector Agent-page200 is additionally retained in [read evidence](../../.artifacts/home-sdk-rollout-20261118/inspector-read.json); full rendered directory completion is not claimed by this short live probe. Reuse the accepted R3 complete fixture/iframe/extension/Inspector interaction evidence; no paid launch/send, collection enablement or automatic recovery was performed on user data.

### Operational commands

Restart this selected new version with `/home/long/works/longwork/agentx/.bin/agentx-home-sdk-r3 daemon restart --scope /home/long/.agentx`. The default installed command still points to the earlier release; using that old executable to restart/acquire a stopped daemon would adopt its older embedded Web. This deployment intentionally does not replace the account-wide program used by unrelated scopes.

Rollback only the selected daemon with `/home/long/works/longwork/agentx/.bin/agentx-home-before-sdk daemon restart --scope /home/long/.agentx`. The retained rollback SHA256 is `b00d60e8d06a383bb5e49281cc595d90e9c759a5fe1e4dd60fa18b9c65627e97`. Ordinary rollback uses current data; do not restore the database backup over subsequent user activity.

## Remaining gaps and evidence limits

| Finding | Evidence and disposition |
| --- | --- |
| Occasional busy responses during actual parallel page reads | The first Management global-role load returned503, then the same endpoint returned200. Space subsequently rendered the correct nine project roles but a periodic workspace refresh returned503. Both appear in retained browser receipts, including `browser-initial-503.json`; no all-green availability claim. SDK adoption does not eliminate native queue/read contention. Further work should measure the native/read workload before changing retry or admission policy. |
| Inspector delays rendering while collecting paged directory | Live page/descriptor/inspection/first Agent-page return200, but the short probe still shows loading. `web/inspector/host/live/client.ts::directory` accumulates up to20 pages before publishing. Full directory first-render latency on this real history was not measured; fixture interaction acceptance is separate. |
| Prior launch incident remains undiagnosed | Original two launch receipts remain Unknown with ENOENT. Stored request directories are absolute existing paths and worker was never claimed. Earlier conversational suggestions that shell tilde expansion or the valid DancingAI prompt path caused these launches were unsupported and are withdrawn. SDK migration neither rewrites receipts nor proves the missing executable/cwd source. Native worker stderr is discarded in `management/service.rs::start_worker`, limiting retrospective diagnosis. |
| Scope and endpoints are distinct | Management/Space/Inspector on38000 now use SDK. Workbench/Observation implementations also use SDK but keep independent listeners and were not silently started/repointed. Daemon-native IPC is intentionally not converted to HTTP SDK. Extension/demo embed the full Space and do not have duplicate API transports. |
| Packaging and default adoption remain bounded | This accepted candidate is a347MiB unoptimized dev build, not a performance-tested release package. Persistent scoped artifact is deployed; account-wide installed executable remains older. An optimized release/default-program promotion is separate from this selected daemon adoption. |
| Preexisting accepted limits | Linux Chromium147/156 finite evidence, no paid Provider/model proof, no Node22 execution (host24 outside root range), three existing moderate plugin audit findings and retained unrelated broad Rust failures remain in the accepted Delivery. They are not represented as resolved by deployment. |

The migration and bounded gap assessment are complete. Remaining findings above are explicit maintenance inputs; no unresolved deployment rollback or source modification is hidden. Exact runtime receipts and sensitive backup stay local; no user conversation bodies are copied into maintained documentation.
