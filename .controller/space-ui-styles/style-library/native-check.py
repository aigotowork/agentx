"""Run the current Space source against an isolated AgentX daemon and a local Qoder fixture."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.error import URLError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests/live_ui_integration"))
from acceptance import Cases
from live_fixture import LiveFixture, wait_for


def ready(url, process):
    if process.poll() is not None:
        raise RuntimeError("Space dev server exited; see vite.log")
    try:
        with urlopen(url, timeout=2) as response:
            return response.status == 200
    except (URLError, TimeoutError):
        return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--port", type=int, help="frontend port when using Vite")
    parser.add_argument("--embedded", action="store_true", help="open the candidate's embedded /space/ without a separate Vite host")
    parser.add_argument("--expect-domain", action="store_true", help="verify native Workspace pagination and exact original launch receipt")
    args = parser.parse_args()
    if not args.embedded and args.port is None:
        parser.error("--port is required for the Vite host")
    fixture = LiveFixture(args.candidate, args.evidence)
    app = None
    try:
        cases = Cases(fixture, setup_only=True)
        cases.launch()
        url = cases.http.url.rstrip('/') + '/space/' if args.embedded else f"http://127.0.0.1:{args.port}"
        env = {**os.environ, "AGENT_SPACE_PORT": str(args.port), "AGENT_SPACE_API_TARGET": cases.http.url,
               "VITE_AGENT_SPACE_MANAGEMENT_URL": url, "VITE_AGENT_SPACE_EXPECTED_SCOPE": str(fixture.root / "primary/scope")}
        config = fixture.evidence / "browser-config.json"
        config.write_text(json.dumps({"url": url, "workspace": str(cases.paths["child"]), "reference": cases.reference,
                                      "generation": cases.http.generation, "scope": env["VITE_AGENT_SPACE_EXPECTED_SCOPE"], "embedded": args.embedded,
                                      "nativeDomain": args.expect_domain}))
        if args.expect_domain:
            launch = cases.launch_receipt
            launch_result = launch["result"]["Launched"]
            assert launch_result["agent_refs"] == [cases.reference], launch_result
            contract = {"nativeUrl": cases.http.url, "workspace": str(cases.paths["child"]), "empty": str(cases.paths["empty"]),
                        "reference": {**cases.reference, "incarnation": str(cases.reference["incarnation"])},
                        "requestId": launch["request_id"], "scope": env["VITE_AGENT_SPACE_EXPECTED_SCOPE"]}
            domain_config = fixture.evidence / "domain-config.json"
            domain_config.write_text(json.dumps(contract))
            subprocess.run(["node", str(ROOT / ".controller/typescript-domain-sdk/verification/native-domain.mjs"), str(domain_config)], cwd=ROOT, check=True, timeout=25)
        if not args.embedded:
            with (fixture.evidence / "vite.log").open("w") as log:
                app = subprocess.Popen(["node", "web/agent-space/scripts/run.mjs", "dev"], env=env, cwd=ROOT, stdout=log, stderr=log)
            wait_for("isolated frontend listener", lambda: ready(url, app))
        subprocess.run(["node", str(Path(__file__).with_name("native.browser.mjs")), str(config)], cwd=ROOT, check=True, timeout=140)
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=10)
        fixture.close()


if __name__ == "__main__":
    main()
