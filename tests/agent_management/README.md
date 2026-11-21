# Public management acceptance

Canonical semantics: [design](../../.docs/agent-management/design.md), [native DTOs](../../src/management/contract.rs), [Task](../../.controller/agent-management/tasks/acceptance.md). Execution status and findings: [evidence](../../.controller/agent-management/evidence.md).

Run `python3 tests/agent_management/acceptance.py --candidate /absolute/separately-named-agentx --evidence /absolute/new-result-directory`. Add `--browser` for actual Chromium interaction with the embedded resources. Set `AGENTX_PLAYWRIGHT_PACKAGE` and `AGENTX_BROWSER_EXECUTABLE` if the existing local defaults are unavailable. No package installation or build occurs.

Use `--browser-only` with a new evidence directory when the unchanged candidate has already passed native cases and only the browser driver changed. Preserve the original run and link both manifests; do not repeat passed native cases without a relevant change.

The runner creates scopes only inside its new `/tmp/agentx-management-*` root. It uses native `[management]` configuration, direct owned `daemon serve`, real HTTP, and candidate-generated hooks. A private PATH substitutes `qoder`; no paid tool executable is reachable. Environment files preserve that PATH through native launch/resume. Local Provider artifacts are fixtures, not evidence of actual Qoder/model interoperability. Assertions never consult managed SQLite.

| Case | State / action | Observable guarantee |
| --- | --- | --- |
| M01 | Empty native-config scope starts | Descriptor is daemon-owned, dynamic loopback port, matching project/generation; embedded page and assets are available |
| M02 | Launch one local Provider | Durable receipt reaches success; exact live reference, session, status, transcript and native hooks become visible |
| M03 | Submit/replay/send/reply | Identical ID/body admits one effect; changed body conflicts; native hook receives request and Provider replies to the same event |
| M04 | Concurrent duplicate launch / lost response | One Provider starts; original request ID retrieves completed work without re-execution |
| M05 | Stop → history → resume → new message | Live ref retires; exact history remains readable/resumable; successor ref differs and rejects old-reference work; stopped and successor message windows exclude each other's events |
| M06 | Scope/generation fences | Separate scope has distinct generation and no primary Agents/receipts; reset rejects old-generation reads/writes |
| M07 | Invalid/oversize requests | Malformed/duplicate/unknown JSON, fields, paths, query bounds, IDs, exact-reference shape, cross-origin writes, body/text bounds reject without effects |
| M08 | Pagination / response shape | Bounded pages/cursors use string identities and expose no PID, launch env, worker ticket or generic native proxy |
| M09 | Daemon restart | Completed receipt is retained under unchanged generation; exact stopped history stays readable |
| B01 | Browser core lifecycle | Launch, select, send, receive reply, transcript, stop/history/resume use actual HTTP and finite visible states |
| B02 | Poll/reload continuity | Selection/draft survive polling; admitted write survives lost reply and reload without duplicate effect |
| B03 | Narrow screen | Same selected object/actions are reachable without document overflow; retained DOM, screenshot and API receipts explain the scene |
| M10 | Cleanup | Only tracked exact fixture processes terminate; owned descriptor is removed; no paid model, live service or managed DB client is involved |
| M11 | CLI enable/autostart → host/port/project reconfigure → replay | `daemon web` enables a disabled disposable scope and autostarts its daemon; reconfigure returns a ready new loopback endpoint without replacing daemon/generation; original launch replay survives removed directory and changed project, changed body still conflicts |

HTTP expectations derive from the intended public contract, independently of server implementation. The suite records first failure and unexecuted later cases; it does not rewrite product semantics to make a run pass. Candidate or harness corrections get new result directories. Browser cases are bound to the actual Web handoff before execution; authored coverage is distinct from executed acceptance.
