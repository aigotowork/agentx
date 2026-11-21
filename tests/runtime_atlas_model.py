#!/usr/bin/env python3
"""Semantic counterexamples for the explanatory atlas, without product execution."""
import copy
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ATLAS = ROOT / ".docs/architecture/runtime-atlas"
spec = importlib.util.spec_from_file_location("atlas_build", ATLAS / "build.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
model = builder.validate(json.loads((ATLAS / "model.json").read_text()))
scenarios = {scenario["id"]: scenario for scenario in model["scenarios"]}
modes = {mode["id"]: mode for mode in model["modes"]}


def snapshot(scenario_id, index=None):
    scenario = scenarios[scenario_id]
    value = copy.deepcopy(scenario["initial"])
    steps = scenario["steps"] if index is None else scenario["steps"][:index + 1]
    for step in steps:
        for dimension in ["nodes", "facts"]:
            value[dimension].update(step["patch"][dimension])
    return value


# A disconnected controller is not proof of a dead shared backend or finished turn.
disconnected = snapshot("appserver-disconnect")
assert disconnected["nodes"]["bridge"]["tone"] == "stopped"
assert disconnected["nodes"]["codex"]["tone"] != "stopped"
assert disconnected["nodes"]["thread"]["tone"] == "unknown"
assert "待收" in disconnected["facts"]["inbox"]

# Durable acceptance, input issuance, recorded input and business completion stay distinct.
assert "待收" in snapshot("appserver-mail", 0)["facts"]["inbox"]
assert "Issuing" in snapshot("appserver-mail", 3)["facts"]["inbox"]
assert "确认" in snapshot("appserver-mail")["facts"]["inbox"]
assert "做完" in scenarios["appserver-mail"]["steps"][-1]["actions"][-1]

# Closing the initiating terminal must not end a detached execution host.
background = snapshot("headless-close", 1)
assert background["nodes"]["terminal"]["tone"] == "stopped"
assert background["nodes"]["pty"]["tone"] == "running"
assert "terminal-pty" not in modes["headless"]["edgeIds"]
assert "pty-provider" in modes["headless"]["edgeIds"]

# The native proxy is not a module or edge of the data-owning daemon.
assert next(node for node in model["nodes"] if node["id"] == "bridge")["kind"] == "process"
assert not any(edge["from"] == "daemon" and edge["to"] == "codex" for edge in model["edges"])

# A repository source reference is not evidence of code running in an external process.
nodes = {node["id"]: node for node in model["nodes"]}
for external_or_data in ["tmux", "provider", "client", "codex", "thread", "store", "history"]:
    assert nodes[external_or_data]["modules"] == [], external_or_data
assert nodes["terminal"]["kind"] == "process-group"
assert any(module["id"] == "codex-service" for module in nodes["bridge"]["modules"])
assert any(module["id"] == "db-module" for module in nodes["daemon"]["modules"])
assert nodes["sender"]["modules"][0]["path"] == "src/cli/commands/send.rs"

# tmux attach preserves the original process; ordinary resume describes a new host.
assert "同一个" in snapshot("tmux-detach", 1)["facts"]["provider"]
assert "新" in snapshot("foreground-resume")["facts"]["host"]
assert snapshot("workbench-close", 1)["nodes"]["daemon"]["tone"] == "running"

# Backward/initial reconstruction is independent of any previously observed end state.
assert snapshot("appserver-disconnect", -1) == scenarios["appserver-disconnect"]["initial"]
assert snapshot("foreground-launch", -1)["nodes"]["daemon"]["tone"] == "stopped"
assert snapshot("headless-launch", -1)["nodes"]["pty"]["tone"] == "stopped"
assert snapshot("appserver-resume", -1)["nodes"]["bridge"]["tone"] == "stopped"

# Visible execution must not turn a disconnect, local crash or observation into delivery.
disconnect_step = scenarios["appserver-disconnect"]["steps"][2]
assert disconnect_step["beats"][0] == {
    "kind": "disconnect", "edgeId": "bridge-native", "direction": "forward", "label": "关闭本桥 WebSocket"
}
assert all(beat["kind"] != "transfer" for beat in scenarios["appserver-crash"]["steps"][0]["beats"])
assert all(beat["kind"] != "transfer" for beat in scenarios["headless-close"]["steps"][0]["beats"])
record = next(step for step in scenarios["appserver-mail"]["steps"] if step["id"] == "record")
assert record["beats"][0]["direction"] == "reverse"
assert record["beats"][0]["edgeId"] == "bridge-native"
assert scenarios["tmux-detach"]["steps"][0]["beats"][0]["kind"] == "disconnect"

invalid = copy.deepcopy(model)
invalid["scenarios"][0]["steps"][0]["beats"][0]["edgeId"] = "database"
try:
    builder.validate(invalid)
except ValueError as error:
    assert ".beats[0].edgeId" in str(error) and "database" in str(error)
else:
    raise AssertionError("A step must not animate an unrelated communication edge")
print(f"PASS: model references and lifecycle boundaries; {len(modes)} modes, {len(scenarios)} scenarios")
