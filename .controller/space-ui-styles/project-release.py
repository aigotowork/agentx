#!/usr/bin/env python3
"""Verify the embedded Space bundle, then switch only the project listener."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import sys
from urllib.parse import urljoin, urlsplit
from urllib.request import ProxyHandler, build_opener

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
ops = runpy.run_path(str(ROOT / ".controller/daemon-performance-audit/launch-rollout.py"))
digest, status, snapshot, storage, replace_binary, restart = (
    ops[name] for name in ("digest", "status", "snapshot", "storage", "replace_binary", "restart")
)
PROJECT_SCOPE, PROJECT_BINARY, PROJECT_PORT = ops["TARGETS"]["project"]
BASELINE = ops["EXPECTED"]
direct = build_opener(ProxyHandler({}))


def web_assets(endpoint, route, frozen=None):
    """Hash the entry assets actually served by the native browser adapter."""
    address = urljoin(endpoint, route)
    with direct.open(address, timeout=4) as response:
        html = response.read().decode()
    paths = re.findall(r'(?:src|href)=["\']([^"\']+\.(?:js|css))["\']', html)
    assets = {}
    for path in paths:
        url = urljoin(address, path)
        assert urlsplit(url).netloc == urlsplit(address).netloc, url
        with direct.open(url, timeout=4) as response:
            asset = urlsplit(url).path
            assets[asset] = hashlib.sha256(response.read()).hexdigest()
        if frozen is not None:
            assert asset.startswith("/space/")
            assert assets[asset] == digest(frozen / asset.removeprefix("/space/")), asset
    assert any(asset.endswith(".js") for asset in assets) and any(asset.endswith(".css") for asset in assets), route
    return assets


def record(file, value):
    staged = file.with_suffix(".tmp")
    staged.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    staged.replace(file)


def verify(candidate, bundle, output):
    sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
    from acceptance import Cases
    from live_fixture import LiveFixture

    fixture = LiveFixture(candidate, output / "embedded-check")
    try:
        cases = Cases(fixture, setup_only=True)
        descriptor = cases.http.get("/api/v1/management")
        assert descriptor["supports_message_history"] is True
        assets = web_assets(cases.http.url, "/space/", bundle)
        check = {"sha256": digest(candidate), "space": assets, "http": ops["http"](cases.http.url)}
        record(output / "embedded-proof.json", check)
    finally:
        fixture.close()
    print(f"Embedded Space matches the verified candidate: {check['sha256']}", flush=True)


def adopt(candidate, bundle, output):
    verified = json.loads((output / "embedded-proof.json").read_text())
    expected = verified["sha256"]
    assert digest(candidate) == expected != BASELINE
    assert PROJECT_BINARY.is_file() and not PROJECT_BINARY.is_symlink() and digest(PROJECT_BINARY) == BASELINE
    before = status(candidate, PROJECT_SCOPE)
    assert before["endpoint"] == f"http://127.0.0.1:{PROJECT_PORT}/"
    assert digest(f"/proc/{before['pid']}/exe") == BASELINE
    rollback = PROJECT_BINARY.with_name("agentx.before-space-appearance-r6")
    assert not rollback.exists(), rollback
    config = PROJECT_SCOPE / "config.toml"
    config_sha = digest(config)
    before_storage = storage(PROJECT_SCOPE)
    before_peers = snapshot(PROJECT_SCOPE)
    siblings = {route: web_assets(before["endpoint"], route) for route in ("/", "/inspector/")}
    env = dict(row.split(b"=", 1) for row in Path(f"/proc/{before['pid']}/environ").read_bytes().split(b"\0") if b"=" in row)
    env[b"AGENTX_DIR"] = os.fsencode(PROJECT_SCOPE)
    assert status(candidate, PROJECT_SCOPE) == before and digest(PROJECT_BINARY) == BASELINE
    os.link(PROJECT_BINARY, rollback)
    assert digest(rollback) == BASELINE
    file = output / "project-adoption.json"
    delivery = {"before": before, "candidate": expected, "rollback": str(rollback), "rollbackSha256": BASELINE}
    record(file, delivery)
    replace_binary(candidate, PROJECT_BINARY, expected)
    result = restart(PROJECT_BINARY, PROJECT_SCOPE, env)
    delivery["restart"] = {key: result[key] for key in ("exit", "seconds")}
    record(file, delivery)
    after = status(candidate, PROJECT_SCOPE)
    delivery["after"] = after
    record(file, delivery)
    assert digest(f"/proc/{after['pid']}/exe") == expected and after["instance"] != before["instance"]
    assert after["generation"] == before["generation"] and after["endpoint"] == before["endpoint"]
    assert digest(config) == config_sha and storage(PROJECT_SCOPE) == before_storage
    assert web_assets(after["endpoint"], "/space/", bundle) == verified["space"]
    assert {route: web_assets(after["endpoint"], route) for route in siblings} == siblings
    process_ids = snapshot(PROJECT_SCOPE)
    missing = [pid for pid, start in before_peers.items() if process_ids.get(pid) != start]
    delivery.update(space=verified["space"], siblingsByteIdentical=True, preservedProcessCount=len(before_peers) - len(missing),
                    missing=missing, http=ops["http"](after["endpoint"]))
    record(file, delivery)
    assert not missing
    print(f"Project :{PROJECT_PORT} serves the verified bundle; kept {len(before_peers)} Agent/PTY process identities.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("verify", "adopt"))
    parser.add_argument("output", type=Path, help="Frozen candidate directory containing agentx and space/")
    args = parser.parse_args()
    output = args.output.resolve(strict=True)
    action = verify if args.mode == "verify" else adopt
    action(output / "agentx", output / "space", output)


if __name__ == "__main__":
    main()
