# Space routes independent browser verification

Owner: `space-routes-verify-gila`. Expected behavior: [canonical navigation model](../../.docs/agent-space/routes.md). Execution matrix/status: [verification Task](../../.controller/space-routes/verification.md).

The runner consumes an externally built complete App over isolated loopback HTTP. `?supply=demo` scenarios use the shipped synthetic demo; API interception scenarios use explicitly simulated native-shaped responses. No native runtime, user mutations or live adoption is established. Browser contexts and local server belong to verification and are released after execution. Failed attempts remain in `.artifacts/space-routes-verification/`.

Run after dune's final source release: `node tests/space_routes/browser.mjs <isolated-origin> <absolute-evidence-output>`. Scoped route codec/state checks belong to dune; final external build and this consumer execution belong to verifier.
