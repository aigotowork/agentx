---
schema: task
status: completed
description: Own shared native/API contracts, hosting, builds and full composition.
---

# Integration

Read [team entry](../AGENTS.md), [design](../design.md) and [Delivery](../delivery.md). Record scenario-to-native supply, unsupported/redundant concepts, selected design/skeleton, source changes and exact verification here. Implementation is authorized; no separate staged approval is required.


## Selected implementation boundaries

Consumer owners: live-space-lila (existing office UI adapter), live-inspector-zolu (existing Inspector native-backed surfaces). Native owner live-native-sumo owns new inspection domain/contracts/store only. This separation earns its complexity through two independent existing UI systems and a distinct persistent observation/recovery domain; integration owns shared transport/hosting instead of duplicate backends. Native declaration skeletons are coordinated before cross-domain edits.

First integration behavior is same-origin asset hosting: `/space/` and `/inspector/`, finite recursive freshly compiled artifacts. The old flat diagnostic/workbench/management path stays as-is. Module wrappers own compilation/base prefix and provenance, integration owns finite native resource tables and routes. Shared public management generated import remains web/management/api/native.ts; new Inspector declarations derive to web/inspector/host/native.ts.


Shared skeleton now registers Inspection in native application plus safe observation domain labels. HTTP exposes finite declared configure/analyze/cancel/continuation paths and GET snapshot, rejects internal Collect/Run and path/variant mismatch, retains same-origin/generation fences. Both fresh UI bundles embed under owned prefixes. Inspector and Space document CSP allows their existing dynamic style ownership while script execution remains self-only; no change to other document CSP. Runtime collector runs every ten seconds, at most sixteen native-selected Provider sources (32MiB pre-read metadata gate) per round, rotating within a first bounded page; incomplete coverage stays explicit. The metadata gate does not prove a file cannot grow during parsing. Jobs finish outside browser lifetime; native admission checks sample source/session/instance/revision.

Initial conservative storage scan retains fifteen unclassified sites in the new inspection store/helper, now dispositioned by the exact daemon-only call edges in the storage model/checker. No new database constructor or client exception is authorized. Corrected scan and first coherent compile/tests follow source freeze; early generator feedback corrected actual serde snake_case and decimal-string public token counters before consumer use.

Storage-boundary disposition check now passes zero violations in verification/storage-classified.json (421 scanned sources, conservative lexical-policy limits). New domain has no managed-storage opener. Native contract generation includes actual serde request phases and exact decimal-string Token quantities. A local generated-shape sanity initially expected nullable fields without the generator's parentheses; corrected the check to its actual equivalent syntax, without changing the contract. This is static source evidence only. Coherent native/UI compilation is pending Inspector source freeze.


## Coherent build execution

Both consumer sources and the native module are frozen. First coherent attempt [native-live-r1](../verification/native-live-r1-receipt.json) failed at two digest LowerHex expressions against the locked SHA dependency; native owner corrected these using the existing bytewise hex convention. The failure remains retained. The second coherent native/UI compile and targeted native tests run as native-live-r2; no final candidate or runtime adoption is claimed yet.


Coherent [native-live-r2](../verification/native-live-r2-receipt.json) passed all 38 targeted native tests, including the nine inspection boundaries; both fresh UI asset builds succeeded during native compilation. The package-live-r1 run creates a distinct immutable functional candidate with release LTO disabled, 256 codegen units and optimization level 0. This is no release-performance claim. Independent public/browser verification and user-scope adoption remain pending.


Immutable candidate `.bin/agentx-live-ui-r1` packaged successfully: SHA-256 `674dcde675639f15c4d9184cc035313680618b8dcf64a4f8b23c52ef58cb52d8`, [receipt](../verification/package-live-r1-receipt.json). Independent acceptance owner received absolute candidate/hash and execution authorization and acknowledged receipt in AgentX #25271. Current-scope [read-only baseline](../verification/live-before.json) confirms expected generation, project, wildcard LAN binding and paged real data; no user collection/recovery/Agent mutation occurred. Final adoption waits for independent acceptance.


## r1 independent acceptance feedback

Independent API cases passed on r1; actual browser Space launch, native message/reply, transcript and lost-response/reload original-receipt recovery passed. Inspector management operations were rejected because its local requestId helper produced a prefixed hex string instead of the native-required UUID. This is a substantive product issue, routed to the Inspector owner; r1 is not accepted for final adoption. The original browser failure remains in verification/live-r1-browser3/browser/report.json. No native contract relaxation is allowed.

Consumer design review of r1 Space reply/transcript and Inspector default 1440×900 captures confirms original visual systems remain: Space exposes offices, exact members, adopted Role, record channels and contextual actions; Inspector exposes its eight-tab shell, native session identity and local operations. Screenshot inspection establishes these layouts only, not operation success. Scene and narrow captures are requested in the affected browser continuation.


Inspector owner froze the F01 UUID repair in AgentX #25405; focused adapter tests now verify native-compatible UUID submission for all four management actions and proper v4 fallback in insecure LAN contexts. Native source/contract and Space are unchanged. Packaging a unique r2 candidate; no unaffected native test repeat is justified. Browser continuation on r2 must verify the actual fixed operations before final acceptance.


## Integrated acceptance closure

Contribution accepted in the [completed Delivery](../delivery.md#accepted-local-outcome-and-running-lan-service). That entry links exact native, independent API/browser and final LAN adoption evidence and their separate limits. Earlier source-only handoff statements remain historical; maintenance follows the native declarations and module guides, without reviving this delivery team.
