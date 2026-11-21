---
schema: task
status: completed
agent: zomi
description: Remediate the plugin development dependency advisories while preserving the real upstream SDK contracts and adding a maintained audit gate.
links: [../../AGENTS.md, ../../CONTRIBUTING.md, ../../package.json, ../../scripts/typecheck.sh, ../../.docs/codex-activation/acceptance.md]
---

# Outcome and boundaries

Resolve the 31 affected package entries reported by the [baseline audit](../../.agentx/.tmp/deliveries/codex-shared-runtime/plugin-dependency-audit.json). This is the user-requested maintenance of the existing plugin check workspace, not a new provider architecture. Work inline: the changes share one npm graph and existing checks; no independent feature implementation or new worker is needed.

The installed provider tools, global Codex entry, credentials, native daemons and packaged Rust executable are outside this task's mutation scope. The upstream SDK packages remain the actual source of plugin contracts; do not replace them with copied declarations, weaken TypeScript checking, suppress advisories, omit development dependencies or downgrade supported SDK majors to make an audit pass.

# Model and selected design

`package.json` owns SDK profiles and narrowly scoped security overrides; `package-lock.json` owns exact resolution and integrity. `scripts/typecheck.sh` installs that graph and checks provider contracts/runtime fixtures. A separate npm/Just audit command checks the entire locked graph, including development dependencies, and the existing CI plugin job runs it. Compatibility cases live beside the current plugin tests and challenge the actual upstream consumer resolution and public APIs changed by the fixes.

The baseline contains five advisory-bearing library families: glob, minimatch, TOML, Undici and Zod. Other reported entries propagate those findings. Chromium BiDi currently requires Zod 3; use Puppeteer's supported same-major update to remove that old schema dependency rather than force Zod 4 into a Zod 3 consumer. [Registry metadata](../../.agentx/.tmp/deliveries/plugin-dependency-maintenance/registry-latest.json) shows that some latest SDK versions still depend on affected libraries, so upgrading every SDK is neither sufficient nor necessary.

| Resolution boundary | Candidate correction | Compatibility obligation |
| --- | --- | --- |
| glob 13 | Patched glob 14.0.4 | Preserve the glob/globSync behavior used by npm workspace discovery and OpenCode utilities |
| minimatch 10 | Patched minimatch 11.0.5 | Preserve named/function/class matching and ignore patterns used by real upstream consumers |
| TOML 4 | Patched TOML 5.0.1 | Preserve the namespace `parse` call used by Effect and normal configuration values |
| Undici 6 and 8 | Patched versions within each existing major | Preserve each consumer's HTTP API; do not force all consumers onto one major |
| Zod 4 | Patched Zod 4.6.7 | Preserve schema parsing and validation behavior |
| Puppeteer 25 / BiDi / Zod 3 | Patched Puppeteer 25.16.0 | Use Puppeteer's own compatible child versions and retain OMP extension typing |

Overrides must select affected version ranges and use exact patched versions. Retire an override when the upstream dependency no longer needs it. Keep the existing direct SDK version profiles unless a concrete compatibility failure requires a coordinated update.

# Verification and completion

The existing audit is the failing security baseline. Add normal, benign compatibility cases for real consumer workspace discovery, file patterns, TOML parsing, schema validation and local-loopback HTTP; no vulnerability payloads are needed. Run the final supported plugin gate after the graph and checks are ready, and run the explicit full-lockfile audit. Confirm that the selected SDK profiles remain installed, npm dependency resolution is valid and the lockfile is stable under a clean install. Preserve failed attempts and corrections in the task evidence directory.

Finish with zero reported audit vulnerabilities, passing dependency compatibility and existing plugin gates, current contributor instructions and acceptance disposition, and no leaked task processes or changes to live user tools. Changes to development manifests/checks do not require a Rust rebuild when embedded plugin sources remain unchanged.

# Working record

- 2026-11-07: Derived the dependency paths from the retained baseline audit and inspected actual installed consumers. Latest SDK metadata was collected before choosing the version policy. Registry output can be either an object or a scalar version when only one field exists; normalize that shape in follow-up reads rather than repeating the registry queries.
- Materialized range-scoped overrides and a full-lockfile audit command, wired it into Just/CI, and authored consumer compatibility cases without changing provider behavior. A cached isolated Node `22.23.3` / npm `10.9.9` pair matches the repository's CI profile without replacing the user's tools. The first resolved lock reduced the audit to three propagated minimatch entries. A targeted npm update retained the stale nested version; read-only Arborist inspection then proved that edge was invalid against the effective `11.0.5` override. An explicit npm dedupe is the selected graph correction; do not suppress that invalid edge or hand-edit the package's integrity/version fields.
- npm dedupe also retained the proven invalid nested node. Regenerated that subtree in a disposable directory seeded with the existing manifest/lock, omitting only the invalid minimatch node. npm generated the replacement resolution and integrity metadata, then the resulting lock was adopted after checking the unchanged direct SDK profiles. The isolated directory was removed. Original attempts and the invalid-edge inspection remain under the task evidence directory.
- The final `just ci audit typecheck` run used the pinned Node/npm profile and passed. Full-lockfile audit (including development dependencies) reports zero vulnerabilities. A clean locked install preserved the lockfile byte for byte, and all eight new consumer compatibility cases, existing TypeScript contracts, 25 plugin delivery cases and 71 Hermes cases passed. `bash -n scripts/typecheck.sh` passed. The SDK profiles and embedded plugin source hashes remain unchanged, so no Rust rebuild or repeat of unrelated native tests was needed.
- `npm ls --all --json` returned zero with no missing/invalid edges. It reports two optional Sharp/WASM install artifacts as extraneous; a normal npm prune preserves them and the lock hash. These are installer metadata notices rather than unresolved dependency constraints. Their exact paths are retained in the final receipt; no provider runtime relies on adding them to the source lockfile.

# Accepted result

| Boundary | Final result |
| --- | --- |
| glob / minimatch | `14.0.4` / `11.0.5`, including actual npm workspace discovery, ignore-walk and OpenCode glob callers |
| TOML / Zod | `5.0.1` / `4.6.7`; actual prerelease Effect and schema consumers covered |
| Undici | `6.29.1` and `8.12.4` remain in their consumers' respective major lines; loopback fetch/dispatcher checks pass |
| Puppeteer / BiDi | `25.16.0` / `157.0.8090-0`, using Puppeteer's own compatible graph; no forced Zod-major replacement in the old BiDi consumer |
| SDK identity | All direct profiles unchanged; real upstream declarations remain the contract source |
| Maintained checks | `just audit`, Just CI selection and the existing GitHub plugin job audit the entire lockfile; `just typecheck` includes consumer compatibility cases |

The [final receipt](../../.agentx/.tmp/deliveries/plugin-dependency-maintenance/final-receipt.json), [source hashes](../../.agentx/.tmp/deliveries/plugin-dependency-maintenance/source-final.json), [gate log](../../.agentx/.tmp/deliveries/plugin-dependency-maintenance/plugin-gate-second.log) and linked per-step logs retain the exact environment/results. Contributor instructions and the [canonical support report](../../.docs/codex-activation/acceptance.md#development-dependency-follow-up) are updated. The local packaged executable and installed AgentX hashes match their earlier receipts; no global Codex wrapper exists and no native/user service was changed. No task fixture directories or processes remain. The audit result is the current registry result, not a guarantee about future advisories; the maintained audit gate owns future detection. Browser execution, paid inference and a full SDK runtime suite are not implied by these targeted compatibility checks.
