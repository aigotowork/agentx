---
status: completed
owner: Cloud Namespace integration
related: [task.md, ../../deploy/README.md, ../../.docs/architecture/cloud-daemon.md]
---

# Public test Namespace cutover

This records the preceding deployment. For the active public service and the current rollback target, see [Cloud v2](cleanup.md#accepted-public-outcome-and-rollback).

Target is `https://agentx.longalong.cn/namespaces/personal/space/`, with longser and xser01 joining the same personal Namespace. The old image or executable, old volume, old reverse proxy upstream and both original enrolled host configurations form the rollback set. Store credentials and backup files in private operator storage; record only artifact hashes, paths, generation and observed host state here.

1. Freeze the candidate and finish the [isolated acceptance](task.md#evidence). Package that exact native artifact in the Compose image and record its digest. At the public server identify the current container or service, exact image or binary, deployment directory/volume, proxy upstream and credential source. Check for outstanding Admin operations before switching. Capture host service units, executable hashes and private copies of existing `cloud.toml`.
2. Start the candidate under a separate Compose project with a new volume and unused loopback port. Use the existing public URL as the advertised URL. Generate a fresh Admin bearer and a distinct Namespace bearer and write them to private files. Create `personal` against the new deployment; check its generation, URL and token separation before moving the HTTPS proxy upstream. Keep the old service and volume available for rollback. Do not copy the old catalog into the new volume.
3. Configure each host with the exact candidate and the same existing local runtime directory. Use `daemon connect --configure-only --namespace-url https://agentx.longalong.cn/namespaces/personal --project-root alias=path --token-file FILE`. On longser set `--project-root default="$HOME/works/longwork"`; on xser01 verify the actual project root and service account before enrollment. Restart only the selected host daemon via its service manager. Preserve independently running Agent hosts, local SQLite and project files. Old remote registration histories remain in the old cloud volume.
4. Check public HTTPS in both access modes. Verify both leases, the auto-discovered longser `agentx` office, exact project/Agent routes, and clean stop of any disposable verification Agent. Confirm retirement with 404 at the old Space and API URLs. Do not publish credentials or request payloads in the delivery evidence.
5. If the cutover does not pass, restore the previous HTTPS upstream and verify the old deployment identity. Restore the saved binaries and scope-local enrollment files on the two hosts, then restart their daemon services. Preserve both volume generations and both local runtime databases for diagnosis.

## Access observation

The live endpoint answered HTTPS with a Caddy redirect to `/deployment`; the new Namespace Space route returned 404 before any cutover. The public SSH host key was not in this machine's trust store. Longser has an independently installed user service for this endpoint; xser01 advertises a system service. Public changes require the verified deployment login and the isolated acceptance above.

The operator supplied the trusted management address `root@192.168.10.3`. It reaches the deployment host. Preflight observed old Compose project `agentx-cloud`, directory `/opt/agentx-cloud`, image `agentx-cloud:build-fc07ba5` (`sha256:27789d2c99a24ab034e5018a12e94d90b188bcf3bbf51cf1fff912c27fc4cd4c`), and volume `agentx-cloud_data`. The old Namespace predecessor had generation `638ec71a-a23b-4dc4-bf6d-ec199c20d688` under deployment `2a07c5ba-fa89-4a32-85f5-a3e040f37718`, two online hosts, two registered Agents, and no outstanding control operations at preflight. The active fragment is `/root/longserver/data/caddy/config/agentx.Caddyfile`, upstream `127.0.0.1:18755`, served by container `longserver_caddy_1`. The new project uses port `18757`.

Both worker services use `KillMode=process`. The longser user service uses `~/.local/share/agentx-longalong` and `~/.local/lib/agentx-longalong/agentx`; the xser01 system service uses `/var/lib/agentx-longalong` and `/opt/agentx-longalong/agentx`. xser01 keeps its existing Git project root. Back up the executable and enrollment in private sibling directories and leave both existing runtime database files in place.


## Accepted public outcome

On 2026-11-24, the [accepted isolated HTTPS candidate](acceptance.md) was installed on both selected workers and served through `agentx-cloud:namespace-96e009cb5ea5` (Docker config digest `sha256:c5fde652a3fe4c37a281d4bb3aff77335f5758aa76f5ca5ed153a65db51d589c`). The Compose project is `agentx-cloud-namespace`, volume `agentx-cloud-namespace_data`, and deployment ID `28e47399-5d33-4b4c-8ac9-7d0379ef964f`. Namespace `personal` is generation `7b217895-dd74-4d99-b10f-5187b77a4159` with freshly issued version 1 credentials. Operator credential sources are root-private `/opt/agentx-cloud-namespace/.env` (Admin) and `/opt/agentx-cloud-namespace/personal.token` (Namespace).

The cutover patched only the AgentX Caddy upstream and saved the new upstream to the boot fragment. The [public Chrome/API result](../../.tmp/cloud-namespace/public-check.wEu1AR/public-result.json) shows longser and xser01 online. The exact [longser / agentx office](https://agentx.longalong.cn/namespaces/personal/space/?host=longser&workspace_id=ws_g1IMogNN8O8DXrbzbvh4TOT3_Oxzn-06yjXb6OTskPI) appeared from the configured root without registration and published ten role entries. Chrome opened it with the Namespace and Admin credentials; only Admin could launch. The Namespace bearer was denied at the Admin API and old Space, Admin and Namespace-predecessor API routes returned 404. The actual launch and exact stop flow was verified in isolated HTTPS with real Codex processes and a deterministic local model.

Rollback is the retained old running container and volume, `/opt/agentx-cloud-namespace/agentx.Caddyfile.before` on the deployment server, `~/.local/lib/agentx-longalong/before-namespace-96e009cb5ea5` on longser, and `/opt/agentx-longalong/before-namespace-96e009cb5ea5` on xser01. These host directories contain the original binaries, enrollment files and service metadata. Use `bash /opt/agentx-cloud-namespace/switch-proxy.sh rollback` on the deployment server, then restore each host's backed-up binary and `cloud.toml` and restart only its selected service. Both original local runtime databases and project directories remain in place.
