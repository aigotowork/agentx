# Dedicated AgentX product execution

This opt-in Linux runner owns one fresh Codex app-server thread and one product turn at a time. It connects the same AgentX native mailbox and host authority used by the ordinary runtime to a registered Hypit peer. The [Delivery](../../.controller/product-execution/delivery.delivery.md) records the current verification boundary.

Use a dedicated AgentX daemon and an empty scope with `product-execution.enabled`, and start its Management endpoint on a private loopback address. Configure an adopted workspace and a registered role with an argument-free Codex profile. The role prompt is passed to the dedicated native root. Choose a fresh canonical native home and an administrator-controlled launcher that isolates Codex from the AgentX control socket, peer credentials, runner state, and unrelated projects or credentials. The runner never sets up that environment from development configuration. The [controlled launcher](../../tests/support/product_native_fixture.py) mounts only the selected roots and a fixed model fixture and isolates its network, process and mount namespaces.

Run `node tools/product-execution/runner.mjs /absolute/private/settings.json` with Node 24. The settings are:

| Fields | Meaning |
| --- | --- |
| `agentx`, `scope`, `workspace`, `nativeHome`, `name`, `role` | Candidate executable and canonical dedicated roots, fresh native participant name, exact role selector. |
| `nativeLauncher`, `nativeArgs`, `nativeVersion` | Explicit isolated launch executable and argv, and supported native version. The runner owns the only native stdio stream. |
| `privateOrigin`, `ca`, `cert`, `key` | Exact Hypit HTTPS origin and private references to the trusted runner certificate and key. The native child receives none of these. |
| `toolWire`, `installationDigest` | Hypit-generated tool catalog and pinned SHA-256 manifest. The runner verifies the actual native tool declarations before publishing the witness. |
| `registration`, `journal` | Fresh absolute private output paths for the verified registration and durable runner journal. Use a directory with mode 0700. |

Use the runner's registration JSON with the Hypit Host registration. Send through Hypit's official Management SDK. The runner waits for Hypit's locator for the original message, and allows no tool until the actual native input has been recorded and Hypit accepts that event. A native tool request uses one immutable invocation and expects a structured MCP result plus a matching native tool completion. The runner publishes the final assistant item, terminal and drain in order; later messages cannot start while that turn is still open.

An interrupted or uncertain prior journal is retained. A new process will refuse to use it as a new execution. Keep the original turn, message and tool keys for diagnosis and reconciliation. This candidate retries transport-lost callbacks and tool responses with original keys on the same live native connection; resuming an ambiguous native turn across a process replacement requires separate recovery support. See the Task for verified cases and limits.
