---
schema: document
status: active
description: Lightweight file-owned launch assignments and rebuildable status observations.
links: [../src/file_bindings/AGENTS.md, ../.controller/file-task-bindings/task.md]
---

# 文件绑定

```sh
agentx launch --tool codex --bind .controller/example/task.md --bind .controller/example/delivery.md --headless --go --json
agentx bindings --refresh
agentx bindings --project /path/to/project --agent naga --refresh
agentx bindings --file /path/to/another/task.md --refresh
```

`--bind` 可重复，用调用者的当前目录解析路径；Agent 的 `--dir` 不改变绑定路径。文件必须已经存在，可写，包含 YAML frontmatter，且 `schema` 为 `task` 或 `delivery`；文件名不限。一次最多 64 个文件，相同规范路径只写一次。多个 Agent 可绑定同一个文件，一个 Agent 可绑定多个文件。Delivery 中的 Tasks 链接只供观察，不会递归分配或推断交付验收。

本地启动时，如果所用工具支持首条消息，AgentX 会把绑定的 Task/Delivery 文件内容和绝对路径附到首条任务消息中。消息的绑定预览和显式 `--agentx-prompt` 共用 64 KiB；先为文件路径留出空间，优先保留显式 prompt，再放文件正文。超限时标记截断，Agent 可沿路径读完整文件，绑定和启动照常。需要最新内容或更新状态时，仍以源文件为准。Kimi 不支持启动消息，仍可正常绑定文件，可在启动后用 `agentx send` 交代任务。

文件是唯一持久来源。Agent 直接编辑文件的 `status`，平台不会替它推进工作状态。绑定不表示 Agent 正在运行，也不表示 Task/Delivery 已完成。这个能力独立于 `agentx work`，没有绑定数据库、隐藏索引或 watcher。

# Assignment representation

Existing `agent` metadata is retained byte-for-byte for ordinary block frontmatter and exposed as legacy declaration. AgentX adds `agents` entries without overwriting scalar or structured legacy assignees:

```yaml
schema: task
status: planning
agent: previous-name
agents: [{scope: /absolute/data/scope, generation: 9c055bab-example, participant: {canonical_name: naga, incarnation_bits: 4745314284439131168}}]
```

`ParticipantRef` is the existing native exact identity (`canonical_name` plus `created_at.to_bits()`), qualified by canonical data scope and database generation. Reusing a name, moving to a different scope, or replacing a dataset does not transfer the prior assignment. Daemon restart preserves the data generation. Do not round incarnation bits through floating point when consuming JSON; use an integer-preserving decoder. Legacy names and unknown `agents` records remain visible but are never promoted to exact identity by name. Assignments are file declarations, never runtime control grants. The current management API exposes matching documents as `AgentView.file_bindings` using its existing exact Agent reference.

# Read model and refresh

`agentx bindings` returns JSON with `project`, `files` and discovery `diagnostics`. Each file has `path`, `checked_at`, `last_success_at`, `document` and `diagnostic`. `document` contains schema, description, raw status, normalized status, raw legacy/sequence assignment metadata, parsed exact assignments and declared relations. When `diagnostic` is present, `document` is a retained prior success, if any; it is not proof of a successful current read. Missing, unreadable, malformed and unsupported sources do not become work states. A fresh index has no prior success to retain.

The nearest ancestor containing `.controller` or `.git` is the default project. `--project` explicitly selects the project; only that project's `.controller` tree is discovered. Discovery traverses at most 8 directory levels, 8192 directory entries and 2048 Markdown candidates; it never follows file/directory symlinks. Source size is limited to 1 MiB. Other schemas and documents without frontmatter are ignored during discovery; malformed candidates and bounded/read failures are observable. Explicit `--file` may select other filenames/extensions and paths outside `.controller`.

Daemon memory holds at most 8192 observations and 64 project roots. Agent views schedule bounded background discovery of the scope's project and visible Agents' directories, and return the latest completed observation for each exact assignment. A cold view may have no observations until the worker publishes them. Routine views refresh known documents no more than once every five seconds; lifecycle hooks and heartbeats do not scan. An explicit `agentx bindings` read discovers its project or explicit paths, and a `--refresh` also rediscovers the selected project and rereads its known files. An explicit query whose worker is occupied can return unavailable. Inspect timestamps to check freshness. Files bound outside `.controller` need explicit `--file` after daemon restart; no hidden durable index recovers arbitrary external paths.

`--agent NAME` resolves the current native participant and filters by exact identity, scope and generation. It fails if that participant is absent rather than treating a name as historical identity. Unfiltered queries retain declared historical assignments. No runtime status is inferred from file status.

| Normalized status | Representative aliases |
| --- | --- |
| `not_started` | planned, pending, todo, not started, queued, draft, backlog |
| `ongoing` | planning, active, in progress, doing, running, implementing, in review |
| `completed` | completed, done, finished, accepted, delivered, succeeded |
| `paused` | paused, on hold, suspended, deferred |
| `blocked` | blocked, stuck, waiting, waiting on dependency |
| `failed` | failed, failure, error, errored |
| `cancelled` | cancelled, canceled, aborted, abandoned |
| `unknown` | Any unrecognized word, preserved in raw_status |
| `undeclared` | Missing/null status |

Normalization ignores casing and treats spaces, hyphens and underscores alike. `planning` means ongoing work; `planned` means not started. Exact alias definitions live in [document.rs](../src/file_bindings/document.rs).

# Launch effects and recovery

Fresh local terminal/PTY/background launches support binding, including multiple launches with `--count`. Remote launch, stdio (including shared app-server and version probes), resume and fork reject `--bind` explicitly. The same native runtime restriction applies to direct internal callers; remote flags are never silently discarded. Dry-run validates paths and shows resolved bindings without writing assignments or starting a daemon/provider.

Before avoidable hook/process effects, preflight checks all source layouts and writable opens and confirms daemon query support. Runtime reserves the exact participant, writes assignments, refreshes observations, then starts its Provider. File write failure fails that assignment and cancels the reservation. If a later source or process spawn fails, compensation removes only records introduced by that attempt, preserving other Agents and later work/body edits. Existing identical assignments are not removed. A rollback failure names recoverable paths and the exact assignment in the error. A successfully started process can later fail asynchronously; its historical assignment remains, while runtime launch diagnostics describe that separate failure. AgentX does not erase assignments when an Agent stops.

AgentX readers/writers take OS advisory locks on the existing source file; there are no sidecar lock/index files. The ordinary block format preserves all bytes outside the `agents` span, including legacy metadata and body. The owned sequence may be reformatted; unusual flow-root/complex-key frontmatter is re-emitted while preserving semantic fields and exact body. Source comments/formatting within a re-emitted span cannot be guaranteed. External editors must honor advisory locks or avoid concurrent writes. Writes use the locked inode and attempt original-byte restoration on failure; incomplete restoration is explicitly reported. Multi-file writes are compensated rather than transactional. A crash or lost launch reply can leave a recoverable file assignment; inspect the file and native Agent state before retrying. These are honest filesystem boundaries, not a hidden task engine.

# Maintenance and evidence

Start at [module guidance](../src/file_bindings/AGENTS.md). The [implementation Task](../.controller/file-task-bindings/task.md) owns actual executed evidence and remaining limits. Parser/binder/index cases are in [native tests](../src/file_bindings/tests.rs); disposable end-to-end verification is in [CLI cases](../tests/file_task_bindings.py).
