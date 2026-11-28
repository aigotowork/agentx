---
scope: AgentX local and public deployment
status: completed
---

# Local and remote release

The operator requested installation and deployment of the [local read contract convergence](local-contract-retirement.md). Package one optimized Linux executable with current embedded Web assets. Use the same executable for local installs, the cloud image and both public uplink workers. GitHub tags, package registry publication and unrelated scope daemons are outside these deployment targets.

| Target | Adoption and rollback |
| --- | --- |
| Local project and home | Save the previous executable for each path, replace atomically, then restart each selected daemon with the authenticated native command. Verify its original database generation, Management listener, capability flags and candidate digest. |
| Cloud deployment | Upgrade the existing v2 Compose service on port 18758 using the current volume and unchanged credentials. Record previous image, database snapshots and Compose environment; rollback restores the former image on the same volume. Keep the proxy and prior-generation containers untouched. |
| Cloud uplink hosts | Upgrade the dedicated longser and xser01 workers one at a time, preserving tokens, enrollment, runtime directory and independently managed Agent processes. Verify the existing installation identity and public workspace after each restart. |

Require an isolated smoke on these exact executable bytes, container smoke and public preflight before adoption. Pause an affected rollout while unreported cloud operations exist. Do not restore a database snapshot as routine binary rollback. Live verification should remain read-only.

## Published outcome

`just package` produced AgentX 0.7.31 with SHA-256 `46c497a93eb5418683e026ab28b8e4222933beea6e61e6f10e03c6b8fb642014`. Build inputs were unchanged across the recorded checks. The exact binary passed the [real tmux read flow](../../.tmp/agentx-local-remote-release.6pRmjE/tui-info), [Compose smoke](../../.tmp/agentx-local-remote-release.6pRmjE/container-smoke.log) and [isolated dual-host HTTPS acceptance](../../.tmp/agentx-local-remote-release.6pRmjE/cloud-https/result.json).

The local executables `.bin/agentx` and `~/.local/bin/agentx` and their selected daemons on ports `38023` and `38000` now run the candidate. Both database generations and listener bindings remain unchanged. [Local Chrome evidence](../../.tmp/agentx-local-remote-release.6pRmjE/local-browser.json) verifies the live Space bundles against the embedded build. Each executable has a sibling backup ending in `.before-agentx-local-remote-release.6pRmjE`. Restore a backup with a same-directory atomic rename, then use the restored executable's `daemon restart --scope` for the corresponding scope.

The public [personal Space](https://agentx.longalong.cn/namespaces/personal/space/) runs `agentx-cloud:release-46c497a93eb5` (remote Docker image ID `sha256:473cd4f24033ec072aaf31dac08e845c1d069c87eac3f8c1640aac7b0b37393d`) on the existing `agentx-cloud-v2_data` volume and proxy upstream. Both longser and xser01 load exactly the candidate with their original enrollments, installation IDs and local generations. [Public browser evidence](../../.tmp/agentx-local-remote-release.6pRmjE/public-result.json) verified the Namespace's original generation, both host leases, workspace `WSH0zNdctPlX`, and distinct Admin/Namespace permissions. The public verification made no writes.

On `root@192.168.10.3`, restore the previous image on its original volume with `bash /opt/agentx-cloud-v2/before-release-46c497a93eb5/adopt-cloud.sh rollback /opt/agentx-cloud-v2/before-release-46c497a93eb5`. This backup directory also holds private SQLite snapshots. To revert either uplink worker, restore its executable atomically from the sibling `before-release-46c497a93eb5/agentx` and restart only its `agentx-longalong.service` (user unit on longser, system unit on xser01). Neither binary rollback requires replacing credentials or rolling back SQLite.
