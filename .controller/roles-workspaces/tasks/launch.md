---
schema: task
status: completed
description: Integrate role/profile launch, frozen instructions, workspace environment and adopted history.
---

Read [team contracts](../AGENTS.md). Own the launch slice. Add role/workspace CLI options and pass native collaboration through reservation. Compose role prompt without losing builtin instructions; ensure fixed prompt persistence. No-role launch receives stable workspace too. Preserve selected role/profile on resume/fork without copying identity. Explicitly reject unsupported remote combinations before effects, or implement target-side resolution. Add focused cases without concurrent Cargo. Report source handoff here.

## Source handoff — soba

Implemented `--role`/`--workspace` host flags, whole-role resolution before profile selection, role-declared profile precedence with explicit profile override, JSON/human workspace preview, stable child `AGENTX_WORKSPACE`, and `LaunchParams.collaboration` forwarding to native reservation. Bound caller metadata overrides mutable inherited workspace environment. Runtime fallback gives non-CLI launch callers stable membership.

Production resume/fork lives in `src/runtime/resume_client.rs`; it carries the adopted collaboration and profile from native `ResumeSnapshot`, clears the endpoint request on forks, and never rereads editable roles. `lifecycle_service.rs` and `lifecycle_resume_store.rs` now extract collaboration from current rows and stopped launch context. Membership owner must include `launch_context` in database stopped snapshots. Controller owns fixed bootstrap role-prompt composition.

Additional owned files authorized by controller: `src/runtime/resume_client.rs`, `src/agents/lifecycle_service.rs`, `src/agents/lifecycle_resume_store.rs` (the actual lifecycle-store child).

Validation: targeted source files parsed/formatted with standalone rustfmt; focused cases authored for CLI selector/native separation, global inheritance versus explicit directory discovery, and stopped adopted prompt recovery. Cargo execution is deferred to controller/independent acceptance; no executable support claimed here.

Limitations: remote fresh launch explicitly rejects role/workspace selectors before dispatch. Resume/fork preserve historical role/workspace and reject those replacement selectors. Shared Codex app-server stdio currently rejects resolved role/workspace launches before provider effects; ordinary global no-role stdio remains supported. Native terminal Codex and other normal provider paths use the standard reservation and frozen bootstrap mechanisms. Shared app-server workspace support would require extending its separate RootLaunch/native-root contract.

### Acceptance repair: fixed instruction additions

Acceptance found that initial `--agentx-system-prompt` text was absent from resumed instruction history. `LaunchCollaboration.instruction_additions` now stores the raw fresh-launch addition with a serde default for existing records. The launcher captures it before provider injection and clears the transient system prompt only on exact equality with that adopted text; resume/fork-generated identity instructions cannot overwrite it. The controller appends the adopted text through fixed bootstrap composition and patches remaining struct literals. The native capability probe includes saved additions, preventing old daemons from silently ignoring this new persisted context. Shared app-server stdio retains its existing native prompt channel and remains outside persisted workspace instruction support.

A focused regression case covers raw capture, duplicate suppression, preservation during new identity generation, and legacy history without additions. Standalone rustfmt and `git diff --check` pass; Cargo remains controller-owned.

Integration executed the launch/profile/bootstrap suite and the new fixed-addition regression. Independent composition, membership and history passed on correction candidate `f76a1a9e`, including deletion of role files, profile edits, resumed fixed additions and fork's own identity. See [integration](integration.md) and [acceptance](acceptance.md) for executable scope and exact artifact hashes. This slice is complete within the documented local native launch path.
