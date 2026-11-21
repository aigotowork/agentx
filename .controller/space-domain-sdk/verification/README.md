# Space SDK browser verification

This runner exercises the actual static Space build with its public SDK and a controlled browser fixture at the wire boundary. It uses a temporary localhost static server on an ephemeral port. The fixture listener URL is intercepted for the lifetime of each test page. No native service is started, queried, or changed.

From the project root, choose a new output directory and a new evidence directory. Build with the host binding used by [the fixture](fixtures.mjs):

```sh
VITE_AGENT_SPACE_MANAGEMENT_URL=http://127.0.0.1:19741 \
VITE_AGENT_SPACE_EXPECTED_SCOPE=/fixture/agentx-space-sdk/.agentx \
VITE_AGENT_SPACE_PAGE_LIMIT=1 \
node web/agent-space/scripts/run.mjs build /tmp/space-sdk-build

node .controller/space-domain-sdk/verification/space.browser.mjs \
  /tmp/space-sdk-build /tmp/space-sdk-evidence
```

The runner uses Playwright and Google Chrome. Set `AGENTX_PLAYWRIGHT_PACKAGE` to the installed Playwright package path if it does not resolve locally, and `AGENTX_BROWSER_EXECUTABLE` if Chrome is not at `/usr/bin/google-chrome`. The evidence directory contains a report with per-case requests and page errors, plus desktop and narrow screenshots. Browser behavior against the fixture is distinct from [previous native SDK verification](../../typescript-domain-sdk/tasks/integration.task.md#independent-outcome) and from any live deployment.

The first member page contains only a different workspace's member and a continuation. Later pages include an exact current incarnation above JavaScript's safe integer range and a stopped namesake. The fixture also varies the expected scope, generation, and the timing of an exact message response. The actual UI must handle these states without changing listener or target or writing to any endpoint.

## Hands-on native preview

Run `python3 .controller/space-domain-sdk/verification/preview.py /absolute/candidate /absolute/new/evidence --host 192.168.2.178 --port 38031 --hours 12` to keep a real daemon and its embedded Space available for hands-on review after the browser check. Choose an unused preview port and bind explicitly to the local interface you want; `--host` defaults to loopback and `--hours` defaults to 2. The command prints the office URL and records it in `ready.json`. The process owns a new private scope and uses a local echo Provider fixture. SIGINT or SIGTERM closes only this preview and its Agents; live user scopes and binaries are not involved.
