# Web SDK adoption acceptance harness

`cross-origin.browser.mjs` verifies actual Chromium Fetch and CORS/preflight behavior from a temporary `http://localhost:<port>` page to configured API URLs. It does not replace the connected consumer and extension runs in `integration.task.md`.

The JSON config contains a `requests` array. Each item has `name`, absolute `url`, optional `method` (default `GET`), `headers`, `body`, `expectedStatuses` (default `[200]`) and optional `expectedContentType`. Use only exact disposable fixture IDs/generations and requests whose effects are intended for this acceptance scope. For JSON writes, set `Content-Type: application/json`; Chromium will perform the real OPTIONS preflight. Use a distinct API origin from the page origin, including a separate port.

Run from the retained integration lane after fixture and package inputs are ready:

```sh
node /home/long/works/longwork/agentx/tests/web_sdk_adoption/cross-origin.browser.mjs \
  /tmp/agentx-web-sdk-movi/<run>/cross-origin.json \
  /tmp/agentx-web-sdk-movi/<run>/cross-origin-evidence
```

The script records browser version, request/response and preflight headers, status, bounded response text, and page errors. A passing run establishes browser CORS access for only the configured requests. It does not establish application SDK usage, complete UI workflows, iframe parent-origin access, extension host permissions, Firefox support or live-service behavior.

Run the disposable connected probe with the exact candidate and hash:

```sh
python3 tests/web_sdk_adoption/connected-probe.py \
  --candidate /tmp/agentx-web-sdk-movi/candidates/agentx-sdk-adoption \
  --sha256 EXPECTED \
  --evidence /tmp/agentx-web-sdk-movi/evidence/cors-01
```

The runner starts one fixture-owned Management listener, requests its descriptor from a separate localhost page, and submits a deliberately invalid empty launch body to exercise a JSON preflight and a CORS-visible 400 response without creating an Agent. It always closes the fixture. This is bounded Management evidence; listener-specific preflight behavior for Workbench and Observation is covered by their native filters, not inferred from this browser request.

The independent current evidence index is [integration.task.md](../../.controller/web-sdk-adoption/integration.task.md#final-independent-acceptance-handoff). `system-browser.py` composes one disposable native scope, three listeners, a browser bundle imported from the public SDK, cross-origin binary/message/receipt/native-error/marker-integer cases and the actual embedded root pages. Its optional `--resume CLEANED_FIXTURE_RECEIPT` reuses only a verified cleaned owned scope. `adapter-check.mjs OUTPUT.json` checks the actual bundled Management adapter using finite Fetch for caller cancellation and removals-only labels; it is separate from native evidence.

`send-diagnostic.py` pins a candidate/hash, launches one exact finite Agent, and accepts `--driver` for `send-diagnostic.mjs`, `pointer-inspection.mjs`, `iframe-browser.mjs` or `space-recovery.mjs`. These cover diagnostic UI/journal state, public inspection lifecycle plus ordinary pointer sends, full narrow iframe with separately evidenced parent Fetch, and lost-admitted-response UI recovery. They always close owned fixtures and retain each run separately. Use an absolute new evidence directory and explicit lane cwd; no tool modifies production source, installed executables, user services or paid Provider configuration.
