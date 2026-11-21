/* Spatial object surface: state fields, inbound pins and call paths.
 * Consumes folded snapshots and resolved calls; owns no state reducer or clock.
 */
(function (global) {
  "use strict";

  function box(model, node, position) {
    var ports = FlowAtlasPorts.owned(model, node.id);
    var rows = ports.length ? ports.length * 24 : (node.modules || []).length * 22;
    var height = 52 + (node.stateFacts || []).length * 36 + rows;
    return { x: position.x, y: position.y, width: 228, height: height + (height > 52 ? 6 : 0) };
  }

  function pins(model, mode, layout, boxes) {
    var callers = {}, result = {};
    model.scenarios.filter(function (s) { return s.modeId === mode.id; }).forEach(function (s) {
      s.steps.forEach(function (step) {
        (step.interactions || []).forEach(function (call) {
          if (!callers[call.portId]) callers[call.portId] = [];
          callers[call.portId].push(boxes[call.from].x + boxes[call.from].width / 2);
        });
      });
    });
    mode.nodeIds.forEach(function (id) {
      var node = model.nodes.find(function (item) { return item.id === id; });
      var bounds = boxes[id];
      FlowAtlasPorts.owned(model, id).forEach(function (port, index) {
        var origins = callers[port.id] || [];
        var mean = origins.length ? origins.reduce(function (sum, x) { return sum + x; }, 0) / origins.length : bounds.x - 1;
        var side = mean > bounds.x + bounds.width / 2 ? "right" : "left";
        result[port.id] = { x: bounds.x + (side === "right" ? bounds.width : 0), y: bounds.y + 52 + (node.stateFacts || []).length * 36 + index * 24 + 12, side: side };
      });
    });
    return result;
  }

  function link(call, boxes, anchors, lane) {
    var source = boxes[call.from.id], target = anchors[call.port.id];
    if (!source || !target) return null;
    var right = source.x + source.width / 2 < boxes[call.target.id].x + boxes[call.target.id].width / 2;
    var start = { x: source.x + (right ? source.width : 0), y: source.y + Math.min(source.height - 12, 24 + (lane || 0) * 8) };
    var middle = (start.x + target.x) / 2 + ((lane || 0) % 3 - 1) * 7;
    if (call.from.id === call.target.id) {
      start = { x: source.x + source.width, y: source.y + 22 };
      middle = source.x + source.width + 28;
    }
    return { start: start, end: target, points: [start, { x: middle, y: start.y }, { x: middle, y: target.y }, target], path: "M " + start.x + " " + start.y + " H " + middle + " V " + target.y + " H " + target.x };
  }

  function drawNode(ctx, node, bounds, anchors) {
    var fields = node.stateFacts || [];
    fields.forEach(function (id, index) {
      var fact = ctx.model.factDefinitions.find(function (f) { return f.id === id; });
      var change = ctx.changes.find(function (c) { return c.kind === "fact" && c.id === id; });
      var applied = ctx.phase === "applied" && Boolean(change);
      var value = ctx.snapshot.facts[id];
      var label = applied ? ctx.shorten(change.before, 13) + " → " + ctx.shorten(value, 20) : value;
      var y = bounds.y + 50 + index * 36;
      var group = ctx.svg("g", { class: "canvas-state" + (applied ? " is-changed" : ""), "data-fact-id": id, "data-testid": "canvas-state-" + node.id + "-" + id, "data-current-value": value, "data-applied": String(applied), role: "button", tabindex: 0, "aria-label": fact.label + "：" + (applied ? change.before + " → " + value : value) });
      group.appendChild(ctx.svg("rect", { class: "canvas-state__body", x: bounds.x + 8, y: y, width: bounds.width - 16, height: 34, rx: 5 }));
      group.appendChild(ctx.svg("text", { class: "canvas-state__key", x: bounds.x + 14, y: y + 12 }, fact.label + (applied ? " · Δ" : "")));
      group.appendChild(ctx.svg("text", { class: "canvas-state__value", x: bounds.x + 14, y: y + 27 }, ctx.shorten(label, 32)));
      group.appendChild(ctx.svg("title", {}, fact.label + " · " + (applied ? change.before + " → " + value : value)));
      ctx.group.appendChild(group);
    });
    FlowAtlasPorts.owned(ctx.model, node.id).forEach(function (port) {
      var pin = anchors[port.id];
      var active = ctx.activePortId === port.id;
      var group = ctx.svg("g", { class: "canvas-port canvas-port--" + port.kind + (active ? " is-active" : ""), "data-port-id": port.id, "data-testid": "canvas-port-" + port.id, "data-pin-x": pin.x, "data-pin-y": pin.y, tabindex: 0, role: "button", "aria-label": node.label + "." + FlowAtlasPorts.signature(port) + "，" + FlowAtlasPorts.kinds[port.kind] });
      group.appendChild(ctx.svg("rect", { class: "canvas-port__body", x: bounds.x + 5, y: pin.y - 11, width: bounds.width - 10, height: 22, rx: 4 }));
      group.appendChild(ctx.svg("circle", { class: "canvas-port__hit", cx: pin.x, cy: pin.y, r: 12 }));
      if (port.kind === "event") group.appendChild(ctx.svg("path", { class: "canvas-port__pin", d: "M " + pin.x + " " + (pin.y - 5) + " l 5 5 l -5 5 l -5 -5 Z" }));
      else group.appendChild(ctx.svg("circle", { class: "canvas-port__pin", cx: pin.x, cy: pin.y, r: 4.5 }));
      group.appendChild(ctx.svg("text", { class: "canvas-port__label", x: bounds.x + 15, y: pin.y + 4 }, ctx.shorten(port.label, 29)));
      group.appendChild(ctx.svg("text", { class: "canvas-port__kind", x: bounds.x + bounds.width - 15, y: pin.y + 4, "text-anchor": "end" }, { command: "命", query: "查", event: "事" }[port.kind]));
      group.appendChild(ctx.svg("title", {}, FlowAtlasPorts.kinds[port.kind] + " · " + FlowAtlasPorts.signature(port)));
      ctx.group.appendChild(group);
    });
  }

  function drawCalls(ctx, svg, calls, boxes, anchors) {
    var activeUse = FlowAtlasPorts.atBeat(ctx.currentCalls, ctx.beatIndex);
    var diagnostics = [];
    calls.forEach(function (entry, index) {
      var call = entry.call, geometry = link(call, boxes, anchors, index);
      var routeId = "call:" + entry.stepIndex + ":" + call.id;
      var issues = ctx.routeDiagnostics ? ctx.routeDiagnostics(routeId, geometry && geometry.points, boxes) : [];
      diagnostics = diagnostics.concat(issues);
      if (!geometry) return;
      var active = entry.stepIndex === ctx.stepIndex && (!activeUse || activeUse.call.id === call.id);
      var returning = active && activeUse && activeUse.response;
      var group = ctx.svg("g", { class: "canvas-call" + (active ? " is-active" : "") + (returning ? " is-response" : ""), "data-call-link": entry.stepIndex + ":" + call.id, "data-target-port": call.port.id, "data-port-id": call.port.id, "data-testid": "canvas-call-" + call.id, tabindex: 0, role: "button", "aria-label": call.from.label + " → " + call.target.label + "." + call.port.label });
      group.setAttribute("data-route-status", issues.length ? issues[0].code : "clear");
      if (issues.length) group.classList.add("has-layout-diagnostic");
      group.appendChild(ctx.svg("path", { class: "canvas-call__hit", d: geometry.path }));
      group.appendChild(ctx.svg("path", { class: "canvas-call__line", d: geometry.path, "marker-end": returning ? "none" : "url(#canvas-call-arrow)", "marker-start": returning ? "url(#canvas-call-arrow)" : "none" }));
      group.appendChild(ctx.svg("circle", { class: "canvas-call__origin", cx: geometry.start.x, cy: geometry.start.y, r: 3.5 }));
      group.appendChild(ctx.svg("title", {}, call.from.label + " 发出调用 → " + call.target.label + "." + FlowAtlasPorts.signature(call.port) + (issues.length ? " · " + issues[0].message : "")));
      svg.appendChild(group);
    });
    return diagnostics;
  }

  function payload(call, response) {
    if (response) return "↩ 返回数据 · 待确认";
    var keys = Object.keys(call.input);
    return keys.length ? keys.slice(0, 2).map(function (key) { return key + "=" + (typeof call.input[key] === "string" ? call.input[key] : JSON.stringify(call.input[key])); }).join(" · ") : "{} · 无参数";
  }

  function caption(ctx, parent, call, response) {
    var lines = response ? ["↩ 返回数据 · 待确认"] : Object.keys(call.input).slice(0, 3).map(function (key) {
      return key + ": " + (typeof call.input[key] === "string" ? call.input[key] : JSON.stringify(call.input[key]));
    });
    if (!lines.length) lines = ["{} · 无参数"];
    var visible = lines.map(function (line) { return ctx.shorten(line, 42); });
    var units = function (value) { return Array.from(value).reduce(function (sum, ch) { return sum + (/[^\x00-\x7f]/.test(ch) ? 2 : 1); }, 0); };
    var width = Math.max(150, Math.max.apply(null, visible.map(units)) * 6.5 + 20);
    var height = lines.length * 18 + 14;
    var group = ctx.svg("g", { class: "canvas-payload", "data-caption-height": height, transform: "translate(0 " + (-height - 14) + ")", "data-testid": "canvas-payload" });
    group.appendChild(ctx.svg("rect", { class: "canvas-payload__body", x: -width / 2, y: 0, width: width, height: height, rx: 6 }));
    visible.forEach(function (line, index) { group.appendChild(ctx.svg("text", { class: "canvas-payload__text", x: -width / 2 + 10, y: 20 + index * 18 }, line)); });
    group.appendChild(ctx.svg("title", {}, call.port.label + " · " + lines.join("\n")));
    parent.appendChild(group);
  }

  global.FlowAtlasCanvas = Object.freeze({ box: box, pins: pins, link: link, drawNode: drawNode, drawCalls: drawCalls, payload: payload, caption: caption });
}(globalThis));
