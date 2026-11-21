---
schema: document
status: completed
description: File binding implementation model, decisions and verification cases.
links: [./task.md, ../../.docs/file-task-bindings.md]
---

# Structural design

`src/file_bindings/` owns frontmatter reading, normalized document state, additive exact assignments, guarded writes/rollback and disposable index. Runtime launcher consumes the file binder after native reservation and before spawning; application composition owns the index outside DB handlers. `Queries::FileBindings` and `agentx bindings` expose discovery/explicit refresh, while management Agent projections expose matching exact associations. Neither `work` nor DB schema changes.

The existing scalar `agent` remains untouched. Add `agents` as a sequence of records `{scope, generation, participant: {canonical_name, incarnation_bits}}`; all three identity components must match. Legacy agent values are exposed as declared metadata and are never promoted to exact identity by name. Relationships remain raw YAML projections, without recursive assignment or inferred acceptance.

Paths are resolved against captured caller cwd, independent of child `--dir`. Local ordinary reservation-based launches support bindings; remote, stdio, resume/fork and version probes reject the flag explicitly. Bounded discovery scans only the nearest project `.controller`, with finite depth/file/byte limits, no symlink traversal. Explicit bindings elsewhere are queryable with their paths; restart rediscovery covers project `.controller` only. Lifecycle refresh reads known paths after selected successful lifecycle requests, coalesced to once per second. Explicit refresh always rereads and rediscovers.

File access uses an OS advisory lock on the existing source file. Writers preserve original text except the top-level `agents` span, retain unknown records, and write through the same file descriptor; on write failure attempt original-byte restoration and report restoration failure. This avoids durable sidecar/index files. Locks coordinate AgentX readers/writers; external editors must cooperate or avoid concurrent edits. Multi-file write or subsequent spawn failure removes only assignments newly introduced by that attempt, retaining intervening document edits and other assignees. Failed rollback reports exact recoverable paths. No completion/status changes occur.

# Method and decisions

Engineering/data modeling: source file is authoritative; index entries hold current read result plus optional prior successful snapshot with separate timestamps. Runtime participant state never substitutes for document state. Missing, unreadable, malformed, unsupported and bounded-discovery diagnostics are independent from normalized status. Unknown raw words survive; planning is ongoing and planned is not started.

Technology selection: the repository has no YAML parser. Use maintained serde_yaml_ng for full YAML value reading instead of inventing a line parser; retain source text and narrowly edit the owned top-level assignment span. This adds one small parser dependency, no framework or persistence layer. CBM lists only the unrelated xpm checkout, so current-source reads supply structural evidence rather than indexing the large dirty workspace.

# Verification cases

| Case | Expected invariant | Owning evidence |
| --- | --- | --- |
| Task/Delivery variants, quoted/flow/block YAML, aliases, unknown fields/words | Correct schema/raw state and generous distinct normalization | Reader unit cases |
| Multi-agent, duplicate bind, name reuse and rollback | Exact identities coexist; old scalar/body/unknown metadata survive; rollback removes only new record | Binder unit cases |
| Missing/malformed/unreadable/removed file | Diagnostic distinct from retained last success | Index unit cases |
| Restart + cwd-relative explicit paths + bounded discovery | Associations rebuilt solely from files; symlinks not followed | Index and disposable CLI cases |
| Invalid binding and failed writes/spawn | No successful assignment reported; coherent rollback/recovery | Binder and real launch CLI cases |
| Ordinary launch and explicit query + management | Usable actual consumers, no DB binding state | Disposable native/CLI evidence |

# Friction and disposition

AgentX CLI `ack` requires a received message ID; initial task arrived as ordinary user chat, so coordination uses `inform`. No unrelated guidance or CLI changes are needed. Cargo search requires explicit registry selection in this checkout; dependency selection uses its configured registry and native Cargo metadata.

# Established result

[Task evidence](task.md#executed-evidence) records 10 passing native cases, 12 passing packaged CLI/HTTP scenarios, public help discovery and cleanup against an exact SHA. The implemented caller chain demonstrates launch reservation → file assignment → observed query/management projection, lifecycle refresh and restart rebuild without task/binding DB persistence. Source and functional-candidate support are complete within the documented Linux local scope. Default optimized release and other Provider/OS interoperability remain outside that evidence.
