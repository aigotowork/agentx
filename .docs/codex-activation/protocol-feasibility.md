---
schema: document
status: draft
owner: gina
last_updated: 2026-11-06
description: Independent feasibility evidence for native Codex activation and protocol routing.
links: [../../.controller/codex-activation-design/protocol-check.md, ./README.md]
---

# Native protocol feasibility investigation

This report owns feasibility evidence only; zomi owns the activation design and contracts. No production source, user configuration, installed tools, credentials or outer client is changed.

## Investigation model and cases

The boundary is `outer client ↔ agentx bridge ↔ one native Codex process`. The bridge must own native framing and both request directions while coordinating client turns with internal wake requests on an explicitly bound thread. Native process identity is not native thread identity.

| Question | Discriminating evidence | Current basis |
| --- | --- | --- |
| Can idle wake execute on the same thread? | Real native process against loopback-only fixed SSE fixture; two sequential turns | Observed: sequential wake succeeds; same thread id is retained |
| Does busy `turn/start` reject, steer or queue? | Hold a fixture response open and issue another start | Observed: active `turn/start` appends input and returns the existing turn; stale `turn/steer` rejects |
| Do hooks run for app-server turns and preserve wake text? | Trusted disposable hooks log input and return fixture context | Observed: `UserPromptSubmit`, `PostToolUse` and `Stop` run in tested paths |
| Can another native thread share the process? | Second `thread/start` and hook session identities | Observed: yes; each thread has its own native session/transcript; agentx binding remains parent-owned |
| Is a side channel simpler? | Installed CLI transport declarations, optional isolated Unix clients | Help declares stdio, Unix, WebSocket, daemon/proxy; no side channel proven for inherited stdio |
| Can protocol requests be routed transparently? | Native schema of client/server requests, IDs and embedded request IDs | Schema confirms bidirectional requests and string/int64 IDs |

Official `https://developers.openai.com/codex/app-server/` retrieval returned HTTP 403 both directly and through `proxy curl`; no official page body was available. Installed declarations and native experiments will be identified separately from documentation claims.

## Installed artifact and source basis

The installed wrapper reports `codex-cli 0.167.0`. It resolves to the native artifact `/home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex`, SHA-256 `cdcb45aaa01d0f14e298985e09465aebccd47f64ecd298aea44814af7b62bbda`. The wrapper package is `@openai/codex` version `0.167.0`; the platform package is `@openai/codex-linux-x64`.

The local schema bundle supplied by the parent is `/tmp/zomi-codex-wake-schema`. Its v1 bundle SHA-256 is `021462a9bacd117f4cd40c256a4888619d059cec6a65b795198d521ba660e554`, and its v2 bundle SHA-256 is `f2576ca5d46be4cfb159906264fbc173efcfcc917ea8fdc38323770e7b657b5a` (the latter is recorded as generated evidence; the exact command below is the reproducible source). The native executable's own `app-server generate-json-schema --experimental` output was generated under `/tmp/gina-codex-experimental-xe_lu_bj/schema`, with the combined schema SHA-256 `486c7c8daa5dc7f3ab2a1fb56de5316804caf7a4210fda1dc28d3f0f4e90cfe1`.

Repository evidence is [src/README.md](../../src/README.md#native-codex-stdio), [src/runtime/codex_stdio.rs](../../src/runtime/codex_stdio.rs), and [src/integrations/hooks/codex.rs](../../src/integrations/hooks/codex.rs). The current stdio host inherits the three standard streams, forwards signals and process status, and never parses protocol bytes. Its hook delivery acknowledges after hook output is written and flushed; that is delivery evidence, not model or business completion. The hook implementation filters internal Codex threads by `agent_id`, transcript/session identity and memory workspace, so a native process can contain multiple thread identities even while agentx currently binds one process to one instance.

The exact discovery commands were:

```text
codex --version
codex app-server --help
codex app-server daemon --help
codex app-server proxy --help
sha256sum /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex /tmp/zomi-codex-wake-schema/codex_app_server_protocol.schemas.json /tmp/zomi-codex-wake-schema/codex_app_server_protocol.v2.schemas.json
codex app-server generate-json-schema --experimental --out /tmp/gina-codex-experimental-xe_lu_bj/schema
```

The schema declares `RequestId` as string or int64 integer. `turn/start` requires `threadId` and `input`; its response contains an in-progress `turn`. `turn/steer` requires `threadId` and `expectedTurnId`, and the native error explicitly rejects a stale expected ID. `ThreadStatus` distinguishes `idle`, `active` with `activeFlags`, `systemError` and `notLoaded`. `TurnStartParams` documents that a start request on an active turn adds input and that `parentTurnId`/`rootTurnId` are attribution fields. The server request union includes command/file/permission approvals, user input, MCP elicitation, dynamic tool calls, token refresh and attestation. These are declarations, not a promise that every method is stable across versions.

## Isolated native lifecycle evidence

The focused probe is [probe_native_protocol.py](./.tools/probe_native_protocol.py). It starts a disposable native process with a fresh `CODEX_HOME` and project under `/tmp`, clears inherited environment down to a minimal `PATH`, uses a loopback-only fixed SSE Responses fixture on an OS-assigned port, and never uses a real model endpoint or credential. The fixture emits fixed assistant text and, for one case, a fixed dynamic tool request; it has no shell or network side effect. The probe does not modify the installed executable, project configuration or managed daemon.

The reproducible commands and retained result directories are:

```text
python3 .docs/codex-activation/.tools/probe_native_protocol.py --codex /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex
# completed, /tmp/gina-native-protocol-j4y26u1s, 7 fixture model requests
python3 .docs/codex-activation/.tools/probe_native_protocol.py --codex /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex --stable-api
# completed, /tmp/gina-native-protocol-s72g5pvz, 5 fixture model requests
python3 .docs/codex-activation/.tools/probe_native_protocol.py --codex /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex --transport unix
# completed, /tmp/gina-native-protocol-g7vd1ru2, 8 fixture model requests
python3 .docs/codex-activation/.tools/probe_native_protocol.py --codex /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex --stable-api --focused-hooks
# completed, /tmp/gina-native-protocol-suqo8t0e, two fixture model requests; direct client id plus idle and active hook rejection cases
python3 .docs/codex-activation/.tools/probe_native_protocol.py --codex /home/long/.nvm/versions/node/v24.14.1/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex --stable-api --valid-steer
# completed, /tmp/gina-native-protocol-rexp47k0, two fixture model requests; one bounded valid active-turn steer
```

The full stdio run established that an agentx-shaped marker is delivered literally to `UserPromptSubmit`, that a normal assistant turn runs `Stop`, and that a dynamic tool turn emits the server request `item/tool/call` with native numeric request id `0`, then runs `PostToolUse` after the client response. The model fixture saw hook-provided context in each model request. The observed event ordering for a normal turn was request response containing an in-progress turn, `thread/status/changed(active)`, `turn/started`, hook events and model/tool events, `hook/completed(Stop)`, `thread/status/changed(idle)`, and `turn/completed`. The request response can precede `turn/started`; a bridge must forward both without treating the response as completion.

When the fixture held a model request open, a second `turn/start` on the same thread returned the currently active turn id and appended its `UserPromptSubmit` input. The final turn contained the merged active input and completed once. A `turn/steer` with the wrong expected id returned `-32600` with the exact active id. Therefore the bridge must serialize or deliberately admit active input; a “check idle, then start” race is unsafe, and `turn/start` is not an atomic wake admission primitive.

The same run started a second thread in the same native process. It received a different native session/thread id, ran its own `SessionStart` and `UserPromptSubmit`, and got its own transcript. That is evidence that process binding cannot identify a unique native thread. It does not prove that agentx's current one-process binding is safe for arbitrary outer `thread/start`, `thread/resume` or `thread/fork`; explicit per-thread ownership and fencing remain required.

The focused hook run used a command hook that returns exit code 2 for the literal `EMPTY_QUEUE_CASE`. On an idle thread, the native response still reported an in-progress turn, the hook completed with status `blocked`, no model request occurred, and native emitted `turn/completed` with an empty item list and no `Stop`. On a busy thread, the `turn/start` response returned the existing active turn id, but that successful response did not prove user input admission: the blocked marker produced no `item/started` or `item/completed` `userMessage`, the active turn completed with an empty item list, and no `Stop` ran. The successful `turn/start` response means that native accepted the request at the RPC/turn layer, while the hook outcome determines whether a user-message item reaches the active turn. A hook can suppress a model call, but it cannot safely express “leave the native turn untouched” once native has admitted the request. This is why hooks are useful for delivery context and observability, not as the bridge's busy/idle admission lock.

The same focused run sent a direct real message with `clientUserMessageId: "client-direct-1"`. The request response arrived before `turn/started`; after hook completion, native emitted `item/started` and `item/completed` for a `userMessage` whose `clientId` was exactly `client-direct-1`, then emitted the assistant item and `turn/completed`. The blocked marker had no corresponding user-message item. The bridge can use `clientUserMessageId` for correlation when forwarding direct client input, but it must not treat that correlation or the initial response as proof that a hook allowed the input into the model context.

A separate bounded run exercised one valid active-turn steer only. It started `HOLD_CASE`, waited until the fixture model request made the native turn active, then sent `turn/steer` with the returned `expectedTurnId`, `clientUserMessageId: "client-steer-1"` and `STEER_CASE`. Native returned `{ "turnId": <same active id> }`; after the fixture was released, it emitted `item/started` and `item/completed` for a `userMessage` with `clientId: "client-steer-1"`, then the assistant item. `turn/completed` retained the original active turn id, and the run ended normally. This supports the parent contract's active path: use `turn/steer` with the exact current active turn id and correlate the durable batch only when the matching completed `userMessage` item is observed. The probe used a fixed fixture and did not test a real provider, interruption during steer, duplicate client ids or concurrent external input; those remain integration checks.

The probe's hook observations are scoped to Codex 0.167.0 with the hooks feature enabled. They establish that these hook names are meaningful for app-server turns in this installed build: `UserPromptSubmit` runs before model work, `PostToolUse` runs after the tested dynamic tool response, and `Stop` runs after normal completion. They do not establish the exact payload contract for every hook, internal thread, approval path or future version. The parent must continue treating delivery acknowledgment as hook-output flush rather than completion.

## Native control-channel comparison

| Option | Installed support | Same process/thread potential | Suitability for agentx independence |
| --- | --- | --- | --- |
| `app-server --stdio` | Supported; one process owns inherited stdio | Yes, but only one byte stream writer/reader | Requires an agentx-owned bridge to multiplex outer traffic and internal turns |
| `app-server --listen unix://PATH` | Supported by help and direct probe; endpoint is HTTP WebSocket over AF_UNIX | Yes; two WebSocket clients in the probe shared one process and read/started turns on one thread | Best native transport if the outer client can connect to an agentx proxy or if agentx launches the server with this endpoint; it is not a side channel into an already-stdio process |
| `app-server --listen ws://IP:PORT` | Declared by installed help; not exercised because the Unix path was sufficient | Potentially yes, subject to auth and endpoint ownership | Similar proxy architecture with greater network exposure; requires explicit auth and lifecycle policy |
| `app-server proxy --sock PATH` | Supported as a daemon control-socket client | Connects to a managed daemon, not to a generic stdio child | Not simpler for a bound arbitrary process; it speaks raw JSON to the daemon control socket and timed out against the WebSocket listener in this build |
| `app-server daemon start` plus `proxy` | Supported managed local service | Shared daemon backend, but separate lifecycle/configuration and user service | A separate production backend would change ownership and can attach to user sessions; excluded from this investigation and should not be assumed equivalent to the bound native process |

The direct Unix test started `codex app-server --listen unix://<temporary socket>` with the same disposable home. A raw WebSocket handshake returned `101 Switching Protocols`, and a WebSocket text frame carrying `initialize` returned a native JSON-RPC response. Two WebSocket clients then shared the thread: the control client got its `turn/start` response, while the first client received `thread/status/changed`, `turn/started`, and `turn/completed`; subscription is connection-scoped. A second `thread/read` from the control client saw the same persisted thread. The native server accepted only one `--listen` option, so stdio and Unix/WebSocket cannot be enabled concurrently by repeating the flag.

The daemon proxy comparison used `codex app-server proxy --sock` against a temporary native Unix listener and a fake Unix socket. The fake socket showed that `proxy` forwards raw JSON bytes. Against the native WebSocket listener it timed out during initialization because that listener first requires the HTTP/WebSocket upgrade. This is evidence of two distinct endpoint protocols, not a claim that the daemon's private socket format is stable. No managed daemon was started, stopped, inspected or modified.

`CODEX_HOME`, hook declarations/trust and model provider configuration are loaded by the backend process. A second client connection inherits the backend's selected home, hooks and provider; it does not establish an independent hook environment. An agentx-owned bridge therefore needs to launch or be handed the native backend with the intended environment and then expose a separate client-facing transport. A bridge cannot add a side channel to a process that was already launched as plain stdio without owning the original stream.

## Bridge routing and arbitration consequences

An agentx-owned bridge is technically feasible as the sole native stream owner. It can forward an outer client's `initialize`, notifications, requests, responses and server requests, while issuing internal `turn/start` requests on the same native connection. The probe proves the native side can run sequential and concurrent-client turns on one process, but it does not prove a transparent production bridge. The bridge must own framing, request correlation, lifecycle fan-out, thread binding, shutdown and recovery.

Both directions contain requests. Native server requests such as approvals, user input, MCP elicitation and dynamic tool calls require a response; the bridge must expose them to the outer client or apply an explicitly owned policy and send the native response. The tested `item/tool/call` used native id `0`, while the outer probe also used string ids and a large int64 id. A bridge cannot assume numeric ids, UUIDs, or a single request direction. It should maintain a per-connection mapping keyed by `(origin, downstream id)` and remap ids whenever forwarding could collide; it must preserve the exact id type and value at each boundary. `serverRequest/resolved` also carries the native request id and thread id, so remapping must cover resolved notifications and late responses.

The outer response to `turn/start` is not the end of a turn, and `turn/completed` can arrive after an input was accepted by an already active turn. A safe arbitration state needs at least native thread id, active turn id, owner/source of accepted input, pending internal wake, external request sequence, and a generation fence. Before an internal wake, the bridge should observe an authoritative idle state and reserve the thread generation; it must then reject, defer or explicitly steer competing external input according to the parent contract. It must never infer completion from hook output, `turn/start` response, `turn/started`, or an idle-looking local timer. If the native connection is lost after request write, the result is unknown and queued agentx messages must remain pending for recovery or explicit reconciliation.

The probe shows a native process can host multiple threads, and the app-server API can resume a running thread by id. The bridge must therefore bind an agentx participant to `(native process generation, native thread id)`, fence stale callbacks after rebind, and keep outer client thread identities separate from agentx participant identities. A new outer `thread/start` must not silently steal an existing agentx binding. The bridge should reject unsupported cross-thread wake requests before writing native traffic.

## Recommendation and limits

Use an agentx-owned protocol adapter as the supported design direction. Prefer a native Unix/WebSocket listener when agentx can launch the Codex backend in that mode and the outer client can connect to the bridge; otherwise make agentx the sole reader/writer of inherited stdio and implement the same adapter there. Treat `daemon/proxy` as a separate service topology, not as a generic control channel for an existing stdio process. The adapter must be version-pinned or capability-gated, preserve arbitrary JSON-RPC ids and server requests, maintain per-thread/process fences, serialize wake admission against external input, and surface native lifecycle events to the outer client.

The recommendation is supported for protocol and lifecycle feasibility, not production readiness. The probe used a real native Codex binary and real native hook execution, but its model endpoint was a fixed local SSE fixture. It did not exercise a paid model, authentication, real approvals, MCP servers, cancellation races with a real provider, process crash recovery, client reconnect, large fragmented WebSocket frames, or every schema method. Official documentation remained unavailable due to HTTP 403. The native protocol is explicitly experimental in the installed CLI help and can change with upgrades.

Before implementation acceptance, the parent design should require focused checks for: outer and internal request-id collision with all string/int64 forms; every server-request category and late response; external input racing internal wake at idle and busy boundaries; duplicate wake suppression; `turn/steer` expected-id fencing; thread start/resume/fork isolation; process restart and unknown write outcomes; hook output and delivery cursor semantics; Unix/WebSocket authentication and socket ownership; client-visible event ordering and reconnect; and native version/schema capability negotiation. A full unmodified outer app-server client spawning an agentx subagent and receiving an autonomous wake remains an integration check, not established by this report.
