---
schema: controller
scope: ../../
agent: zomi
deliveries: [./delivery.md]
links: [./AGENTS.md, ../../.docs/proposals/codex-shared-runtime.md]
---

# Coordination scope

The user authorized complete architecture/contracts/protocol design, implementation, domain subagents and verification under this Delivery. zomi owns scheduling, authority, result acceptance, resource coordination and truthful reporting. Professional design and implementation decisions remain with the assigned domain owners.

The split is justified by independent responsibilities and the need to separate implementation from acceptance: architecture/contracts, native runtime service, transactional identity/message state, protocol/launcher composition and integration. Use at most five direct workers; no nested delegation or unsolicited review team.

## Runtime bindings

| Task | Instance | Professional basis | Write scope and next condition |
| --- | --- | --- | --- |
| architecture | kana | Delivery AGENTS architecture route | Assignment accepted and closed; production contracts/model updated, writes released |
| native-service | sake | Delivery AGENTS native-service route | Assignment accepted and closed; implementation/corrections and evidence released |
| identity-messaging | reko | Delivery AGENTS identity-messaging route | Assignment accepted and closed; stores/claims/policy and affected shared paths released |
| bridge-composition | vume | Delivery AGENTS bridge-composition route | Assignment accepted and closed; callable composition/corrections released |
| integration | lobo | Delivery AGENTS integration route | Assignment accepted and closed; exact scenario/static/package/resource receipt delivered in #1902 |

Assignments become real only after actual runtime identity/readiness is returned. Workers record their binding in their own Task; the controller records the live relationship here. Current Task status is not duplicated in this table.

All five native launch receipts returned ready identities. Batches: kana `1b0ac921`, sake `b0a23198`, reko `629b3d1b`, vume `2c9d0f07`, lobo `648363cb`. Readiness establishes runtime availability; domain acceptance remains each Task's own evidence.

kana acknowledged the supplement naming all consumers in message #444; vume and sake reported their claimed assignments and discovery-only boundary in #438/#455. The architecture and integration Task metadata also records kana/lobo as active owners. No extra workers or review workflow was introduced.

## Baseline and resources

There is no Git metadata in the workspace. Before production changes, saved 238 source/config/test files in `.agentx/.tmp/deliveries/codex-shared-runtime/source-before.tar.gz`, SHA-256 `4123805bc4672c47b1596718e5e08e5c335ef3965ab494b0b0840e0c14b15961`. The adjacent `source-before.json` contains individual hashes. Installed AgentX is `/home/long/.local/bin/agentx`; do not replace it during the Delivery.

Only the integration owner runs shared Cargo/test/build jobs. Workers retain their own Task and assigned leaves; architecture owns initial module exports/skeleton and hands subsequent ownership to the implementation owner. No user native/Paseo daemon, config or credential is an experimental resource.

## Closure and recovery

The bounded Delivery is accepted and completed. lobo's final result #1902 establishes the local package and independent scenario/static/resource receipt; kana's #1916 returns that support level to the canonical architecture and protocol model. The controller verified the artifact SHA-256, all 238 entries in the final source manifest, the manifest digest and unchanged installed AgentX hash against package-receipt.json. The global local-bin Codex wrapper is absent. No build/test reruns or live host changes were needed for closure.

Root-reservation failure #1324 was corrected at vume's allocator callsite and verified by actual roots/shared-eof cases (#1484). Parent-child acquisition conflict #1503 was corrected by narrow native-provenance/PATH handling plus explicit config dependency guards, then independently verified (#1754/#1800). Both real native and installed unmodified Paseo 0.15.3 provider API parent-child loops pass with no new user prompt, actual native child launch, business reply and exact SQLite identity/cursor/claim checks. Earlier failures remain in the acceptance report, followed by owned fixes and independent reruns.

All five direct assignments and source claims are closed. Retirement commands for kana/sake/reko/vume/lobo returned success after their final results and resource release. No new work is scheduled under this Delivery. Future work starts from the current source/model and canonical acceptance limits; it must not treat closed worker names as ongoing write authority. Installation/publishing/live-host integration require their own explicit operation and must preserve the user's native global Codex entry.

Initial planning verification passed: eleven planning/guidance files, thirty-four local file/heading references, code fences and five distinct stage-owned Tasks. This is planning structure evidence, not implementation acceptance. Native and provider test/build work remains with lobo.

Inbox checkpoint before awaiting the first contract handoff: targeted request audit after #345 found outgoing request #420, answered by kana #444, and no unresolved incoming requests. All five direct assignments have independent ready discovery/case work; the next coordination event is a released contract, concrete blocker or completed result, not a routine status poll.

User reaffirmed full coordination authority and asked for fast, cohesive execution. Recovered messages through #990: request #667 was superseded by kana #811 and sake's real thread-overlay evidence; sent its explicit acknowledgement to kana. Official managed-daemon direction remains selected, with no alternate-listener fallback. The actual-Paseo environment request #836 is addressed by the installed unmodified provider path now recorded in integration.md; a faithful isolated provider-client run is within scope, while live-user GUI/daemon mutation is unnecessary. All scoped native-service probe resources were reported cleaned up by sake; independent verification must still test the production connector and full composition.

Historical status inquiry checkpoint: lobo acknowledged the actual-client environment in #1075 and authored the unmodified installed 0.15.3 provider cases. Request #1264 concerned unsupported-version acquisition versus observation degradation; scope disposition preserves explicit unsupported shared-service acquisition errors and limits degraded forwarding to a compatible existing connection, consistent with kana #1318. No private-process fallback or broadened native-version claim was required. Request #1324 was acknowledged and routed to vume as the concrete launcher/root fix, with reko informed and lobo continuing independent work. At that checkpoint the native root and parent-child/Paseo gates remained unmet; later evidence and closure above supersede that temporary execution state.

Latest steering and inbox reconciliation: user asked first to remove the global local-bin Codex wrapper and keep ordinary real use unaffected, then continue AgentX. Read-only inspection found `/home/long/.local/bin/codex` already absent, with no codex-named entries in that directory. Fresh shell PATH resolves `/home/long/.nvm/versions/node/v24.14.1/bin/codex` to official `@openai/codex/bin/codex.js`; `codex --version` returns `codex-cli 0.167.0`. No deletion was necessary and no wrapper/config/service was changed. The Delivery scope now prohibits recreating that wrapper; lobo was notified while finishing the local package.

Request #1503 has now been explicitly acknowledged against its domain correction and independent success #1800; no unresolved compatibility fallback remains. Domain owners report source/resource release (#1812/#1828), and lobo retained final package/acceptance ownership at that checkpoint. Runtime/plugin evidence and unsupported environments are in the canonical acceptance report. The original package receipt disclosed missing local plugin type dependencies; the later resolved disposition is recorded below. Do not convert bounded evidence into an unsupported all-platform/provider certification.

Final inbox audit through the closure receipt found no incoming requests after #1890. Known scope/runtime requests #667, #1264, #1324 and #1503 all have explicit dispositions and completed downstream evidence. No blocker or unresolved user decision remains for this bounded local implementation/build Delivery.

## Plugin verification follow-up

2026-11-07: The user requested resolution of the missing local plugin dependencies. zomi completed this bounded follow-up inline; the original workers remain retired. The earlier direct `npm run typecheck` bypassed the dependency setup already provided by `just typecheck`. The supported recipe installed the locked development dependencies and passed typechecking, 25 plugin delivery cases and 71 Hermes cases. Input hashes confirm that plugin sources, manifests, lockfile and check script were unchanged. This recoverable verification prerequisite is now closed. The [follow-up receipt](../../.agentx/.tmp/deliveries/codex-shared-runtime/plugin-typecheck-followup.json) and [canonical acceptance disposition](../../.docs/codex-activation/acceptance.md#development-dependency-follow-up) retain the exact result and the separately discovered development-dependency audit finding. No Cargo rebuild, native service operation or global wrapper change was performed.

The user's subsequent request to resolve the dependency advisories is completed in the standalone [maintenance task](../plugin-dependency-maintenance/task.md). zomi retained direct SDK versions, applied scoped patched-library resolutions, added upstream-consumer compatibility cases and a full-lockfile audit command in Just/CI. The pinned Node/npm gate passes with zero reported vulnerabilities, eight new compatibility cases and the existing type/runtime checks. The final lock is stable under a clean install; original native/package receipts remain historical. No retired worker was reused, and no provider source or live service changed.
