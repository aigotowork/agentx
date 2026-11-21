/* Pure finite-trace projections. Load before viewer scripts; see README.md. */
var FlowAtlas = (function () {
  "use strict";

  function byId(items, id) {
    return (items || []).find(function (item) { return item.id === id; }) || null;
  }

  function selectedScenario(model, scenarioId) {
    var scenario = byId(model.scenarios, scenarioId);
    if (!scenario) throw new RangeError("Unknown scenario: " + scenarioId);
    return scenario;
  }

  function checkpoint(scenario, index) {
    if (!Number.isInteger(index) || index < -1 || index >= scenario.steps.length) {
      throw new RangeError("Scenario " + scenario.id + ": checkpoint must be -1.." + (scenario.steps.length - 1));
    }
  }

  function copyMap(value) {
    return JSON.parse(JSON.stringify(value || {}));
  }

  function initialState(value) {
    return { nodes: copyMap(value.nodes), facts: copyMap(value.facts) };
  }

  function apply(result, patch) {
    Object.keys(patch.nodes || {}).forEach(function (id) {
      var status = copyMap(result.nodes[id]);
      Object.keys(patch.nodes[id]).forEach(function (key) { status[key] = patch.nodes[id][key]; });
      result.nodes[id] = status;
    });
    Object.keys(patch.facts || {}).forEach(function (id) { result.facts[id] = patch.facts[id]; });
  }

  /** Return a fresh {nodes,facts} snapshot; -1 means initial, i includes step i. */
  function fold(model, scenarioId, index) {
    var scenario = selectedScenario(model, scenarioId);
    checkpoint(scenario, index);
    var result = initialState(scenario.initial);
    for (var i = 0; i <= index; i += 1) apply(result, scenario.steps[i].patch || {});
    return result;
  }

  /** Return authored-order differences; node tones supplement label strings. */
  function changes(model, scenarioId, index) {
    var scenario = selectedScenario(model, scenarioId);
    checkpoint(scenario, index);
    if (index === -1) return [];
    var before = fold(model, scenarioId, index - 1);
    var after = fold(model, scenarioId, index);
    var patch = scenario.steps[index].patch || {};
    var result = [];
    Object.keys(patch.nodes || {}).forEach(function (id) {
      var previous = before.nodes[id];
      var next = after.nodes[id];
      if (previous.label !== next.label || previous.tone !== next.tone) {
        result.push({
          id: id, kind: "node", label: byId(model.nodes, id).label,
          before: previous.label, after: next.label,
          beforeTone: previous.tone, afterTone: next.tone
        });
      }
    });
    Object.keys(patch.facts || {}).forEach(function (id) {
      if (before.facts[id] !== after.facts[id]) {
        result.push({ id: id, kind: "fact", label: byId(model.factDefinitions, id).label, before: before.facts[id], after: after.facts[id] });
      }
    });
    return result;
  }

  /** Resolve authored calls at checkpoint i; callers own every returned value. */
  function calls(model, scenarioId, index) {
    var scenario = selectedScenario(model, scenarioId);
    checkpoint(scenario, index);
    if (index === -1) return [];
    var differences = changes(model, scenarioId, index);
    return (scenario.steps[index].interactions || []).map(function (interaction) {
      var port = byId(model.ports, interaction.portId);
      var outcome = byId(port.outcomes, interaction.result.outcomeId);
      var result = {
        id: interaction.id,
        from: byId(model.nodes, interaction.from),
        target: byId(model.nodes, port.nodeId),
        port: port,
        requestBeat: interaction.requestBeat,
        input: interaction.input,
        result: {
          id: outcome.id, label: outcome.label, kind: outcome.kind,
          summary: interaction.result.summary === undefined ? outcome.label : interaction.result.summary
        },
        effects: (interaction.effects || []).map(function (effect) {
          var difference = differences.find(function (item) { return item.kind === effect.target && item.id === effect.id; });
          var resolved = {
            target: effect.target, id: effect.id, kind: effect.kind,
            label: difference.label, before: difference.before, after: difference.after
          };
          if (effect.target === "node") {
            resolved.beforeTone = difference.beforeTone;
            resolved.afterTone = difference.afterTone;
          }
          return resolved;
        })
      };
      if (interaction.responseBeat !== undefined) result.responseBeat = interaction.responseBeat;
      return copyMap(result);
    });
  }

  /** Use authored defaults; raw/empty UI fallback chooses first compatible IDs. */
  function defaultSelection(model) {
    var defaults = model.defaults || {};
    var mode = byId(model.modes, defaults.modeId) || (model.modes || [])[0];
    var scenarios = (model.scenarios || []).filter(function (item) { return mode && item.modeId === mode.id; });
    var scenario = byId(scenarios, defaults.scenarioId) || scenarios[0];
    return { modeId: mode ? mode.id : null, scenarioId: scenario ? scenario.id : null };
  }

  return Object.freeze({ fold: fold, changes: changes, calls: calls, defaultSelection: defaultSelection });
}());
