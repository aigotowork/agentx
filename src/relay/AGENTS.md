# Relay module

Read the [Relay model](../../.docs/architecture/relay.md), [native operations](service.rs) and [active Task](../../.controller/domain-daemon/tasks/relay.md) before changing behavior. The Task owns actual implementation and independent evidence status.

[Join tokens](token.rs) only admit the format emitted by the current encoder; a decoded token always contains the 32-byte Relay key. The public broker list is append-only because the token carries the broker index.

`service_store.rs`, `pull.rs`, `backfill.rs` and push projection helpers are daemon-only persistence adapters. MQTT and actual runtime/file/process effects belong to the worker after application dispatch returns. `persistence.rs` is the typed client binding; it never opens managed SQLite or falls back after IPC failure. No daemon handler may call it or any other same-daemon client.

Local queue/peer display and targeting use native Snapshot through `snapshot_client`. Supply the originating own-device UUID; peers retain every short mapping, and status presentation owns per-device grouping/online sorting. The snapshot handler returns database failures rather than an empty successful projection. Health remains the canonical native Status projection.

Client-bound push hints use `trigger_push_client` or `notify_relay_daemon_client`; every WorkerPort query must retain the supplied Client. Startup retry and worker spawn are host-only policy shared with the ambient wrappers. A notification is not evidence of publication or remote acceptance.

Reset/disable callers use `worker::stop_relay_worker_client(&Client)` for scoped Relay worker shutdown. It validates the retained Client before host effects, takes the scoped spawn lock, requires the worker's PID/incarnation sidecar, rechecks identity and exact pidfile/sidecar before TERM/KILL, confirms positive exit or known PID reuse, then issues transactional `WorkerStopped` and runtime cleanup through that same Client. Unknown ownership, changed/partial successor records, unconfirmed exit or cleanup replies preserve evidence and return an error; the data daemon is never stopped. Integration filters are `relay::worker::stop::tests::`, `relay::service_store::tests::worker_`, and the reset client's `cli::commands::reset::client_tests::`.

Keep the existing peer envelope and source-owned native contracts authoritative. Preserve AEAD topic/group binding, nonce replay checks, per-peer reset/timestamp/cursor rules, bounded payload repair and exact RPC correlation. A lost acknowledgment is an unknown effect, never permission to replay a mutation. Control intent lookup binds generation/group/source/request ID and compares normalized action/input inside the record. Host completion must match generation, exact token and observed host identity.

Worker plans require a nonempty observed process incarnation; equal unknown identities never authorize Begin/Complete or re-offer. Retire abandoned plans only on positive exit or two known differing incarnations, inspecting processes before a write scope. Receive and Backfill return frozen allowed wake ports; only the worker performs Relay-owned TCP fan-out. Keep domain-owned subscription/event post-commit policy with its existing owner.

Use actual runtime, participant, event, transcript and scoped configuration ports; never import CLI command helpers into Relay. Domain mutations stay with their domain handler. Only actual host effects leave dispatch. Meaningful cases are authored here or handed to integration; the independent integration owner alone executes Cargo/Node/build checks. Do not use live peers, credentials or default databases.
# Launch profile selection

`launch-profile` and `launch-preview` are finite peer capabilities for unresolved profile/tool selectors and argv overrides. `control_host` resolves them in the target scope, clears target `AGENTX_PROFILE` selection, and calls the shared runtime resolver. Never compose local defaults before forwarding or recompute profile values per child. Preview returns configuration without launching a Provider; it still requires the existing Relay transport/host path. [Usage](../../.docs/launch-profiles.md) and [Delivery](../../.controller/launch-profiles/delivery.md) distinguish parser/target tests from actual cross-device evidence.

