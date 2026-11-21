/* Port contracts and observable call phases; consumes pure calls, owns no clock. */
(function (global) {
  "use strict";
  var kinds = { command: "命令", query: "查询", event: "事件" };
  var results = { accepted: "已接受", returned: "已返回", observed: "已观察", rejected: "被拒绝", unknown: "结果未知" };

  function status(call, phase, beatIndex) {
    if (phase === "applied") return { stage: call.result.kind, label: call.result.label, known: true };
    if (phase === "ready" || (phase === "flow" && beatIndex < call.requestBeat)) return { stage: "planned", label: "未调用", known: false };
    if (phase === "flow" && beatIndex === call.requestBeat) return { stage: "calling", label: call.port.kind === "event" ? "事件传入" : "调用中", known: false };
    if (phase === "flow" && call.responseBeat !== undefined && beatIndex === call.responseBeat) return { stage: "returning", label: "返回中", known: false };
    return { stage: "waiting", label: "等待结果", known: false };
  }

  function forStep(model, scenarioId, index) { return index < 0 ? [] : FlowAtlas.calls(model, scenarioId, index); }
  function owned(model, nodeId) { return (model.ports || []).filter(function (port) { return port.nodeId === nodeId; }); }
  function signature(port) { return port.label + "(" + port.input.map(function (field) { return field.name + (field.required ? "" : "?"); }).join(", ") + ")"; }

  function atBeat(calls, index) {
    var response = calls.find(function (call) { return call.responseBeat === index; });
    if (response) return { call: response, response: true };
    var request = calls.find(function (call) { return call.requestBeat === index; });
    return request ? { call: request, response: false } : null;
  }

  function beatLabel(calls, index, phase) {
    var use = atBeat(calls, index);
    if (!use) return null;
    if (use.response) return "↩ " + (phase === "applied" ? use.call.result.label : "结果待确认");
    return kinds[use.call.port.kind] + " · " + use.call.port.label;
  }

  function showContract(port, owner, ui) {
    var wrapper = ui.element("section", "port-contract");
    wrapper.setAttribute("data-testid", "port-contract");
    wrapper.appendChild(ui.element("h3", "", port.label));
    wrapper.appendChild(ui.element("div", "port-owner", owner.label + " · " + kinds[port.kind]));
    wrapper.appendChild(ui.element("code", "port-signature", signature(port)));
    if (port.description) wrapper.appendChild(ui.element("p", "detail-description", port.description));
    var inputs = ui.element("section", "inspector-section");
    inputs.appendChild(ui.element("h4", "", "输入"));
    var list = ui.element("dl", "detail-list");
    if (!port.input.length) inputs.appendChild(ui.element("span", "source-item__basis", "无参数"));
    port.input.forEach(function (field) { ui.row(list, field.name, field.type + (field.required ? " · 必填" : " · 可选")); });
    inputs.appendChild(list);
    wrapper.appendChild(inputs);
    var outcomes = ui.element("section", "inspector-section");
    outcomes.appendChild(ui.element("h4", "", "允许的结果"));
    port.outcomes.forEach(function (outcome) {
      var item = ui.element("div", "port-outcome port-outcome--" + outcome.kind, results[outcome.kind] + " · " + outcome.label);
      item.setAttribute("data-testid", "port-outcome-" + outcome.id);
      outcomes.appendChild(item);
    });
    wrapper.appendChild(outcomes);
    ui.sources(wrapper, port.sourceIds);
    return wrapper;
  }

  function showCall(call, current, ui) {
    var wrapper = ui.element("section", "call-details");
    wrapper.setAttribute("data-testid", "call-details-" + call.id);
    var heading = ui.element("div", "call-details__heading");
    heading.appendChild(ui.element("strong", "", call.from.label + " → " + call.target.label));
    heading.appendChild(ui.element("span", "call-status call-status--" + current.stage, current.label));
    wrapper.appendChild(heading);
    wrapper.appendChild(ui.element("code", "port-signature", signature(call.port)));
    var inputs = ui.element("dl", "detail-list");
    Object.keys(call.input).forEach(function (key) {
      var value = typeof call.input[key] === "string" ? call.input[key] : JSON.stringify(call.input[key]);
      ui.row(inputs, key, value);
    });
    wrapper.appendChild(inputs);
    if (current.known) {
      var result = ui.element("p", "call-result call-result--" + call.result.kind, call.result.summary);
      result.setAttribute("data-testid", "call-result-" + call.id);
      wrapper.appendChild(result);
    } else wrapper.appendChild(ui.element("span", "source-item__basis", "结果尚未可知"));
    var effects = ui.element("section", "inspector-section");
    effects.appendChild(ui.element("h4", "", current.known ? "相关变化" : "待应用变化"));
    if (!call.effects.length) effects.appendChild(ui.element("span", "source-item__basis", "此交互未改写检查点"));
    call.effects.forEach(function (effect) {
      var value = effect.before + " → " + effect.after;
      if (effect.before === effect.after && effect.beforeTone !== effect.afterTone) value += " (" + effect.beforeTone + " → " + effect.afterTone + ")";
      var row = ui.element("div", "call-effect", (effect.kind === "observation" ? "◎ " : "Δ ") + effect.label + " · " + value);
      row.setAttribute("data-testid", "call-effect-" + effect.id);
      row.setAttribute("data-applied", String(current.known));
      effects.appendChild(row);
    });
    wrapper.appendChild(effects);
    return wrapper;
  }

  global.FlowAtlasPorts = Object.freeze({ kinds: kinds, status: status, owned: owned, forStep: forStep, signature: signature, atBeat: atBeat, beatLabel: beatLabel, showContract: showContract, showCall: showCall });
}(globalThis));
