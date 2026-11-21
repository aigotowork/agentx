#!/usr/bin/env python3
"""Independent OBS-01/06 CLI effects against an explicit packaged executable.

No product is run on import. The native-integration Task owns execution and
receipts; this runner never acquires a daemon or reads managed SQLite.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import tempfile
import time


SECRET_MARKER = "obs-fixture-concealed-credential-6a747c"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def tree(path):
    """Record contents/modes, excluding read-induced timestamps."""
    if not path.exists():
        return None
    return {
        str(item.relative_to(path)): {
            "mode": stat.S_IMODE(item.lstat().st_mode),
            "sha256": digest(item) if item.is_file() else None,
            "link": os.readlink(item) if item.is_symlink() else None,
        }
        for item in sorted(path.rglob("*"))
    }


class Fixture:
    """Only this temporary root, selected image and exact scope daemons are owned."""

    def __init__(self, binary, evidence, name):
        self.binary = binary
        self.evidence = evidence
        self.name = name
        self.directory = tempfile.TemporaryDirectory(prefix="agentx-obs-cli-")
        self.root = Path(self.directory.name)
        self.scope = self.root / "scope"
        self.known_scopes = {self.scope}
        self.children = []
        self.daemons = []
        self.attempts = []
        self.cleanup_record = {}
        for leaf in ("home", "cwd", "tmp"):
            (self.root / leaf).mkdir(mode=0o700)
        home = self.root / "home"
        self.environment = {
            "PATH": "/usr/bin:/bin",
            "LANG": "C.UTF-8",
            "CI": "1",
            "NO_COLOR": "1",
            "AGENTX_UPDATE_VERIFY": "1",
            "HOME": str(home),
            "AGENTX_DIR": str(self.scope),
            "TMPDIR": str(self.root / "tmp"),
            "XDG_CONFIG_HOME": str(home / "config"),
            "XDG_CACHE_HOME": str(home / "cache"),
            "XDG_DATA_HOME": str(home / "data"),
            "CODEX_HOME": str(home / "codex"),
            "CLAUDE_CONFIG_DIR": str(home / "claude"),
            "OPENAI_API_KEY": SECRET_MARKER,
        }

    def run(self, arguments):
        index = len(self.attempts)
        prefix = self.evidence / f"{self.name}-{index}"
        command = [str(self.binary), *map(str, arguments)]
        started = time.monotonic()
        stdout_path = prefix.with_suffix(".stdout")
        stderr_path = prefix.with_suffix(".stderr")
        attempt = {"argv": command, "cwd": str(self.root / "cwd")}
        self.attempts.append(attempt)
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            child = subprocess.Popen(
                command,
                env=self.environment,
                cwd=self.root / "cwd",
                stdin=subprocess.DEVNULL,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
            )
            self.children.append(child)
            try:
                code = child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
                attempt["timed_out"] = True
                raise AssertionError(f"CLI did not finish within 8s: {command}")
            finally:
                attempt.update(
                    pid=child.pid,
                    returncode=child.returncode,
                    elapsed_ms=round((time.monotonic() - started) * 1000, 3),
                    stdout=str(stdout_path),
                    stderr=str(stderr_path),
                )
        stdout = stdout_path.read_text()
        stderr = stderr_path.read_text()
        assert SECRET_MARKER not in stdout + stderr, "raw credential escaped diagnostic output"
        return code, stdout, stderr

    def local_failure(self, selected, kinds, stages, *, explicit_inspect=False):
        command = ["observe", "inspect"] if explicit_inspect else ["observe"]
        code, stdout, _ = self.run([*command, "--scope", selected, "--json"])
        assert code == 1, f"expected completed local failure with exit 1, got {code}"
        value = json.loads(stdout)
        assert isinstance(value, dict), "one-shot diagnosis must emit one complete object"
        # Consume ObserveOutput/ObserveResult from observe/output.rs directly;
        # a failure nested only in historical/local evidence is not the result.
        assert value["command"] == "inspect", "default/explicit inspection lost its command identity"
        assert value["requested_scope"] == str(selected), "lexical requested scope was replaced"
        result = value["result"]
        assert isinstance(result, dict) and set(result) == {"LocalFailure"}, "expected the native local-failure result branch"
        failure = result["LocalFailure"]
        assert failure["kind"] in kinds and failure["stage"] in stages, (
            f"unexpected native local failure for {kinds}/{stages}: {failure}"
        )
        # Native LocalFailure.at is optional. Null retains a known stage/kind
        # without supplying a timestamp that can order or age this fact.
        assert "at" in failure, "local failure omitted timestamp availability"
        local = value["local"]
        assert isinstance(local["inputs"], dict) and "Ok" in local["inputs"], "bounded originating input capture was lost"
        attempt = local["attempt"]
        assert isinstance(attempt, dict), "no-server diagnosis lost its originating attempt"
        assert attempt["kind"] == "Probe" and attempt["knowledge"] == "DiscoveryOnly", "passive diagnosis claimed an ordinary call"
        assert attempt["request_id"] is None, "passive diagnosis manufactured a business request"
        assert attempt["last_ready"] is None and attempt["original_epoch"] is None, "local failure claimed daemon evidence"
        assert attempt["failure"] == failure, "attempt and result disagree on the observed local failure"
        return value

    def observe(self, arguments, expected_code=0):
        code, stdout, stderr = self.run(["observe", *arguments, "--scope", self.scope, "--json"])
        assert code == expected_code, f"diagnosis exit {code}, expected {expected_code}: {stderr}"
        return json.loads(stdout)

    def start_daemon(self):
        """Explicit fixture action; observation itself must never start one."""
        prefix = self.evidence / f"{self.name}-daemon-{len(self.daemons)}"
        with prefix.with_suffix(".stderr").open("wb") as stderr:
            child = subprocess.Popen(
                [str(self.binary), "daemon", "serve", "--scope", str(self.scope)],
                env=self.environment,
                cwd=self.root / "cwd",
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=stderr,
                start_new_session=True,
            )
        self.children.append(child)
        record = {"pid": child.pid, "scope": str(self.scope), "stderr": str(prefix.with_suffix(".stderr"))}
        self.daemons.append(record)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            assert child.poll() is None, "explicit fixture daemon exited during initialization"
            code, stdout, _ = self.run(["observe", "--scope", self.scope, "--json"])
            value = json.loads(stdout)
            if code == 0:
                result = value["result"]["Inspection"]
                snapshot = result["response"]["Runtime"]["Ok"]["Snapshot"]
                if "Present" in snapshot["generation"]["value"]:
                    assert result["ready"]["process"]["pid"] == child.pid, "inspection was not bound to the launched kernel peer"
                    record["ready"] = result["ready"]
                    return child, result
            time.sleep(0.025)
        raise AssertionError("explicit fixture daemon never published a generation within 10s")

    def native_file(self, name, value):
        path = self.root / name
        path.write_text(json.dumps(value) + "\n")
        return path

    def event_window(self):
        """Read one frozen native window, retaining every actual CLI receipt."""
        result = self.observe(["events", "--limit", "1"])["result"]["Inspection"]
        page = result["response"]["Events"]["Ok"]["Read"]["Page"]
        records = list(page["records"])
        frozen = page["next"]
        for _ in range(32):
            if page["next"]["after"]["sequence"] == frozen["through"]:
                return page, records
            previous = page["next"]
            path = self.native_file("continuation.json", previous)
            result = self.observe(["events", "--window-file", path, "--limit", "1"])["result"]["Inspection"]
            page = result["response"]["Events"]["Ok"]["Read"]["Page"]
            assert page["next"]["through"] == frozen["through"], "continuation refreshed its frozen upper frontier"
            assert page["next"]["filter"] == frozen["filter"], "continuation changed its immutable filter"
            assert page["next"]["after"]["stream"] == frozen["after"]["stream"], "continuation changed stream"
            assert page["next"]["after"]["sequence"] > previous["after"]["sequence"], "continuation made no scanned progress"
            records.extend(page["records"])
        raise AssertionError("bounded startup window exceeded 32 one-record pages")

    def close(self):
        for child in self.children:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
        # A no-start failure might itself have spawned a daemon. Identify only
        # an exact fixture scope + actual selected image before killing it.
        stopped = []
        for proc in Path("/proc").iterdir():
            if not proc.name.isdecimal():
                continue
            try:
                arguments = (proc / "cmdline").read_bytes().split(b"\0")
                if len(arguments) < 5 or arguments[1:4] != [b"daemon", b"serve", b"--scope"]:
                    continue
                selected = Path(os.fsdecode(arguments[4]))
                if selected not in self.known_scopes or (proc / "exe").resolve() != self.binary:
                    continue
                pid = int(proc.name)
                os.kill(pid, signal.SIGTERM)
                deadline = time.monotonic() + 3
                while proc.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                if proc.exists():
                    os.kill(pid, signal.SIGKILL)
                    deadline = time.monotonic() + 1
                    while proc.exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                state = (proc / "stat").read_text().rsplit(")", 1)[1].split()[0] if proc.exists() else None
                stopped.append({"pid": pid, "scope": str(selected), "alive_after_cleanup": state not in (None, "Z")})
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                continue
        endpoints = []
        for selected in self.known_scopes:
            key = hashlib.sha256(os.fsencode(selected)).hexdigest()
            for suffix in (".sock", ".observe.sock"):
                endpoint = Path(f"/tmp/agentx-{os.geteuid()}") / f"{key}{suffix}"
                if endpoint.exists() and stat.S_ISSOCK(endpoint.lstat().st_mode):
                    endpoint.unlink()
                    endpoints.append(str(endpoint))
        root = str(self.root)
        self.directory.cleanup()
        self.cleanup_record = {
            "root": root,
            "root_removed": not Path(root).exists(),
            "all_direct_children_reaped": all(child.poll() is not None for child in self.children),
            "explicit_daemons": self.daemons,
            "unexpected_daemons_stopped": stopped,
            "exact_leftover_endpoints_removed": endpoints,
            "scope_limit": "Exact disposable scope and selected executable only; no live scope census",
        }


def missing_scope(fixture):
    selected = fixture.scope / "does-not-exist"
    fixture.known_scopes.add(selected)
    fixture.local_failure(selected, {"MissingScope"}, {"ScopeLookup"})
    assert not fixture.scope.exists(), "passive diagnosis created an absent scope ancestor"
    marker = fixture.root / "unexpected-envrc-execution"
    (fixture.root / "cwd" / ".envrc").write_text(f"touch '{marker}'\n")
    for leaf in ([], ["inspect"], ["request"], ["config"], ["config-current"], ["compare"], ["events"]):
        code, stdout, stderr = fixture.run(["observe", *leaf, "--help", "--scope", selected])
        assert code == 0 and "--scope" in stdout and "--json" in stdout, f"parser help lost inherited options: {leaf}: {stderr}"
    for command in (["help", "observe"], ["help", "observe", "events"]):
        code, stdout, stderr = fixture.run(command)
        assert code == 0 and "--scope" in stdout and "--json" in stdout, f"early help route failed: {command}: {stderr}"
    assert not fixture.scope.exists() and not marker.exists(), "help entered product setup or project environment execution"


def existing_scope_without_daemon(fixture):
    fixture.scope.mkdir(mode=0o700)
    before = tree(fixture.scope)
    fixture.local_failure(
        fixture.scope,
        {"EndpointMissing", "ConnectionRefused"},
        {"EndpointLookup", "Connect"},
    )
    assert tree(fixture.scope) == before, "passive diagnosis changed scope files/modes"


def scope_is_file(fixture):
    fixture.scope.write_text("original scope-path bytes\n")
    fixture.scope.chmod(0o600)
    before = digest(fixture.scope)
    fixture.local_failure(fixture.scope, {"NotDirectory"}, {"ScopeLookup"})
    assert digest(fixture.scope) == before, "diagnosis replaced an invalid selected file"


def relative_alias(fixture):
    fixture.scope.mkdir(mode=0o700)
    alias = fixture.root / "alias"
    alias.symlink_to(fixture.scope, target_is_directory=True)
    before = tree(fixture.scope)
    for index, selected in enumerate((fixture.scope, Path("../alias"))):
        value = fixture.local_failure(
            selected,
            {"EndpointMissing", "ConnectionRefused"},
            {"EndpointLookup", "Connect"},
            explicit_inspect=index == 1,
        )
        assert value["scope"]["Existing"]["canonical_scope"] == str(fixture.scope), "local evidence lost canonical alias selection"
    assert tree(fixture.scope) == before, "canonicalizing an alias changed the scope"


def malformed_reference_files(fixture):
    fixture.scope.mkdir(mode=0o700)
    before = tree(fixture.scope)
    invalid = fixture.root / "invalid.json"
    invalid.write_text('{"incomplete":')
    commands = [
        ["request", "--ref-file", invalid],
        ["config", "--ref-file", invalid],
        ["compare", "--left-file", invalid, "--right-file", invalid],
        ["events", "--window-file", invalid],
    ]
    for command in commands:
        code, stdout, stderr = fixture.run(["observe", *command, "--scope", fixture.scope, "--json"])
        assert code == 2, f"invalid native input did not use usage exit 2: {command}"
        assert not stdout.strip(), "input rejection mixed a result into stdout"
        assert stderr.strip(), "input rejection supplied no explanation"
    assert tree(fixture.scope) == before, "invalid input triggered product setup"


def process_restart_and_native_continuation(fixture):
    """OBS-04/05/08: real package, same durable scope, fresh incarnation."""
    fixture.scope.mkdir(mode=0o700)
    first, before = fixture.start_daemon()
    page, records = fixture.event_window()
    old_window = page["next"]
    old_window_file = fixture.native_file("old-window.json", old_window)
    published = [
        record["input"]["payload"]["Configuration"]["SnapshotPublished"]
        for record in records
        if "SnapshotPublished" in record["input"]["payload"].get("Configuration", {})
    ]
    assert published, "clean startup produced no retained configuration reference"
    old_ref = published[0]["snapshot"]
    old_ref_file = fixture.native_file("old-config.json", old_ref)
    historical = fixture.observe(["config", "--ref-file", old_ref_file])["result"]["Inspection"]
    old_snapshot = historical["response"]["Configuration"]["Ok"]["Snapshot"]["result"]["Found"]
    assert old_snapshot["reference"] == old_ref, "retained lookup returned a different snapshot"
    hidden = fixture.observe(["compare", "--left-file", old_ref_file, "--right-file", old_ref_file, "--field", "relay_token"])["result"]["Inspection"]
    hidden_fields = hidden["response"]["Configuration"]["Ok"]["Comparison"]["result"]["Compared"]["fields"]
    assert len(hidden_fields) == 1 and hidden_fields[0]["value"] == {"Unknown": "Withheld"}, "same-ref comparison disclosed hidden equality"
    # Only this disposable fixture source is edited. A passive retained lookup
    # must keep the captured body, while explicit reevaluation is a new fact.
    (fixture.scope / "config.toml").write_text("[preferences]\ntimeout = 41\n")
    reread = fixture.observe(["config", "--ref-file", old_ref_file])["result"]["Inspection"]
    assert reread["response"]["Configuration"]["Ok"]["Snapshot"]["result"]["Found"] == old_snapshot, "file edit rewrote retained evidence"
    current = fixture.observe(["config-current"])["result"]["ConfigurationCurrent"]["Ok"]
    assert current["body"]["purpose"] == "DiagnosticReevaluation", "current settings were presented as historical actual use"
    settings = {row["field"]["Setting"]: row for row in current["body"]["fields"] if "Setting" in row["field"]}
    assert settings["timeout"]["value"] == {"Present": {"Seconds": 41}}, "explicit current read did not consume edited native configuration"
    # Abrupt termination deliberately loses volatile evidence; no mutation or
    # retry is performed. The next explicit fixture daemon owns the same DB.
    first.kill()
    first.wait(timeout=5)
    fixture.daemons[0]["deliberate_abrupt_restart"] = True
    second, after = fixture.start_daemon()
    old_daemon = before["ready"]["daemon"]
    new_daemon = after["ready"]["daemon"]
    assert old_daemon != new_daemon, "restart reused a volatile daemon incarnation"
    assert old_daemon["scope"] == new_daemon["scope"], "restart changed the selected canonical scope"
    assert before["ready"]["generation"]["value"] == after["ready"]["generation"]["value"], "restart changed durable generation value"
    assert before["ready"]["sampled_at"]["clock"] != after["ready"]["sampled_at"]["clock"], "restart reused the process clock origin"
    for command in (["events", "--window-file", old_window_file], ["config", "--ref-file", old_ref_file]):
        result = fixture.observe(command, expected_code=1)["result"]["Inspection"]
        assert result["response"] == {"DaemonChanged": {"expected": old_daemon, "actual": new_daemon}}, "stale operand reached an owner leaf after restart"
    fresh_page, fresh_records = fixture.event_window()
    assert fresh_page["head"]["stream"] != old_window["after"]["stream"], "restart silently reused an old observation stream"
    new_refs = [
        record["input"]["payload"]["Configuration"]["SnapshotPublished"]["snapshot"]
        for record in fresh_records
        if "SnapshotPublished" in record["input"]["payload"].get("Configuration", {})
    ]
    assert new_refs, "successor clean startup produced no retained configuration reference"
    new_ref_file = fixture.native_file("new-config.json", new_refs[0])
    compared = fixture.observe(["compare", "--left-file", old_ref_file, "--right-file", new_ref_file])["result"]["Inspection"]
    assert compared["response"]["Configuration"]["Ok"]["Comparison"]["result"] == {"Incomparable": "DifferentEpoch"}, "compare preempted the native operand-era result"
    assert second.poll() is None, "passive diagnosis stopped the explicit daemon"


CASES = {
    "missing-scope": missing_scope,
    "existing-scope-without-daemon": existing_scope_without_daemon,
    "scope-is-file": scope_is_file,
    "relative-alias": relative_alias,
    "malformed-reference-files": malformed_reference_files,
    "process-restart-and-native-continuation": process_restart_and_native_continuation,
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--case", action="append", choices=CASES)
    args = parser.parse_args()
    if not Path("/proc").is_dir():
        parser.error("this process/cleanup receipt requires Linux /proc")
    binary = args.binary.resolve(strict=True)
    args.evidence.mkdir(parents=True, exist_ok=True)
    results = []
    for name in args.case or CASES:
        fixture = Fixture(binary, args.evidence, name)
        result = {
            "case": name,
            "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "artifact": str(binary),
            "artifact_sha256": digest(binary),
            "environment": {key: value for key, value in fixture.environment.items() if key != "OPENAI_API_KEY"},
            "credential_input": "synthetic disclosure sentinel; no real secret",
        }
        try:
            CASES[name](fixture)
            result["status"] = "passed"
        except Exception as error:
            result.update(status="failed", error=f"{type(error).__name__}: {error}")
        finally:
            try:
                fixture.close()
            except Exception as error:
                result.update(status="failed", cleanup_error=f"{type(error).__name__}: {error}")
            result.update(attempts=fixture.attempts, cleanup=fixture.cleanup_record)
        if fixture.cleanup_record.get("unexpected_daemons_stopped"):
            result.update(status="failed", error="passive invocation started a scope daemon")
        (args.evidence / f"{name}.json").write_text(json.dumps(result, indent=2) + "\n")
        results.append({"case": name, "status": result["status"]})
        print(json.dumps(results[-1]), flush=True)
    (args.evidence / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
    return int(any(result["status"] != "passed" for result in results))


if __name__ == "__main__":
    raise SystemExit(main())
