"""Finite native report supply inside a genuinely launched fixture participant.

The parent may select work and a finite action; native CLI resolves authority
from this Provider process's unchanged environment and actual ancestry.
"""

import hashlib
import json
import os
from pathlib import Path
import subprocess


def poll(root, record):
    name = os.environ.get("AGENTX_INSTANCE_NAME")
    directory = root / "actor-jobs" / (name or "unbound")
    if not directory.is_dir():
        return
    for path in sorted(directory.glob("*.request.json")):
        output = path.with_name(path.name.replace(".request.json", ".response.json"))
        if output.exists():
            continue
        job = json.loads(path.read_text())
        scope = Path(os.environ["AGENTX_DIR"])
        common = ["--scope", str(scope), "--project", str(scope.parent / "project")]

        def run(args):
            command = [os.environ["AGENTX_MANAGEMENT_CANDIDATE"], "work", *args]
            result = subprocess.run(command, text=True, capture_output=True, timeout=15)
            return {"command": command, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr}

        try:
            shown = run(["show", *common, "--work", job["work"]])
            assert shown["exit"] == 0, shown
            work = json.loads(shown["stdout"])["Read"]["Detail"]["work"]
            assignment = next(item for item in work["assignments"] if item["participant"]["canonical_name"] == name)
            source = scope.parent / "project" / "capability.txt"
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            assert source.read_bytes() == b"independent fixture capability\n"
            if job["action"] == "result":
                report = {"Result": {"summary": "Concrete finite artifact", "source": {"reference": str(source), "revision": digest},
                                     "artifacts": [{"Path": {"path": "capability.txt", "fingerprint": digest}}]}}
            elif job["action"] == "check":
                request = work["check_requests"][-1]
                result = next(item for item in work["results"] if item["id"] == request["result_id"])
                assert result["input"]["source"]["revision"] == digest
                report = {"Check": {"request_id": request["id"], "result_id": request["result_id"],
                                    "observed_fingerprint": digest, "verdict": "Passed",
                                    "criteria": [{"criterion_id": item["id"], "verdict": "Passed", "evidence": "Read exact bytes and SHA256 " + digest}
                                                 for item in work["briefs"][-1]["input"]["criteria"]],
                                    "evidence": ["Independent checker process read concrete artifact bytes"]}}
            else:
                raise ValueError("Unknown finite actor action")
            command_id = job["command_id"]
            applied = run(["report", *common, "--work", work["reference"]["id"], "--generation", work["reference"]["generation"],
                           "--revision", str(work["revision"]), "--assignment", assignment["id"],
                           "--brief", str(assignment["dispatched_brief"]), "--command-id", command_id,
                           "--payload", json.dumps(report)])
            observed = run(["command", *common, "--generation", work["reference"]["generation"], "--command-id", command_id])
            response = {"name": name, "pid": os.getpid(), "shown": shown, "report": report, "applied": applied, "observed": observed}
        except Exception as error:
            response = {"name": name, "pid": os.getpid(), "failure": repr(error)}
        temporary = output.with_suffix(".tmp")
        temporary.write_text(json.dumps(response, indent=2) + "\n")
        temporary.replace(output)
        record("native-work-report", response=response)
