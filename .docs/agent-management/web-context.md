---
schema: document
status: active
owner: web-context-tuli
last_updated: 2026-11-15
links: [../../.controller/web-context/implementation.md, ../proposals/web-context.md, ../../src/management/contract.rs]
---

# Embedded conversation context

## Professional model and decisions

The user captures page text or files in a parent wrapper, reviews pending context in the existing exact Agent conversation, and deliberately sends through AgentX. The resource chain is scope/generation → exact Agent instance → page-owned composer → durable uploads → accepted native message. Query navigation and parent context acceptance never send messages or launch Agents. Provider image ingestion is outside this contract: mailbox delivery contains readable host paths and instructions to inspect them with the Agent's available tools. Remote Agents may not share that filesystem.

| Owner | Responsibility / public boundary |
| --- | --- |
| `management/contract.rs` | Canonical Attachment, StoredAttachment (internal local path), MessageInput attachment IDs, MessageView attachment metadata, register/read IPC. Existing text callers omit attachments. |
| `management/uploads.rs` | Host filesystem persistence, UUID filenames, bounded metadata/bytes, complete write before registration. No filesystem I/O in SQLite dispatch. |
| `management/store.rs` | Generation-keyed upload registration and reference validation; atomic native send plus message→attachment projection in the same write scope. Existing receipt IDs and exact-recipient fencing remain authoritative. |
| `management/web.rs` | Binary upload/preview routes on existing HTTP listener, generation validation before/after host I/O; no extra service. |
| `SpaceController` | Per-exact-ID drafts and uploaded references; attachments survive local selection/view unmounts, never leak to namesakes. |
| `live/context.ts` | Query parsing and parent source/type validation; bridge readiness, correlated results and exact target resolution. |
| `Conversation`, `Attachments`, `History` | Compact pending chips, file picker/drop/paste, deliberate send, durable image thumbnails/file links. Existing layouts/cameras remain owned by App. |

Use existing React, Axum, serde, UUID, native KV persistence and operation journal. Raw bounded binary request bodies avoid base64 inflation and multipart dependency/wiring. Native schema generation remains the single TS wire source. Persistent KV message projections avoid changing every messaging consumer/envelope, while the outer native write transaction keeps references and accepted message atomic. No deletion API or automatic garbage collection in this slice; removed/abandoned drafts can leave unused uploads, deliberately avoiding destruction of sent files. Service capacity is bounded per upload and concurrent HTTP calls, not a total storage quota; host storage administration remains manual.

## HTTP and durable lifetime

`POST /api/v1/attachments?generation=G&name=ENCODED_NAME&media_type=ENCODED_TYPE` with `Content-Type: application/octet-stream` uploads raw bytes. Maximum 10 MiB per file, 8 attachments per message, filename 255 UTF-8 bytes (no control characters) and media type 128 ASCII bytes (letters/digits and `/.-+_`; unsupported parameter syntax returns a clear error). Zero-byte files are valid. Return 201 `Attachment {id,name,media_type,size,url}` only after complete disk persistence and generation-fenced native registration. Metadata keeps the original filename/type/size; the host-generated UUID alone chooses the filename under `<scope>/management-attachments/<generation>/`. Caller paths never choose disk destinations. Registration failure leaves an unreferenced file, never a successful response.

`GET /api/v1/attachments/ID?generation=G` reads registered bytes. Preview only raster image types (`image/png`, `image/jpeg`, `image/gif`, `image/webp`); all other content is served as an octet-stream download. No uploaded HTML/SVG executes in the service origin. `Attachment.url` is the browser URL; the local filesystem path stays internal until normalized mailbox text includes it. Files survive browser closure and daemon restart. Old generation references fail. The service never accepts an unregistered/missing attachment ID for send.

Existing `SubmitMessage.message` adds optional `attachments: string[]`; text can be empty if this list is nonempty. HTTP 202 admits the management operation, not the eventual mailbox effect. Its isolated worker checks registered host bytes before native message submission; missing/changed bytes yield a terminal Failed operation without a message. Native send resolves IDs from generation-owned metadata, appends filenames/types/host paths and inspection guidance, then submits with the unchanged durable operation/request ID. `MessagePage.messages[].attachments` restores cards after refresh, independent of Markdown parsing. Text-only wire callers remain compatible (omitted attachment list defaults to empty; generated clients supply an empty list). Directory send remains text-only to avoid attaching uploads to an implicitly resolved/new endpoint.

## Query and compact layout

`/space/?scope=S&generation=G&agent=NAME&incarnation=BITS&office=PATH&view=chat&layout=sidebar` focuses an existing current exact Agent. For history, replace incarnation with `stopped_event=ID`; exactly one is required. `scope` and `generation` are required whenever agent is supplied. `layout=sidebar|embedded` remains a compatibility hint for the same complete responsive Space. [Canonical navigation](../agent-space/routes.md) now owns ongoing URL/state/history synchronization, resource tabs, exact record/channel selection and primary reader panels. Existing query focus remains compatible; absent legacy office resolves actual membership, while copied modern all-office links explicitly retain `office=`. Unknown/stale/ambiguous targets yield visible status with no namesake fallback; exact GET lookup and existing load-more support asynchronous supply. Navigation restoration and parent context acceptance never upload query text/files, send, handle letters or launch Agents. Native facts and page-owned exact drafts/camera memories retain their existing ownership. This source extension does not imply adoption by the selected live service.

## Parent wrapper protocol

All envelopes use `protocol: "agentx.context.v1"`. Child checks `event.source === window.parent`; iframe hosting has no origin allowlist, account or authorization handshake, per user decision. The parent checks `event.source === iframe.contentWindow`. Use targetOrigin matching the child service origin in parent sends; child responses use the actual event origin (or `*` for an opaque parent). Cross-origin iframe document navigation is permitted while API requests remain same-origin within the child.

Parent sends `{protocol,type:"hello",requestId}` after iframe load and may repeat hello until response; child installs its listener before advertising `{protocol,type:"ready"}`. Hello returns `{protocol,type:"ready",requestId}` even while native data is loading. Parent waits for readiness before sending context. Ready means the bridge is installed, not that an Agent is available.

Parent sends `{protocol,type:"addContext",requestId,target:{scope,generation,agent:{name,incarnation,stopped_event}},context:{title?,text?,url?,files?:[File|Blob]}}`. Text/title/source URL become editable composer text; files use the same native upload path as picker/drop/paste. Blob uses `attachment` as its name; File preserves `name`. URL must be http/https. At least text/title/url or a file is required; metadata and total draft text remain bounded. Add context targets only a loaded, current, sendable exact Agent. History focus is readable but cannot accept context.

Result is `{protocol,type:"result",requestId,status:"added"|"error",error?}`. `added` means uploads completed and context entered that target's pending draft; it does not mean a message was sent. Upload failure leaves the previous draft intact and responds error. Completed request IDs are cached for the page lifetime with a bounded 128-entry cache; duplicate request IDs replay the original result rather than append again. Parents must use unique IDs per new input and never interpret hello/ready as send confirmation. Simultaneous duplicate requests share the same pending result. Closing/reloading loses pending drafts/result cache, while uploaded/sent files persist. No automatic send or new-Agent launch operation exists. Iframe postMessage structured-clones File/Blob. Extension runtime hops may JSON-serialize (notably Chrome); the wrapper must transport bounded bytes or an encoded representation through that hop and reconstruct a File/Blob before iframe postMessage. Do not promise File/Blob survives the extension hop itself. No extension package is needed.

## State and interface

Pending attachment state is uploading → ready or failed; removal changes only draft references. Selection changes never retarget an in-flight upload. Add context commits all supplied text/references to the captured exact draft only after successful uploads and a final scope/identity check. Failed upload/add is visible and never unlocks send with dangling IDs. Native send preserves the receipt journal and unknown-outcome block; the original draft is retained so failures remain reviewable, with unresolved receipts preventing duplicate send. Browser cards use service URLs and native metadata.

The conversation header retains exact identity and contextual actions. Pending files are compact rows/chips immediately above the textarea; images have small thumbnails, filename, size and removal. Common attach action is keyboard reachable; errors appear beside the composer. Narrow sidebar uses the same history/composer layout with compact outer shell. No standing tutorial panel displaces messages. File input, drag/drop and image clipboard events converge on one uploader; notes/model channels never upload or send.

## Finite development and independent cases

| Case | Guarantee and observation |
| --- | --- |
| Text caller and attachment-only send | Old omitted fields accepted; attachment-only normalization gives native nonempty text. |
| Upload boundary | Zero bytes, 10 MiB edge, over-limit, invalid name/type/ID, missing disk file, stale generation produce explicit results; collision/path names never choose paths. |
| Persistence | Upload then native send; message attachment references survive HTTP refresh and daemon reopen; bytes remain host-readable. |
| Idempotency | Same request/body yields same event/reference set; changed body conflicts; failed/unknown receipts never silently resend. |
| Exact isolation | Same name new incarnation/scope/generation cannot receive old query/context/draft; late upload remains bound to original ID. |
| Parent bridge | Cross-origin parent iframe, hello after load, pre-data ready, duplicate request correlation, wrong source, malformed payload, binary structured clone, upload error. |
| User workflow | Text/image/file picker/drop/paste, pending review/remove, deliberate send, durable thumbnail/file card after refresh. |
| Navigation/layout | No mutation on URL/reload/view change; compact 390px and wider reader; existing overview/camera state remains intact. |

Implementation owns focused native/state/type/build checks; @muvu owns `tests/web_context` and combined runtime/browser evidence. [Implementation Task](../../.controller/web-context/implementation.md) records actual checks/candidate; this design alone does not establish executable support.

## Parent harness and isolated startup

Serve [the parent harness](examples/context-parent.html) separately from the candidate, so iframe behavior crosses origins:

```sh
python3 -m http.server 18763 --bind 127.0.0.1 --directory .docs/agent-management/examples
```

Open `http://127.0.0.1:18763/context-parent.html`, enter the candidate URL and exact target JSON from its descriptor/member reads. The harness retries hello after iframe load, accepts readiness only from that iframe/origin, sends structured-cloned selected files, and displays correlated `added`/`error` results. Sending to the Agent still requires the child's send button. Copy scope/generation/incarnation strings without numeric conversion. A Chrome extension wrapper reconstructs File/Blob after JSON runtime transport before using this same iframe boundary.

Candidate startup (replace executable with the Task's actual packaged path):

```sh
candidate=/absolute/path/to/.bin/agentx-web-context-r3
scope=$(mktemp -d /tmp/agentx-web-context.XXXXXX)
"$candidate" daemon web --scope "$scope" --project "$PWD" --host 127.0.0.1 --port 18764
```

Use a disposable Provider fixture from [existing management acceptance](../../tests/agent_management/README.md); the independent acceptance owner supplies its pinned invocation and lifecycle hooks. No paid Provider inference or local-path multimodal ingestion is implied. Stop only the daemon/processes created in this disposable scope after evidence; never restart or replace the user's live scopes.

## Accepted local implementation and adoption

The separate adoption owner @libu deployed the exact accepted r3 to http://192.168.2.178:38023/space/ while retaining the original scope/generation: [rollout receipt](../../.controller/office-details/verification/lounge/rollout-after.json). Its [scene/API/assets read-only smoke](../../.controller/office-details/verification/lounge/live-r3/report.json) and @gabi's [binding live check](../../.controller/file-task-bindings/evidence/space/live-r3/report.json) passed. These separately owned observations establish the selected service outcome; this implementation performed no live action or additional build.

The current integrated local candidate is `.bin/agentx-web-context-r3` (`agentx 0.7.31`), SHA-256 `4f175b5d39ed02852dca6f493a336a5bb1d7f6e596f1e3c826eee7f4adff005a`. It includes the compact receipt positioning repair and the separately owned [binding UI](../../.controller/file-task-bindings/space.md). Packaging uses the established Linux functional settings (`LTO=false`, `CODEGEN_UNITS=256`, `OPT_LEVEL=0`) through command-local Cargo profile overrides; no optimized-performance claim. The candidate embeds fresh `/space/` assets; its 14 asset hashes exactly match the snapshot passed by the binding UI owner. Independent acceptance is complete for the documented Linux local public-HTTP/Chrome/generated-hook fixture scope, including actual pointer recovery at 450px and 360px, exact successor/history/office isolation and affected reader/composer behavior. Original failed attempts and reused r1 native/browser evidence remain preserved in the [acceptance ledger](../../tests/web_context/evidence.md). Source/asset receipts, commands, cleanup and current judgments are owned by [the implementation Task](../../.controller/web-context/implementation.md) and [acceptance Task](../../.controller/web-context/acceptance.md). Live adoption is separately owned and evidenced above.

Compact receipt recovery is part of the embedded layout contract: expanding original-operation receipts keeps the panel inside the child viewport, with long scope/request IDs wrapping locally and the original receipt query reachable by pointer and keyboard. The compact status region owns popup positioning; the summary's horizontal position must not determine the available panel width. The r1 independent browser finding and bounded CSS repair are recorded in the implementation Task; r1 remains historical rather than an accepted live candidate.
