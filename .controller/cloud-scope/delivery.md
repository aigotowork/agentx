# Cloud scope delivery record

The Linux candidate with SHA-256 `f772771ae8eaccc907e622eb45ac1776bdfa7165786f4d61d52d7bf591f1089e` passed deployment, communication, native HTTPS, external SDK/CLI, interactive TUI and embedded browser acceptance. The [independent acceptance record](acceptance.task.md) links each run and its observations.

| Operator or client action | Delivered contract and evidence |
| --- | --- |
| Manage the deployment from a local machine | One admin bearer grants scope creation, inventory, host and Agent inspection, and token rotation through the CLI, TUI and Web. See the [operator guide](../../.docs/architecture/cloud-scope.md) and [admin OpenAPI](../../packages/agentx-client/cloud-admin.openapi.json). |
| Address an Agent on another host | One shared bearer grants communication within a scope. Short names are unique within the scope, and a qualified address checks the host. See the [scope OpenAPI](../../packages/agentx-client/cloud-scope.openapi.json) and [delivery semantics](communication-wire.md). |
| Send and receive across isolated networks | Two clients on distinct internal Docker networks connected to the same HTTPS origin with DNS hostname and private CA verification. The external SDK and two real Codex 0.171.0 hosts exchanged requests and related replies. Both directions reached confirmed delivery. Only the gateway joined both networks. See the [frozen result](../../.tmp/cloud-scope/network-frozen.json) and [reproduction instructions](../../tests/cloud_scope/README.md#separate-network-namespaces). |
| Recover a local host | After forced termination, the host recovered the original Agent incarnation and Codex thread and delivered a queued message. Explicit listen also confirmed an exact message. See the [recovery result](../../.tmp/cloud-scope/native-client-frozen-evidence/result.json). |

The [native test log](../../.tmp/cloud-scope/final-cloud-tests.log) records all ten cloud-module tests. The [SDK task](sdk.task.md) records package verification. Validation covered Linux and Codex using a scripted Responses model. The Docker run used one physical host with zero published ports. Production rollout, public Internet routing and a full repository test run remain unverified. The installed AgentX and running services were preserved.
