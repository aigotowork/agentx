# Independent live UI acceptance

Read the [finite case map and execution boundary](README.md), assigned [Task](../../.controller/live-ui-integration/tasks/acceptance.md) and [evidence](../../.controller/live-ui-integration/evidence.md). This directory owns independent composed candidate tests only, never product repairs or build output.

`acceptance.py` pins the explicit candidate SHA-256 before creating a disposable fixture; `live_fixture.py` reuses the management/office fixtures and adds a private Codex executable; `codex_provider.py` consumes candidate-generated terminal hooks and offers only a finite local trust-discovery responder; `browser.mjs` drives both actual embedded routes and retains DOM/state/bitmap/trace/native receipts. No real Codex app-server or paid model runs. Provider transcript/control files belong to this disposable fixture and are distinct from managed daemon storage.

Do not execute before integration freezes and hands over the candidate path/hash. No managed DB clients, privileged internal collection routes, mock successful API results, installations, native builds, user-service changes, delegation or source reset. Ports 18967, 18971 and 38023 are protected. Preserve every failed run separately and report evidence gaps explicitly.
