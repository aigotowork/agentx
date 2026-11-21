#!/usr/bin/env python3
"""Run the published SDK on two private Docker networks joined only by the deployment.

Uses already available images and an externally installed SDK. No ports are published.
The cloud image must have /bin/sh and support the candidate's glibc. Client image needs Node 22+.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--candidate", type=Path, required=True)
    p.add_argument("--consumer", type=Path, required=True)
    p.add_argument("--cloud-image", required=True)
    p.add_argument("--node-image", required=True)
    p.add_argument("--codex-bin", type=Path, help="also run native Agents using this Linux Codex ELF and the host Node executable")
    args = p.parse_args()
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    caddy = Path(shutil.which("caddy") or "caddy").resolve(strict=True)
    candidate = args.candidate.resolve(strict=True)
    consumer = args.consumer.resolve(strict=True)
    if not (consumer / "node_modules/@agentx/client/dist/index.js").exists():
        p.error("--consumer requires an installed @agentx/client package")
    env = dict(os.environ, AGENTX_ADMIN_TOKEN=secrets.token_urlsafe(32),
               AGENTX_SCOPE_TOKEN=secrets.token_urlsafe(32))
    prefix = "agentx-cloud-" + uuid.uuid4().hex[:12]
    containers, networks = [], []

    def docker(*argv, script=None, check=True):
        return subprocess.run(["docker", *argv], input=script, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              text=True, env=env, timeout=90, check=check)

    def client(host, body, admin=False):
        preamble = """
import { createCloudAdminClient, createCloudScopeClient } from '@agentx/client';
import assert from 'node:assert/strict';
import { hostname } from 'node:os';
const origin='https://cloud:8443', host=hostname(), name=host==='west'?'luna':'nova';
const scope=createCloudScopeClient({baseUrl:origin+'/scopes/global',token:process.env.AGENTX_SCOPE_TOKEN});
const agent={host,name,incarnation:host+'-instance'};
"""
        flags = ["-e", "AGENTX_SCOPE_TOKEN"]
        if admin:
            flags += ["-e", "AGENTX_ADMIN_TOKEN"]
        res = docker("exec", "-i", *flags, prefix + "-" + host, "node", "--input-type=module", "-",
                     script=preamble + body, check=False)
        if res.returncode:
            detail = res.stderr
            for key in ("AGENTX_SCOPE_TOKEN", "AGENTX_ADMIN_TOKEN"):
                detail = detail.replace(env[key], "[redacted]")
            raise RuntimeError(detail)
        return res.stdout.strip()

    def run_container(name, net, *opts):
        containers.append(name)
        docker("run", "-d", "--pull", "never", "--name", name, "--network", net,
               "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--read-only",
               "--tmpfs", "/tmp:rw,nosuid,nodev,mode=1777", *opts)

    with tempfile.TemporaryDirectory(prefix=prefix) as temp:
        root = Path(temp)
        def openssl(*argv):
            subprocess.run(["openssl", *argv], cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        openssl("req", "-x509", "-nodes", "-newkey", "rsa:2048", "-days", "1", "-subj", "/CN=AgentX network test CA",
                "-addext", "basicConstraints=critical,CA:TRUE", "-keyout", "ca.key", "-out", "ca.pem")
        openssl("req", "-nodes", "-newkey", "rsa:2048", "-subj", "/CN=cloud", "-keyout", "key.pem", "-out", "server.csr")
        (root / "server.ext").write_text("basicConstraints=CA:FALSE\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:cloud\n")
        openssl("x509", "-req", "-in", "server.csr", "-CA", "ca.pem", "-CAkey", "ca.key", "-CAcreateserial", "-days", "1",
                "-extfile", "server.ext", "-out", "cert.pem")
        (root / "ca.key").unlink()
        (root / "Caddyfile").write_text("""{
    admin off
    auto_https disable_redirects
}
https://cloud:8443 {
    tls /config/cert.pem /config/key.pem
    reverse_proxy 127.0.0.1:18755
}
""")
        try:
            for host in ("west", "east"):
                net = prefix + "-" + host
                docker("network", "create", "--internal", net)
                networks.append(net)
            gateway = prefix + "-gateway"
            run_container(gateway, networks[0], "--network-alias", "cloud", "--user", f"{os.getuid()}:{os.getgid()}",
                          "-v", f"{root}:/config:ro", "-v", f"{caddy}:/bin/caddy:ro",
                          "-v", f"{candidate}:/bin/agentx:ro", "-e", "AGENTX_ADMIN_TOKEN",
                          "-e", "XDG_DATA_HOME=/tmp/data", "-e", "XDG_CONFIG_HOME=/tmp/config",
                          "--entrypoint", "/bin/sh", args.cloud_image, "-ec",
                          "agentx daemon cloud --root /tmp/db --public-url https://cloud:8443 & exec caddy run --config /config/Caddyfile")
            docker("network", "connect", "--alias", "cloud", networks[1], gateway)
            for host, net in zip(("west", "east"), networks):
                image, extra = args.node_image, []
                if args.codex_bin:
                    image = args.cloud_image
                    node = Path(shutil.which("node") or "node").resolve(strict=True)
                    extra = ["--user", f"{os.getuid()}:{os.getgid()}",
                             "-v", f"{node}:/usr/local/bin/node:ro",
                             "-v", f"{args.codex_bin.resolve(strict=True)}:/usr/local/bin/codex:ro",
                             "-v", f"{candidate}:/usr/local/bin/agentx:ro",
                             "-v", f"{Path(__file__).resolve().parent}:/scenario:ro",
                             "-e", "HOME=/tmp/home", "-e", "CODEX_HOME=/tmp/home/.codex", "-e", "AGENTX_DIR=/tmp/ax",
                             "-e", "AGENTX_SCOPE_URL=https://cloud:8443/scopes/global", "-e", "SSL_CERT_FILE=/ca.pem",
                             "-e", "CLOUD_ACCEPTANCE_MODEL_KEY=local-model-no-real-credential", "-e", "AGENTX_SCOPE_TOKEN",
                             "-e", "PYTHONDONTWRITEBYTECODE=1", "-e", "PATH=/usr/local/bin:/usr/bin:/bin"]
                    code_mode = args.codex_bin.with_name("codex-code-mode-host")
                    if code_mode.is_file():
                        extra += ["-v", f"{code_mode.resolve()}:/usr/local/bin/codex-code-mode-host:ro"]
                run_container(prefix + "-" + host, net, "--hostname", host, "--workdir", "/sdk",
                              "-v", f"{consumer}:/sdk:ro", "-v", f"{root / 'ca.pem'}:/ca.pem:ro",
                              "-e", "NODE_EXTRA_CA_CERTS=/ca.pem", *extra, "--entrypoint", "sleep", image, "infinity")

            client("west", """
const admin=createCloudAdminClient({baseUrl:origin,token:process.env.AGENTX_ADMIN_TOKEN});
for(let i=0;;i++){try{await admin.overview();break}catch(error){if(i===20)throw error;await new Promise(r=>setTimeout(r,250))}}
await admin.putScope('global',{request_id:'provision',token:process.env.AGENTX_SCOPE_TOKEN});
""", admin=True)
            init = """
const {generation}=await scope.describe(), installation_id=host+'-install';
const input={generation,installation_id};
"""
            for host in ("west", "east"):
                client(host, init + """
await scope.renewHost(host,input);
await scope.registerAgent(name,{...input,host,incarnation:agent.incarnation});
""")
            client("west", init + """
await scope.send({...input,request_id:'request',from:agent,to:'@east/nova',body:'request over HTTPS',intent:'request'});
""")
            client("east", init + """
const claim={...input,agent,claim_id:'east-claim'}, {items:[message]}=await scope.claimInbox(claim);
assert.equal(message.body,'request over HTTPS'); assert.equal(message.from.host,'west');
await scope.confirmInbox({...claim,message_ids:[message.id]});
await scope.send({...input,request_id:'reply',from:agent,to:'@west/luna',body:'reply over HTTPS',reply_to:message.id,intent:'ack'});
""")
            client("west", init + """
const claim={...input,agent,claim_id:'west-claim'}, {items:[message]}=await scope.claimInbox(claim);
assert.equal(message.body,'reply over HTTPS'); assert.equal(message.from.host,'east');
await scope.confirmInbox({...claim,message_ids:[message.id]});
const request=await scope.getOperation('request'),reply=await scope.getOperation('reply');
assert.equal(request.status,'confirmed'); assert.equal(reply.status,'confirmed');
assert.equal(reply.message.reply_to,request.message.id);
""")
            if args.codex_bin:
                def native(host, *argv, check=True):
                    return docker("exec", "-w", "/tmp", prefix + "-" + host, *argv, check=check)

                for host, name, prompt in (("east", "receiver", "cloud-acceptance-receiver-ready"),
                                           ("west", "sender", "cloud-acceptance-sender-start")):
                    # The SDK probe reserved the same real host with a synthetic installation.
                    # Use a second scope to exercise normal native enrollment independently.
                    if host == "east":
                        env["AGENTX_SCOPE_TOKEN"] = secrets.token_urlsafe(32)
                        client("west", """
const admin=createCloudAdminClient({baseUrl:origin,token:process.env.AGENTX_ADMIN_TOKEN});
await admin.putScope('native',{request_id:'native-provision',token:process.env.AGENTX_SCOPE_TOKEN});
""", admin=True)
                    docker("exec", "-d", prefix + "-" + host, "python3", "/scenario/network_model.py")
                    for _ in range(50):
                        if native(host, "test", "-s", "/tmp/model-ids.json", check=False).returncode == 0:
                            break
                        time.sleep(0.15)
                    launched = docker("exec", "-w", "/tmp", "-e", "AGENTX_SCOPE_TOKEN",
                                      "-e", "AGENTX_SCOPE_URL=https://cloud:8443/scopes/native", prefix + "-" + host,
                                      "agentx", "launch", "--tool", "codex", "--name", name, "--headless", "--go",
                                      "--json", "--agentx-prompt", prompt, check=False)
                    if launched.returncode:
                        raise RuntimeError(f"{host} native launch: {launched.stderr.replace(env['AGENTX_SCOPE_TOKEN'], '[redacted]')}")
                request_id = json.loads(native("west", "cat", "/tmp/model-ids.json").stdout)[0]
                reply_id = json.loads(native("east", "cat", "/tmp/model-ids.json").stdout)[1]
                client("west", """
const nativeScope=createCloudScopeClient({baseUrl:origin+'/scopes/native',token:process.env.AGENTX_SCOPE_TOKEN});
const [requestId,replyId] = """ + json.dumps([request_id, reply_id]) + """;
for(let i=0;;i++) {
  try {
    const a=await nativeScope.getOperation(requestId),b=await nativeScope.getOperation(replyId);
    assert.equal(a.status,'confirmed'); assert.equal(b.status,'confirmed');
    assert.equal(a.message.from.host,'west'); assert.equal(b.message.from.host,'east');
    assert.equal(b.message.reply_to,a.message.id); break;
  } catch(error) { if(i===60)throw error; await new Promise(r=>setTimeout(r,500)); }
}
""")
            with candidate.open("rb") as binary:
                build_digest=hashlib.file_digest(binary,"sha256").hexdigest()
            print(json.dumps({"candidate_sha256":build_digest, "model": "scripted Responses" if args.codex_bin else None, "tls": "private CA with DNS hostname verification", "networks": 2,
                              "client_hosts": ["west", "east"], "published_ports": 0,
                              "request": "confirmed", "reply": "confirmed",
                              "native_codex": "confirmed request and related reply" if args.codex_bin else "SDK only"}))
        finally:
            for name in reversed(containers):
                docker("rm", "-f", name, check=False)
            for name in reversed(networks):
                docker("network", "rm", name, check=False)


if __name__ == "__main__":
    main()
