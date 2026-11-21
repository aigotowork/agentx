"""Owned actual-product fixture helpers; no effect occurs merely by importing.

The caller must supply one fixed packaged artifact and the prepared disposable
root recorded by the acceptance Task. Provider identities come only from real
launch receipts. No process search or shared-scope management is offered.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import socket
import struct
import time


def process_record(pid, role):
    proc = Path(f"/proc/{pid}")
    return {"pid": pid, "role": role, "executable": str((proc / "exe").resolve()),
            "start_ticks": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]}


def exact_process(record):
    try:
        current = process_record(record["pid"], record["role"])
        return all(current[key] == record[key] for key in current)
    except FileNotFoundError:
        return False


class ActualSession:
    """Commands always use this staged binary, selected scope and explicit name."""

    def __init__(self, root, evidence, operator="wb-operator"):
        self.root = Path(root).resolve()
        self.evidence = Path(evidence).resolve()
        if not self.root.name.startswith("agentx-workbench-fixture-") or self.root.parent != Path("/tmp"):
            raise ValueError("requires the exact disposable root recorded by the Task")
        self.project = self.root / "project"
        self.scope = self.root / "scope"
        self.binary = self.root / "bin/agentx"
        self.operator = operator
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith(("AGENTX_", "PASEO_", "CARGO_"))
                    and (not key.startswith("CODEX_") or key == "CODEX_HOME")
                    and key not in {"CI", "GITHUB_ACTIONS"}}
        self.env.update(AGENTX_DIR=str(self.scope), PATH=str(self.binary.parent) + os.pathsep + self.env["PATH"], NO_COLOR="1")

    def stage(self, artifact):
        artifact = Path(artifact).resolve(strict=True)
        self.binary.parent.mkdir()
        self.scope.mkdir(mode=0o700)
        shutil.copyfile(artifact, self.binary)
        self.binary.chmod(0o755)
        record = {"artifact": str(artifact), "staged": str(self.binary),
                  "sha256": hashlib.sha256(self.binary.read_bytes()).hexdigest(),
                  "scope": str(self.scope), "project": str(self.project),
                  "operator": self.operator, "limits": "Explicit disposable scope; inherited participant/dev-root/session claims removed. Authentication values are neither copied nor retained."}
        with (self.evidence / "actual-artifact.json").open("x") as target:
            target.write(json.dumps(record, indent=2) + "\n")
        return record

    def run(self, label, arguments, timeout=45, expected=(0,)):
        record_path = self.evidence / f"{label}-command.json"
        if record_path.exists():
            raise FileExistsError(record_path)
        position = arguments.index("--") if "--" in arguments else len(arguments)
        command = [str(self.binary), *arguments[:position], "--iam", self.operator, *arguments[position:]]
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=self.project, env=self.env, capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired as error:
            def text(value):
                return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
            (self.evidence / f"{label}.stdout").write_text(text(error.stdout))
            (self.evidence / f"{label}.stderr").write_text(text(error.stderr))
            record_path.write_text(json.dumps({"command": command, "cwd": str(self.project), "scope": str(self.scope),
                                               "exit": "timeout", "elapsed_seconds": time.monotonic() - started,
                                               "limit": "Outer launcher stopped; inspect only this scope for any admitted detached participant before cleanup."}, indent=2) + "\n")
            raise
        (self.evidence / f"{label}.stdout").write_text(result.stdout)
        (self.evidence / f"{label}.stderr").write_text(result.stderr)
        record = {"command": command, "cwd": str(self.project), "scope": str(self.scope),
                  "exit": result.returncode, "elapsed_seconds": time.monotonic() - started}
        record_path.write_text(json.dumps(record, indent=2) + "\n")
        if result.returncode not in expected:
            raise AssertionError((label, result.returncode, result.stderr[-3000:]))
        return result

    def work(self, label, operation, options=(), payload=None, expected=(0,)):
        arguments = ["work", operation, "--scope", str(self.scope), "--project", str(self.project), *options]
        if payload is not None:
            arguments.extend(["--payload", json.dumps(payload, ensure_ascii=False)])
        return json.loads(self.run(label, arguments, expected=expected).stdout)

    def launch(self, label, role):
        if role not in {"implementer", "checker"}:
            raise ValueError("only the two outcome roles may be launched")
        prompt = (f"You are the actual AgentX {role} for a user-authorized bounded workbench acceptance fixture. "
                  "Read TASK.md for constraints and your role. Wait for the explicit structured AgentX work assignment/check request before acting or reporting. "
                  "Use your own exact name from AgentX context and the supplied work/assignment/brief/command identities. "
                  "Do not spawn agents, modify criteria/checker probe or any AgentX source/service, or accept results. "
                  "Remain available only for this finite question, baseline-check, return-and-repair workflow.")
        arguments = ["codex", "--go", "--headless", "--title", f"wb-{role}", "--dir", str(self.project), "--json", "--agentx-prompt", prompt,
                     "-m", "gpt-6.1-sol", "-c", 'model_reasoning_effort="max"', "-a", "never", "-s", "workspace-write"]
        return json.loads(self.run(label, arguments, timeout=180).stdout)

    def serve(self, label, port, collaboration=True, diagnostics=False, assets=None):
        """Launch one exact embedded host from a checkout/asset-free cwd."""
        if port < 1024 or port == 18731:
            raise ValueError("requires an explicit disposable non-default port")
        outside = self.root / "outside"
        outside.mkdir(exist_ok=True)
        command = [str(self.binary), "observe-web" if diagnostics else "workbench", "--scope", str(self.scope),
                   "--host", "0.0.0.0", "--port", str(port), "--iam", self.operator]
        if not diagnostics:
            command.extend(["--project", str(self.project)])
            if assets is not None:
                command.extend(["--assets", str(Path(assets).resolve(strict=True))])
            if collaboration:
                command.append("--collaboration")
        with socket.socket() as reservation:
            reservation.bind(("0.0.0.0", port))
        with (self.evidence / f"{label}.stdout").open("x") as output, (self.evidence / f"{label}.stderr").open("x") as error:
            process = subprocess.Popen(command, cwd=outside, env=self.env, stdout=output, stderr=error, start_new_session=True)
        record = {**process_record(process.pid, label), "command": command, "cwd": str(outside), "port": port}
        (self.evidence / f"{label}-process.json").write_text(json.dumps(record, indent=2) + "\n")
        deadline = time.monotonic() + 30
        while not (self.evidence / f"{label}.stdout").read_text().startswith("http://"):
            if process.poll() is not None:
                raise RuntimeError((label, process.returncode, (self.evidence / f"{label}.stderr").read_text()))
            if time.monotonic() > deadline:
                raise TimeoutError(f"owned host readiness: {record}")
            time.sleep(0.05)
        return record

    def daemon_record(self):
        """Read the kernel peer of only this actual scope's native endpoint."""
        digest = hashlib.sha256(str(self.scope).encode()).hexdigest()
        endpoint = f"/tmp/agentx-{os.geteuid()}/{digest}.sock"
        with socket.socket(socket.AF_UNIX) as client:
            client.connect(endpoint)
            pid, uid, _gid = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        record = process_record(pid, "fixture-daemon")
        if uid != os.geteuid() or record["executable"] != str(self.binary):
            raise RuntimeError("selected native endpoint does not belong to staged fixture")
        (self.evidence / "native-daemon-process.json").write_text(json.dumps({**record, "endpoint": endpoint}, indent=2) + "\n")
        return record

    @staticmethod
    def terminate(record):
        """Signal only one already-recorded exact executable/start-time identity."""
        if not exact_process(record):
            return {**record, "already_absent_or_changed": True}
        os.kill(record["pid"], signal.SIGTERM)
        deadline = time.monotonic() + 10
        while exact_process(record) and time.monotonic() < deadline:
            time.sleep(0.05)
        if exact_process(record):
            raise RuntimeError(f"owned process did not stop: {record}")
        return {**record, "stopped": True}
