---
schema: task
status: completed
description: Map current AgentX objects and HTTP capabilities to proposed OpenAPI SDK and CLI panoramas.
---

Owner: rovi. The user requests a global explanation and inventory, not SDK/CLI implementation or runtime changes. Work inline because the survey follows one cohesive contract projection responsibility; delegation would add coordination without independent implementation work.

Outcome: a Chinese source-linked panorama under `.docs/api-panorama/`, reachable from root AGENTS. Explain object relationships, the native-to-OpenAPI-to-SDK/CLI chain, all currently exposed business HTTP operations, the closed observation/workbench operations and proposed SDK methods/CLI grammar. Label existing HTTP, proposed consumer names and native-only capabilities separately.

Method: inventory actual HTTP routing and native declarations together; trace one exact-reference operation through receipt/reconciliation; retain parameter and option mappings and source coverage. Documentation and visual projection only; no service calls, launches, installs, rebuilds or live adoption.

Working findings and friction:

- Native Rust/serde declarations and finite TypeScript derivation exist. Initial scoped searches found no maintained OpenAPI document, operationId map or generated SDK. Confirm repository-wide before the final claim.
- Management, observation and collaboration expose different listeners, authority and wire codecs. A unified SDK can compose them, but a common HTTP base URL or shared integer policy cannot be assumed.
- The codebase-memory service has no index for this project. Use targeted `rg` searches and source reads; do not claim graph completeness.

Completed outcome: [Chinese panorama](../.docs/api-panorama/README.md), [portable interactive view](../.docs/api-panorama/index.html) and [table projection/preview tool](../.docs/api-panorama/render.py), reachable from root AGENTS. The source-linked inventory contains 30 management HTTP combinations (including the historical stop/kill alias), 15 workbench logical operations and 14 observation logical operations: 59 rows over 35 listener-qualified HTTP method/path combinations. The document includes the complete proposed command tree, common/local options, defaults/config mappings, effects/errors and native-only exclusions. No SDK, CLI, OpenAPI description or business runtime was implemented.

Retained decisions: keep Rust/serde and actual HTTP routing authoritative; derive the visual page from the document tables instead of introducing an operational registry. Compose three independently configured clients. Workbench/observation multiplexed union inputs need thin named wrappers to obtain the proposed SDK hierarchy; ordinary OpenAPI generation only provides the underlying read/apply calls. Keep integer codecs, exact references, accepted effects, revision conflicts and unknown outcomes visible.

Additional findings: management AgentRef and collaboration ParticipantRef have different names/representation; ResultSelection also requires brief_revision. Workbench has five Read and nine OperatorCommand variants but no HTTP AgentReport authority. Observation has thirteen BrowserRead variants. Management error strings and several status Strings do not establish exhaustive domain enums. These are documented boundary obligations, not authorization to repair or expose internals.

Verification: resolved 21 local reference targets in the initial complete document and checked the 59-row 30/15/14 partition; checked literal/segment source evidence for management routes and complete native BrowserRead(13), Read(5), OperatorCommand(9) coverage. Checked Markdown table column consistency. Chrome exercised offline filtering, parameter search, selected detail, reload identity, keyboard activation and 390px layout with no external HTTP requests or page errors; desktop and narrow screenshots were inspected for the actual information task. A targeted follow-up verified the narrow diagram pans with legible labels instead of shrinking them, and verified preview source response200/unlisted file404. Preview source links may download raw Rust files according to browser MIME handling; this is not an AgentX API call. Whitespace check passed. No build, Provider call or AgentX service action was run, and these checks do not establish generated SDK or live API behavior.

Preview: `http://192.168.2.178:38085/index.html`, kept running by `python3 -u .docs/api-panorama/render.py --serve --port 38085` (tool session 95904). Manual reload loads regenerated content. The listener exposes only the panorama, its document and explicitly linked public source paths; it does not expose the workspace as a directory or contact AgentX. Earlier temporary preview owned by this task was replaced on the same port; existing user services were preserved. Browser screenshots are `/tmp/rovi-api-panorama-desktop.png` and `/tmp/rovi-api-panorama-mobile-final.png`.

Planning follow-up (2026-11-17): the user requested how to construct the system from Service contracts before deriving consumers. Added [proposed implementation route](../.docs/proposals/contract-service-projections.md), linked from the panorama. Reuse existing Application/domain/worker boundaries; establish named typed consumer operations and a five-operation exact-Agent message/receipt slice, then derive real HTTP/OpenAPI, one-language SDK and a small HTTP CLI before broadening domains. Record trusted-context separation, native/public/wire mapping, nonuniform outcome types, projection ownership and acceptance. This is planning only, not implementation authorization or a completed delivery. Local reference targets and code fences checked; no runtime or code generation changes.
