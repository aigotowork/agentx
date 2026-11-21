"""Actual native children launched by the private generated-hook Qoder process."""
import json
from pathlib import Path
import sys
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "office_management"))
from office_fixture import OfficeFixture, wait_for


class RelationFixture(OfficeFixture):
    def __init__(self, candidate, evidence):
        super().__init__(candidate, evidence)
        provider = self.root / "bin/qoder"
        source = provider.read_text()
        session_line = '    session = arguments[arguments.index("--resume") + 1] if resumed else str(uuid.uuid4())'
        assert source.count(session_line) == 1, "upstream Provider session recipe changed"
        source = source.replace(session_line, session_line.replace('if resumed else', 'if resumed and "--fork-session" not in arguments else'))
        anchor = "        while running and time.monotonic() < deadline:\n"
        assert source.count(anchor) == 1, "upstream Provider loop changed"
        # Commands execute inside the real admitted Provider's process ancestry
        # and inherited process binding. No caller identity is spoofed by tests.
        control = '''            control = root / "providers" / f"command-{os.environ['AGENTX_INSTANCE_NAME']}.json"
            if control.exists():
                command = json.loads(control.read_text())
                response = control.with_name(command["id"] + ".result.json")
                if not response.exists():
                    assert command["argv"][0] in ["launch", "fork", "resume"]
                    result = subprocess.run([os.environ["AGENTX_MANAGEMENT_CANDIDATE"], *command["argv"]],
                                            capture_output=True, text=True, timeout=45)
                    receipt = {"id": command["id"], "actor": os.environ["AGENTX_INSTANCE_NAME"],
                               "argv": command["argv"], "code": result.returncode,
                               "stdout": result.stdout, "stderr": result.stderr}
                    response.write_text(json.dumps(receipt))
                    record("native-command", **receipt)
'''
        provider.write_text(source.replace(anchor, anchor + control))

    def command(self, actor, argv):
        identity = str(uuid.uuid4())
        control = self.root / "providers" / f"command-{actor}.json"
        temporary = control.with_suffix(".tmp")
        temporary.write_text(json.dumps({"id": identity, "argv": argv}))
        temporary.replace(control)
        response = self.root / "providers" / f"{identity}.result.json"
        result = wait_for("actual Provider native child command", lambda: json.loads(response.read_text()) if response.exists() else None, timeout=55)
        control.unlink()
        (self.evidence / f"native-{identity}.json").write_text(json.dumps(result, indent=2) + "\n")
        assert result["code"] == 0, result
        return result
