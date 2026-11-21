"""Structural contracts for finite, authored Flow Atlas traces (schema version 1).

This module owns generic validation and normalization; repository provenance and
runtime claims belong to consumers. See README.md for the data model and limits.
"""
from __future__ import annotations

import math
import re
from typing import Any

TONES = frozenset({"neutral", "running", "busy", "waiting", "stopped", "unknown", "stored"})
NODE_KINDS = frozenset({"process", "process-group", "store", "logical", "actor", "service"})
ID_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]*")
RESERVED_IDS = frozenset({"__proto__", "constructor", "prototype"})


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def _json_copy(value: Any, path: str = "$") -> Any:
    """Copy a JSON tree without coercing keys, tuples or non-finite numbers."""
    if value is None or type(value) in {str, bool, int}:
        return value
    if type(value) is float:
        if not math.isfinite(value):
            _fail(path, "expected a finite JSON number")
        return value
    if type(value) is list:
        return [_json_copy(item, f"{path}[{index}]") for index, item in enumerate(value)]
    if type(value) is dict:
        result = {}
        for key, item in value.items():
            if not isinstance(key, str):
                _fail(path, "object keys must be strings")
            result[key] = _json_copy(item, f"{path}.{key}")
        return result
    _fail(path, "expected a JSON value")


def _object(value: Any, path: str) -> dict:
    if not isinstance(value, dict):
        _fail(path, "expected an object")
    return value


def _array(value: Any, path: str, *, nonempty: bool = False) -> list:
    if not isinstance(value, list) or (nonempty and not value):
        _fail(path, "expected a nonempty array" if nonempty else "expected an array")
    return value


def _text(value: Any, path: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str) or (nonempty and not value.strip()):
        _fail(path, "expected a nonempty string" if nonempty else "expected a string")
    return value


def _optional_text(item: dict, keys: tuple[str, ...], path: str) -> None:
    for key in keys:
        _text(item.setdefault(key, ""), f"{path}.{key}", nonempty=False)


def _identifier(value: Any, path: str) -> str:
    _text(value, path)
    if value in RESERVED_IDS or not ID_PATTERN.fullmatch(value):
        _fail(path, "id must match [A-Za-z0-9][A-Za-z0-9_.:-]* and cannot be __proto__, constructor or prototype")
    return value


def _strings(value: Any, path: str, *, nonempty: bool = False) -> list[str]:
    values = _array(value, path, nonempty=nonempty)
    for index, item in enumerate(values):
        _text(item, f"{path}[{index}]")
    return values


def _keyed(value: Any, path: str, *, nonempty: bool = False) -> dict[str, dict]:
    result = {}
    for index, item in enumerate(_array(value, path, nonempty=nonempty)):
        item_path = f"{path}[{index}]"
        _object(item, item_path)
        key = _identifier(item.get("id"), f"{item_path}.id")
        if key in result:
            _fail(item_path, f"duplicate id {key!r}")
        result[key] = item
    return result


def _references(value: Any, known: Any, path: str, *, nonempty: bool = False) -> list[str]:
    values = _strings(value, path, nonempty=nonempty)
    seen = set()
    for key in values:
        if key in seen:
            _fail(path, f"duplicate reference {key!r}")
        if key not in known:
            _fail(path, f"unknown or hidden reference {key!r}")
        seen.add(key)
    return values


def _enum(value: Any, choices: Any, path: str) -> str:
    _text(value, path)
    if value not in choices:
        _fail(path, f"expected one of {', '.join(sorted(choices))}; got {value!r}")
    return value


def _number(value: Any, path: str, *, positive: bool = False) -> None:
    if type(value) not in {int, float}:
        _fail(path, "expected a finite number")
    try:
        finite = math.isfinite(value)
    except OverflowError:
        finite = False
    if not finite or (positive and value <= 0):
        _fail(path, "expected a positive finite number" if positive else "expected a finite number")


def _point(value: Any, path: str) -> None:
    _array(value, path)
    if len(value) != 2:
        _fail(path, "expected [x,y]")
    for index, coordinate in enumerate(value):
        _number(coordinate, f"{path}[{index}]")


def _state(value: Any, mode: dict, facts: dict, path: str, *, complete: bool) -> None:
    _object(value, path)
    if set(value) - {"nodes", "facts"}:
        _fail(path, "state supports only nodes and facts")
    if not complete:
        value.setdefault("nodes", {})
        value.setdefault("facts", {})
    node_states = _object(value.get("nodes"), f"{path}.nodes")
    fact_values = _object(value.get("facts"), f"{path}.facts")
    visible = set(mode["nodeIds"])
    for key, status in node_states.items():
        status_path = f"{path}.nodes.{key}"
        if key not in visible:
            _fail(status_path, "unknown or hidden node")
        _object(status, status_path)
        if not status or set(status) - {"label", "tone"}:
            _fail(status_path, "node status must contain label and/or tone only")
        if complete and set(status) != {"label", "tone"}:
            _fail(status_path, "initial node status needs label and tone")
        if "label" in status:
            _text(status["label"], f"{status_path}.label")
        if "tone" in status:
            _enum(status["tone"], TONES, f"{status_path}.tone")
    for key, fact in fact_values.items():
        if key not in facts:
            _fail(f"{path}.facts.{key}", "undeclared fact")
        _text(fact, f"{path}.facts.{key}", nonempty=False)
    if complete and set(node_states) != visible:
        _fail(f"{path}.nodes", "initial state must name every visible node")
    if complete and set(fact_values) != set(facts):
        _fail(f"{path}.facts", "initial state must name every declared fact")


def _layout(value: Any, mode: dict, path: str) -> None:
    _object(value, path)
    for dimension in ("width", "height"):
        _number(value.get(dimension), f"{path}.{dimension}", positive=True)
    positions = _object(value.get("nodes"), f"{path}.nodes")
    for key, position in positions.items():
        position_path = f"{path}.nodes.{key}"
        if key not in mode["nodeIds"]:
            _fail(position_path, "unknown or hidden node")
        _object(position, position_path)
        for coordinate in ("x", "y"):
            _number(position.get(coordinate), f"{position_path}.{coordinate}")
    routes = _object(value.setdefault("routes", {}), f"{path}.routes")
    for key, route in routes.items():
        route_path = f"{path}.routes.{key}"
        if key not in mode["edgeIds"]:
            _fail(route_path, "unknown or hidden edge")
        _object(route, route_path)
        for side in ("start", "end"):
            _enum(route.get(side), {"top", "bottom", "left", "right"}, f"{route_path}.{side}")
            ratio = f"{side}Ratio"
            if ratio in route:
                _number(route[ratio], f"{route_path}.{ratio}")
                if not 0 <= route[ratio] <= 1:
                    _fail(f"{route_path}.{ratio}", "expected a ratio in [0,1]")
        for index, point in enumerate(_array(route.get("via"), f"{route_path}.via")):
            _point(point, f"{route_path}.via[{index}]")
        _point(route.get("label"), f"{route_path}.label")


def _beats(value: Any, step: dict, mode: dict, path: str) -> None:
    for index, beat in enumerate(_array(value, path, nonempty=True)):
        beat_path = f"{path}[{index}]"
        _object(beat, beat_path)
        _text(beat.get("label"), f"{beat_path}.label")
        kind = _enum(beat.get("kind"), {"transfer", "disconnect", "local", "observe", "retain"}, f"{beat_path}.kind")
        if kind in {"transfer", "disconnect"}:
            _references([beat.get("edgeId")], step["edgeIds"], f"{beat_path}.edgeId")
            _enum(beat.get("direction"), {"forward", "reverse"}, f"{beat_path}.direction")
            if "nodeId" in beat:
                _fail(beat_path, "transport beat cannot also target a local node")
        else:
            _references([beat.get("nodeId")], mode["nodeIds"], f"{beat_path}.nodeId")
            if "edgeId" in beat or "direction" in beat:
                _fail(beat_path, "local/observe/retain beat cannot claim transport or direction")


def _ports(value: Any, nodes: dict, sources: dict, path: str) -> dict:
    """Validate inbound operation contracts independently from trace playback."""
    ports = _keyed(value, path)
    for key, port in ports.items():
        port_path = f"{path}[{key}]"
        _text(port.get("label"), f"{port_path}.label")
        _references([port.get("nodeId")], nodes, f"{port_path}.nodeId")
        _enum(port.get("kind"), {"command", "query", "event"}, f"{port_path}.kind")
        _optional_text(port, ("description",), port_path)
        _references(port.setdefault("sourceIds", []), sources, f"{port_path}.sourceIds")
        names = set()
        for index, parameter in enumerate(_array(port.get("input"), f"{port_path}.input")):
            parameter_path = f"{port_path}.input[{index}]"
            _object(parameter, parameter_path)
            name = _identifier(parameter.get("name"), f"{parameter_path}.name")
            if name in names:
                _fail(parameter_path, f"duplicate parameter name {name!r}")
            names.add(name)
            _text(parameter.get("type"), f"{parameter_path}.type")
            if type(parameter.get("required")) is not bool:
                _fail(f"{parameter_path}.required", "expected a boolean")
        outcomes = _keyed(port.get("outcomes"), f"{port_path}.outcomes", nonempty=True)
        for outcome_id, outcome in outcomes.items():
            outcome_path = f"{port_path}.outcomes[{outcome_id}]"
            _text(outcome.get("label"), f"{outcome_path}.label")
            _enum(outcome.get("kind"), {"accepted", "returned", "observed", "rejected", "unknown"}, f"{outcome_path}.kind")
    return ports


def _advance_state(before: dict, patch: dict) -> dict:
    """Apply the same label/tone merge and fact replacement as core.fold."""
    return {
        "nodes": {key: {**status, **patch["nodes"].get(key, {})} for key, status in before["nodes"].items()},
        "facts": {**before["facts"], **patch["facts"]},
    }


def _beat_at(value: Any, beats: list, path: str) -> dict:
    if type(value) is not int or not 0 <= value < len(beats):
        _fail(path, f"expected an integer beat index in 0..{len(beats) - 1}")
    return beats[value]


def _transfer_endpoints(beat: dict, edges: dict) -> tuple[str, str] | None:
    if beat["kind"] != "transfer":
        return None
    edge = edges[beat["edgeId"]]
    if beat["direction"] == "forward":
        return edge["from"], edge["to"]
    return edge["to"], edge["from"]


def _interactions(step: dict, mode: dict, ports: dict, edges: dict, before: dict, after: dict, path: str) -> None:
    """Validate call ownership, authored transport and patch-linked effects."""
    interactions = _keyed(step.setdefault("interactions", []), path)
    for key, interaction in interactions.items():
        call_path = f"{path}[{key}]"
        if "to" in interaction:
            _fail(call_path, "target is owned by portId; a competing to field is not allowed")
        _references([interaction.get("from")], mode["nodeIds"], f"{call_path}.from")
        _references([interaction.get("portId")], ports, f"{call_path}.portId")
        port = ports[interaction["portId"]]
        _references([port["nodeId"]], mode["nodeIds"], f"{call_path}.portId")
        caller, target = interaction["from"], port["nodeId"]
        request = _beat_at(interaction.get("requestBeat"), step["beats"], f"{call_path}.requestBeat")
        local_self = request["kind"] == "local" and caller == target == request["nodeId"]
        if not local_self and _transfer_endpoints(request, edges) != (caller, target):
            _fail(f"{call_path}.requestBeat", "expected caller-to-port transfer or a local self call")

        inputs = _object(interaction.get("input"), f"{call_path}.input")
        declared = {parameter["name"] for parameter in port["input"]}
        required = {parameter["name"] for parameter in port["input"] if parameter["required"]}
        if set(inputs) - declared:
            _fail(f"{call_path}.input", "contains undeclared parameters")
        if required - set(inputs):
            _fail(f"{call_path}.input", f"missing required parameters: {', '.join(sorted(required - set(inputs)))}")
        result = _object(interaction.get("result"), f"{call_path}.result")
        outcomes = {outcome["id"]: outcome for outcome in port["outcomes"]}
        _references([result.get("outcomeId")], outcomes, f"{call_path}.result.outcomeId")
        outcome = outcomes[result["outcomeId"]]
        _text(result.setdefault("summary", outcome["label"]), f"{call_path}.result.summary", nonempty=False)

        if "responseBeat" in interaction:
            response_path = f"{call_path}.responseBeat"
            response = _beat_at(interaction["responseBeat"], step["beats"], response_path)
            if outcome["kind"] == "unknown":
                _fail(response_path, "unknown outcome cannot claim a response")
            if interaction["responseBeat"] <= interaction["requestBeat"]:
                _fail(response_path, "response must follow the request")
            if request["kind"] != "transfer" or response["kind"] != "transfer":
                _fail(response_path, "response requires a transfer request and a transfer reply")
            if response["edgeId"] != request["edgeId"] or response["direction"] == request["direction"]:
                _fail(response_path, "response must use the same channel in the opposite direction")
            if _transfer_endpoints(response, edges) != (target, caller):
                _fail(response_path, "response must return from the port owner to the original caller")

        for index, effect in enumerate(_array(interaction.setdefault("effects", []), f"{call_path}.effects")):
            effect_path = f"{call_path}.effects[{index}]"
            _object(effect, effect_path)
            effect_target = _enum(effect.get("target"), {"node", "fact"}, f"{effect_path}.target")
            kind = _enum(effect.get("kind"), {"change", "observation"}, f"{effect_path}.kind")
            group = "nodes" if effect_target == "node" else "facts"
            _references([effect.get("id")], step["patch"][group], f"{effect_path}.id")
            effect_id = effect["id"]
            if before[group][effect_id] == after[group][effect_id]:
                _fail(effect_path, "effect must reference an actual changed patch value")
            if outcome["kind"] in {"rejected", "unknown"} and kind == "change":
                _fail(effect_path, "rejected/unknown outcome cannot claim a successful change effect")


def validate(model: Any) -> dict[str, Any]:
    """Return a validated independent JSON-compatible model, or raise ValueError.

    Required: schemaVersion=1, title, factDefinitions, defaults, nodes, edges,
    modes and scenarios. Every mode/scenario initial names all visible nodes and
    declared facts. Patches may omit nodes/facts and may update label/tone alone.
    Optional ports and step interactions resolve inbound contracts, real beats
    and changed patch values; inert input values are never evaluated or coerced.
    Optional metadata becomes empty strings/lists, absent patch maps become {},
    and layout hints remain optional. Input is never mutated; source paths are
    display/provenance data and are never opened here. Failures name their path.
    """
    try:
        model = _object(_json_copy(model), "$")
    except RecursionError:
        _fail("$", "expected a finite acyclic JSON tree within Python's nesting limit")
    if type(model.get("schemaVersion")) is not int or model["schemaVersion"] != 1:
        _fail("$.schemaVersion", "expected schema version 1")
    _text(model.get("title"), "$.title")
    _optional_text(model, ("scope",), "$")
    sources = _keyed(model.setdefault("sources", []), "$.sources")
    facts = _keyed(model.get("factDefinitions"), "$.factDefinitions")
    nodes = _keyed(model.get("nodes"), "$.nodes", nonempty=True)
    edges = _keyed(model.get("edges"), "$.edges")
    modes = _keyed(model.get("modes"), "$.modes", nonempty=True)
    scenarios = _keyed(model.get("scenarios"), "$.scenarios", nonempty=True)
    ports = _ports(model.setdefault("ports", []), nodes, sources, "$.ports")

    for collection_name, collection in (("sources", sources), ("factDefinitions", facts), ("nodes", nodes), ("edges", edges), ("modes", modes), ("scenarios", scenarios)):
        for key, item in collection.items():
            _text(item.get("label"), f"$.{collection_name}[{key}].label")

    for key, source in sources.items():
        path = f"$.sources[{key}]"
        _text(source.get("path"), f"{path}.path")
        _optional_text(source, ("basis",), path)
    module_ids = set()
    for key, node in nodes.items():
        path = f"$.nodes[{key}]"
        _enum(node.get("kind"), NODE_KINDS, f"{path}.kind")
        _optional_text(node, ("owner", "executable", "lifetime"), path)
        _strings(node.setdefault("capabilities", []), f"{path}.capabilities")
        _references(node.setdefault("sourceIds", []), sources, f"{path}.sourceIds")
        _references(node.setdefault("stateFacts", []), facts, f"{path}.stateFacts")
        modules = _keyed(node.setdefault("modules", []), f"{path}.modules")
        for module_id, module in modules.items():
            module_path = f"{path}.modules[{module_id}]"
            if module_id in module_ids:
                _fail(module_path, "duplicate module id across nodes")
            module_ids.add(module_id)
            _text(module.get("label"), f"{module_path}.label")
            _optional_text(module, ("path", "description"), module_path)
    for key, edge in edges.items():
        path = f"$.edges[{key}]"
        for endpoint in ("from", "to"):
            _references([edge.get(endpoint)], nodes, f"{path}.{endpoint}")
        _optional_text(edge, ("transport", "purpose"), path)
        _references(edge.setdefault("sourceIds", []), sources, f"{path}.sourceIds")
    for key, mode in modes.items():
        path = f"$.modes[{key}]"
        _optional_text(mode, ("command", "summary"), path)
        _references(mode.get("nodeIds"), nodes, f"{path}.nodeIds", nonempty=True)
        _references(mode.get("edgeIds"), edges, f"{path}.edgeIds")
        for edge_id in mode["edgeIds"]:
            edge = edges[edge_id]
            if not {edge["from"], edge["to"]} <= set(mode["nodeIds"]):
                _fail(f"{path}.edgeIds", f"edge {edge_id!r} has a hidden endpoint")
        _state(mode.get("initial"), mode, facts, f"{path}.initial", complete=True)
        if "layout" in mode:
            _layout(mode["layout"], mode, f"{path}.layout")
    for key, scenario in scenarios.items():
        path = f"$.scenarios[{key}]"
        _references([scenario.get("modeId")], modes, f"{path}.modeId")
        mode = modes[scenario["modeId"]]
        _optional_text(scenario, ("question",), path)
        _state(scenario.get("initial"), mode, facts, f"{path}.initial", complete=True)
        steps = _keyed(scenario.get("steps"), f"{path}.steps", nonempty=True)
        state = scenario["initial"]
        for step_id, step in steps.items():
            step_path = f"{path}.steps[{step_id}]"
            _text(step.get("title"), f"{step_path}.title")
            _text(step.get("event"), f"{step_path}.event")
            _strings(step.get("actions"), f"{step_path}.actions", nonempty=True)
            for endpoint in ("actor", "handler"):
                _references([step.get(endpoint)], mode["nodeIds"], f"{step_path}.{endpoint}")
            _references(step.get("edgeIds"), mode["edgeIds"], f"{step_path}.edgeIds")
            _optional_text(step, ("certainty", "note"), step_path)
            _references(step.setdefault("sourceIds", []), sources, f"{step_path}.sourceIds")
            _state(step.get("patch"), mode, facts, f"{step_path}.patch", complete=False)
            _beats(step.get("beats"), step, mode, f"{step_path}.beats")
            after = _advance_state(state, step["patch"])
            _interactions(step, mode, ports, edges, state, after, f"{step_path}.interactions")
            state = after
    for mode_id in modes:
        if not any(scenario["modeId"] == mode_id for scenario in scenarios.values()):
            _fail(f"$.modes[{mode_id}]", "mode must have at least one scenario")
    defaults = _object(model.get("defaults"), "$.defaults")
    _references([defaults.get("modeId")], modes, "$.defaults.modeId")
    _references([defaults.get("scenarioId")], scenarios, "$.defaults.scenarioId")
    if scenarios[defaults["scenarioId"]]["modeId"] != defaults["modeId"]:
        _fail("$.defaults", "scenarioId must belong to modeId")
    return model
