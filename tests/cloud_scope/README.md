# Cloud scope integration acceptance

The [delivery contract](../../.controller/cloud-scope/AGENTS.md) defines the supported surfaces. The harness uses an explicitly supplied native candidate, one new deployment root and private environments. Its public boundary is HTTP or the actual consumer interface; it does not import cloud implementation modules or use the installed AgentX.

| Case | Scenario | Independent observation |
| --- | --- | --- |
| Deployment | An admin creates two scopes, pages inventory, retries and rotates one credential, then restarts the listener. | Creation replay yields the same generation. Admin and scope credentials cannot be substituted. The old credential stops working; the new scope credential survives restart without changing generation or affecting its neighbor. Response receipts never disclose credentials. |
| Communication | Two installations register exact identities; one sends and the other acknowledges and replies, including retry/recovery. | The wrong installation or host cannot claim the identity; the retry resolves to the same operation. Inbox, reply and admin inventory agree on the exact identities and do not bleed into the neighbor scope. |
| Cloud Space | The candidate serves the scoped shell and finite assets. An admin sends an operator message to an exact Agent, and both authorized read paths show its mailbox history. | Host/name/incarnation and generation fence the history; credentials cannot be substituted. Replays retain the original message after retirement and restart, and persisted replies are readable without claiming an inbox. |
| Native hosts | Two separate local roots launch named Codex agents and exchange messages. The receiver also answers a Cloud Space operator message using ordinary `agentx send`. | Public inventory and receipt lookup establish exact identities and delivery. Admin history contains the explicitly sent Provider answer. The test uses actual Codex processes with a finite loopback model; raw scope API traffic cannot stand in for this case. |
| Management consumers | An operator manages scopes through the public CLI, TUI and deployed browser route. | Another interface independently observes the mutation and its authoritative credential version. Read-only inventory, actual management writes, package identity and browser assets are recorded separately. |

Run from the repository root with Python 3 on Linux. Choose a new absolute evidence path and an explicit candidate. All listeners bind to loopback; the harness stops only processes it starts.

```sh
python3 tests/cloud_scope/acceptance.py --candidate /tmp/agentx-candidate --suite deployment --evidence /tmp/cloud-deployment
```

Select `space`, `communication`, `native`, `tui`, `cli`, or `browser` to run just that case in its own deployment. `native` requires the real Codex CLI and uses a scripted local Responses model. The hosts have separate AgentX and Codex roots and exact installation identities; both run on the same Linux machine. The default native suite uses loopback HTTP; `--native-https` uses a private CA and first checks that omitting it fails. `tui` requires `tmux`. `cli` requires `--consumer /path/to/external/project` with an installed tarball; use the [external package installer](../contract_service_projections/install_consumer.py). `browser` requires `--chromium /path/to/chrome` and the checkout's Puppeteer dependency. CLI and browser use a real private HTTPS terminator and trust its fixture certificate (Node CA file, Chromium SPKI pin). The HTTPS suites require `openssl`. No public DNS, public CA, remote machine or real model service is required.

Each case writes `result.json`, HTTP or consumer observations, and cleanup status. The fixture removes private runtime files after the run; retained logs redact the test credentials. The [bound task](../../.controller/cloud-scope/acceptance.task.md) links the actual runs and their boundaries.

### Shared Space workbench

Run `python3 tests/cloud_scope/shared_workbench.py --candidate /absolute/path/to/agentx --evidence /tmp/new-space-evidence` on Linux with Chrome, Codex, OpenSSL and the checkout's Puppeteer dependencies. The browser uses the embedded page over private HTTPS, selects an exported role, and exchanges ordinary text with a real Codex process backed by a scripted loopback Responses endpoint. It reloads the exact selection and verifies the persisted reply; the fixture stops the launched instance and its daemon.

### Cloud-connected native daemon

Use [daemon.py](daemon.py) for the daemon-owned HTTPS path. It starts two native daemons via ordinary cold `list`, confirms live management with zero Agents, remotely launches and stops an exact Agent, exercises retries and lease fencing, and verifies both request and related reply with real Codex tool egress denied. The one-machine run uses a private CA and deliberately discards a committed launch response. The physical run uses a separate cloud node with Caddy and a separate east worker; the operator and west worker are on the invoking machine. Chrome submits the east Agent launch through the actual embedded Management app, and an externally installed `agentx-http` submits the exact stop. For determinism the two real Codex processes use a scripted Responses endpoint, with an SSH reverse forwarding connection dedicated to the east worker's model traffic. AgentX traffic on both workers uses direct HTTPS to the cloud node.

```sh
python3 -B tests/cloud_scope/daemon.py --candidate /absolute/path/to/frozen-agentx \
  --evidence /tmp/cloud-daemon-local

python3 -B tests/cloud_scope/daemon.py --candidate /absolute/path/to/frozen-agentx \
  --evidence /tmp/cloud-daemon-physical \
  --physical root@CLOUD CLOUD_PRIVATE_IP root@EAST \
  --physical-codex-bin /absolute/path/to/codex-vendor/bin \
  --chromium /absolute/path/to/chrome \
  --consumer /absolute/path/to/external/package/install
```

The fixture requires Linux with pidfds, Python, OpenSSL, and working Codex, including its matching code-mode host. The physical lane additionally requires `zstd` on all three machines, explicitly authorized SSH access with known host keys, reverse loopback forwarding at east, and Caddy on the cloud node. The operator machine needs Chrome, the checkout's Puppeteer dependency, and an [external package install](../contract_service_projections/install_consumer.py) for the independent CLI. If supplied, `--physical-codex-bin` stages the two Codex executables into east's private root. Both lanes require a new evidence directory; every station has a fresh temporary scope, environment and process identity; physical copies verify the candidate SHA-256 before execution. The fixture stops only its verified processes or SSH sessions, removes the temporary roots, and reports cleanup. See the [acceptance task](../../.controller/cloud-daemon/acceptance.task.md) for actual results and limits.

The physical lane accepts `--east-jump SSH_HOST` for an explicitly authorized SSH jump, and `--control-only` to verify Web launch, exact CLI stop and daemon reconnection without claiming tool messaging. On tested Linux Codex 0.171.0, default egress denial also denies filesystem Unix socket connections; the version's [upstream Linux test](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/linux-sandbox/tests/suite/managed_proxy_unix_sockets_tests.rs#L129) expects individual Unix socket grants to remain denied in managed proxy mode. The full fixture keeps the deny policy in place and exposes this failure instead of changing the sandbox.

Use `python3 tests/cloud_scope/control_consumers.py CANDIDATE INSTALLED_AGENTX_HTTP NEW_EVIDENCE_DIR --chromium /usr/bin/google-chrome` to check the packaged CLI, embedded browser and native TUI against one isolated candidate deployment. A scripted host issues leases, claims and reports; the TUI crosses a private TLS fixture that drops its first launch response. This check requires tmux and OpenSSL and does not exercise a Provider. Omit `--chromium` for the packaged capability read and TUI path alone.

### Separate network namespaces

The optional [network scenario](network.py) runs a real cloud daemon behind Caddy and places each Linux client on a different internal Docker bridge. Only the gateway joins both networks; there are zero published ports. Both clients validate a private TLS certificate for the same DNS origin. The installed SDK registers, sends, claims, confirms and replies over HTTPS. Supply the Linux Codex executable to also launch two native Codex hosts in separate filesystem, PID and network namespaces and drive their shell tools with the scripted Responses model.

```sh
python3 tests/cloud_scope/network.py --candidate /absolute/path/to/agentx \
  --consumer /absolute/path/to/installed-js-project \
  --cloud-image local-linux-image-with-compatible-glibc-and-python \
  --node-image node:24.14.1-bookworm-slim --codex-bin /absolute/path/to/linux-codex-elf
```

Requires locally available images, Caddy, Docker, OpenSSL and Node. The fixture mounts explicitly supplied binaries and test resources read-only, creates temporary credentials, and removes its containers and networks on exit.
