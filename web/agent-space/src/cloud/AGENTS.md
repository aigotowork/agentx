---
scope: Namespace Space Web
status: active
owner: Space Web
related: [../../../../.controller/cloud-namespace/task.md, ../../../../.ui-craft/surfaces/space.md, ../../README.md]
---

The [Namespace integration Task](../../../../.controller/cloud-namespace/task.md) owns cloud identity and URLs; [direct cloud reads](../../../../.controller/direct-calls/cloud.task.md) owns generation-bound paging and operation compatibility. Admin sign-in calls the exact Namespace resource directly; member sign-in calls its descriptor. The pinned session supplies generation to every directory page. A stale page or revoked bearer clears old rows and disables reads and writes until sign-in; receipts and drafts retain their existing exact identity. The [older Cloud Space delivery](../../../../.controller/cloud-space/shared/web.task.md) is historical evidence for the retired host-as-office model.

| Boundary | Contract |
| --- | --- |
| [CloudSpace.tsx](CloudSpace.tsx) | Mount only the login gate before admission; preserve the existing audience and token-duration choices. |
| [model.ts](model.ts), [route.ts](route.ts) | Use same-origin public SDKs and compose separate host filters, project offices and exact Agent links. No local paths, host root aliases or display names serve as project IDs. |
| [workbench.ts](workbench.ts), [changes.ts](changes.ts) | Page hosts, projects and members independently of exact readers. One visible change watcher invalidates these reads; on failure it retries the same endpoint. Offline catalogue entries remain usable for reads. |
| [OfficeNavigation.tsx](../OfficeNavigation.tsx), [AgentCommands.tsx](../interactions/AgentCommands.tsx) | Group project links by host; keep the same compact selector on narrow screens. Use the selected project's roles, and use project root aliases only to register a project. Verify installation and exact registration before stop. |
| [history.ts](history.ts), [operations.ts](operations.ts) | Keep older and live pages independent. Render authenticated durable messages and real cloud receipts; retain only control identities in session storage. Never automatically resubmit a write. |
| [transcript.ts](transcript.ts) | Read the exact sanitized Codex projection, using a bounded latest page, independent older cursor and incremental watermark. Availability comes from the response; HTTP failures surface as read errors. Report host facts separately from the AgentX mailbox. |

Namespace bearer reads only. Admin can send text to a current exact registration and use the original receipt for control recovery. The accepted message ID is not a delivery acknowledgement. Retired registrations remain linkable when their exact histories remain available; never navigate to their replacements. Agent location is supplied launch membership; cwd is a separate fact. Relevant native code paths remain separate from this adapter.

Build the public SDK, then run `node web/agent-space/build.mjs <new-absolute-output>` and `node web/agent-space/tests/cloud-space.browser.mjs <output> [evidence-directory]`. The browser uses built assets and disposable loopback routes.

For host-backed working-directory selection and its real browser/native evidence, see the [directory launch task](../../../../.controller/cloud-space/shared/directory.task.md).
