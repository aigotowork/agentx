---
schema: task
status: completed
owner: nezi
description: Build the currently implemented AgentX capabilities and atomically replace the user's local executable
links: [../AGENTS.md, ../CONTRIBUTING.md, ./agent-labels/implementation.delivery.md, ./message-delivery/result.md, ./convention-roles/implementation.md]
---

# Authorized outcome

The user explicitly requests a release of already implemented capabilities and replacement of `/home/long/.local/bin/agentx`. Nezi owns this bounded packaging/install operation inline, consuming existing implementation and independent acceptance evidence. No new specialist, feature implementation, public registry release, live daemon restart or Agent-host migration is required. The socket proposal remains design-only. Existing service processes, data, accepted candidates and project default executable are preserved.

# Release boundary and resources

Use the existing `just package OUTPUT` recipe with the default optimized Cargo release profile; build.rs compiles current browser sources into the executable. Package to a separate named artifact, record build inputs and checksum, smoke-check the resulting executable, preserve the old installed bytes, then replace the home installation through a same-directory atomic rename. Do not write through an in-use executable inode. Record exact installed path, artifact identity, backup and rollback command. No version bump is inferred from a local install.

Earlier native/source lanes were released by maho/tara to this Controller and subsequently by labels/message-delivery acceptance. Both maho and tara have now exited; the attempted coordination send was rejected for inactive recipients, and raze was informed. No Cargo/rustc/just process was running at release admission. Current repair input drift consists of accepted convention-role changes plus recorded test-only corrections. Read convention-role implementation evidence instead of reverting those changes.

# Execution

Initial installed and project executable SHA256: `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`, version `agentx 0.7.31`. Both are regular executable files. Accepted labels and message-delivery candidates stay immutable. Evidence directory and final result are recorded below as the operation progresses.

Evidence: [release-before](../.artifacts/local-release-20261117/release-before.json), [1,227 actual input hashes](../.artifacts/local-release-20261117/inputs-before.json), [package log](../.artifacts/local-release-20261117/package.log), [build process](../.artifacts/local-release-20261117/package-process.json). The first tool-attached package invocation received SIGTERM/exit143 before producing an artifact; its log is retained as `package-interrupted.log`. The supervised detached invocation runs the identical standard recipe and writes its exact exit result. The existing release profile is opt-level3/LTO/codegen-units1/strip, with no profile overrides. Existing unused-Result warnings in labels progress-handler registration are retained in the build log; this packaging operation does not alter accepted source.

The packaging smoke will run the exact binary from an unrelated directory with a disposable scope and cleared AgentX caller/development environment. It checks native labels capability/filtering, the delivery-status query path, and management/Space/Inspector HTML and same-origin assets. Its own daemon is stopped through an exact executable/serve-argv match and pidfd; existing daemons/Agent hosts are not test resources. This is packaging verification, not a repetition or extension of existing independent functional/browser acceptance.

# Completed local release and installed outcome

The standard optimized build completed successfully in 15m58s. Artifact: `/home/long/works/longwork/agentx/.bin/agentx-release-20261117`; installed atomically at `/home/long/.local/bin/agentx`, mode755, version `agentx 0.7.31`, SHA256 `b00d60e8d06a383bb5e49281cc595d90e9c759a5fe1e4dd60fa18b9c65627e97`, native build `native-build-v1:1d54df28-7f55-4c05-b6fe-037420716c53`. No version bump was made for this local release. [Installed receipt](../.artifacts/local-release-20261117/installed.json) owns exact installation/backup identity; [package result](../.artifacts/local-release-20261117/package-result.json) owns build completion.

[Input comparison](../.artifacts/local-release-20261117/inputs-after.json) found zero changes across the 1,227 recorded files during the build. [Packaging smoke](../.artifacts/local-release-20261117/smoke.json) passed23 checks covering binary-only execution from an unrelated directory, native labels capability and filtered query, the native delivery-status lookup with no resend, and management/Space/Inspector embedded HTML plus JS/CSS. This consumes the existing independent labels/message-delivery/convention-role acceptance; it does not claim new browser interaction or paid-Provider acceptance. Owned fixture daemon PID982063 exited through exact executable/serve-argv/pidfd cleanup. No production source was edited.

The installed command resolves through PATH to `/home/long/.local/bin/agentx` and reports the expected version. Read-only post-install native status verifies the new client can authenticate the existing project daemon PID3400255 and home daemon PID3779326; both retain their original process incarnation, daemon instance and generation. [Post-install metadata](../.artifacts/local-release-20261117/post-install.json) and [native status](../.artifacts/local-release-20261117/live-status-after.json) retain the observed facts. Existing live services and Agent hosts were not restarted or migrated; they continue using their loaded builds. Installing this file does not establish that existing daemons already serve the new labels/report operations. The project `.bin/agentx`, accepted labels candidate and accepted message-delivery candidate remain byte-identical. Socket architecture remains proposed and unimplemented.

Old installed bytes are preserved at `/home/long/.local/bin/agentx.before-release-20261117-041835`, SHA256 `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`. If an explicit rollback is required, use a same-directory temporary file and atomic rename:

```sh
release_rollback_tmp="$(mktemp /home/long/.local/bin/.agentx-rollback.XXXXXX)"
install -m 755 /home/long/.local/bin/agentx.before-release-20261117-041835 "$release_rollback_tmp"
mv -f -- "$release_rollback_tmp" /home/long/.local/bin/agentx
```

This completes the user's requested local packaging and installed-file replacement. Public registry/GitHub publication and live daemon/host adoption were not requested in this operation.
