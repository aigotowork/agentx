/* Alternate projections of one authored trace. No clock, reducer or domain IDs. */
(function (global) {
  "use strict";
  var symbols = { local: "⚙", observe: "◎", retain: "Ⅱ", disconnect: "×" };
  var labels = { transfer: "传输", disconnect: "断开", local: "本地", observe: "观察", retain: "保留" };

  function root(ctx, width, height, label) {
    return ctx.svg("svg", { viewBox: "0 0 " + width + " " + height, width: width, height: height, role: "img", "aria-label": label });
  }

  function title(ctx, parent, text) { parent.appendChild(ctx.svg("title", {}, text)); }

  function arrow(ctx, svg, id) {
    var defs = ctx.svg("defs");
    var marker = ctx.svg("marker", { id: id, viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 6, markerHeight: 6, orient: "auto-start-reverse" });
    marker.appendChild(ctx.svg("path", { d: "M 0 0 L 10 5 L 0 10 z", class: "trace-arrow" }));
    defs.appendChild(marker);
    svg.appendChild(defs);
  }

  function stepStatus(ctx, index) {
    if (index === ctx.stepIndex) return ctx.phase === "applied" ? "已应用" : "演示中";
    return index < ctx.stepIndex ? "已过" : "待演示";
  }

  function stepClass(ctx, index) {
    return "trace-step" + (index === ctx.stepIndex ? " is-current" : "") + (index < ctx.stepIndex ? " is-completed" : "");
  }

  function renderFlow(ctx) {
    var steps = ctx.scenario.steps;
    var available = ctx.viewportWidth || 1044;
    var columns = Math.min(available >= 1016 ? 3 : (available >= 684 ? 2 : 1), steps.length);
    var width = columns * 332 + 20;
    var height = Math.max(360, Math.ceil(steps.length / columns) * 166 + 54);
    var svg = root(ctx, width, height, "当前场景的步骤流程");
    arrow(ctx, svg, "trace-arrow");
    var positions = steps.map(function (_, index) {
      var row = Math.floor(index / columns);
      var column = index % columns;
      if (row % 2) column = columns - 1 - column;
      return { x: 32 + column * 332, y: 36 + row * 166 };
    });
    positions.forEach(function (p, index) {
      if (!index) return;
      var prev = positions[index - 1];
      var sameRow = p.y === prev.y;
      var towardRight = p.x > prev.x;
      var x1 = sameRow ? prev.x + (towardRight ? 288 : 0) : prev.x + 144;
      var x2 = sameRow ? p.x + (towardRight ? 0 : 288) : p.x + 144;
      var y1 = sameRow ? prev.y + 62 : prev.y + 124;
      var y2 = sameRow ? p.y + 62 : p.y;
      svg.appendChild(ctx.svg("path", { d: "M " + x1 + " " + y1 + " L " + x2 + " " + y2, class: "trace-connector", "marker-end": "url(#trace-arrow)" }));
    });
    steps.forEach(function (step, index) {
      var p = positions[index];
      var group = ctx.svg("g", { class: stepClass(ctx, index), "data-step-index": index, "data-testid": "flow-step-" + step.id, role: "button", tabindex: 0, "aria-label": (index + 1) + ". " + step.title + "，" + stepStatus(ctx, index) + "，演示此事件" });
      title(ctx, group, step.title + " · " + ctx.label(step.actor) + " → " + ctx.label(step.handler));
      group.appendChild(ctx.svg("rect", { class: "trace-step__body", x: p.x, y: p.y, width: 288, height: 124, rx: 10 }));
      group.appendChild(ctx.svg("circle", { class: "trace-step__number-bg", cx: p.x + 23, cy: p.y + 24, r: 13 }));
      group.appendChild(ctx.svg("text", { class: "trace-step__number", x: p.x + 23, y: p.y + 28, "text-anchor": "middle" }, index + 1));
      group.appendChild(ctx.svg("text", { class: "trace-step__title", x: p.x + 46, y: p.y + 28 }, ctx.shorten(step.title, 30)));
      group.appendChild(ctx.svg("text", { class: "trace-step__route", x: p.x + 14, y: p.y + 56 }, ctx.shorten(ctx.label(step.actor), 24) + " →"));
      var call = ctx.calls(index)[0];
      var targetLabel = call ? call.target.label + "." + call.port.label : ctx.label(step.handler);
      group.appendChild(ctx.svg("text", { class: "trace-step__route", x: p.x + 14, y: p.y + 76 }, ctx.shorten(targetLabel, 38)));
      group.appendChild(ctx.svg("text", { class: "trace-step__meta", x: p.x + 14, y: p.y + 106 }, ctx.beats(step).length + " 个动作 · Δ " + ctx.changes(index).length));
      var known = index < ctx.stepIndex || (index === ctx.stepIndex && ctx.phase === "applied");
      var status = call && known ? call.result.label : stepStatus(ctx, index);
      group.appendChild(ctx.svg("text", { class: "trace-step__status" + (call && known ? " result--" + call.result.kind : ""), x: p.x + 273, y: p.y + 106, "text-anchor": "end", "data-result-known": String(Boolean(call && known)) }, ctx.shorten(status, 19)));
      svg.appendChild(group);
    });
    return { svg: svg, width: width };
  }

  function sequenceRows(ctx) {
    var rows = [];
    ctx.scenario.steps.forEach(function (step, stepIndex) {
      ctx.beats(step).forEach(function (beat, beatIndex) {
        rows.push({ step: step, stepIndex: stepIndex, beat: beat, beatIndex: beatIndex });
      });
    });
    return rows;
  }

  function renderSequence(ctx) {
    var rows = sequenceRows(ctx);
    var used = new Set();
    rows.forEach(function (row) {
      used.add(row.step.actor); used.add(row.step.handler);
      if (row.beat.nodeId) used.add(row.beat.nodeId);
      if (row.beat.edgeId) { var edge = ctx.edge(row.beat.edgeId); used.add(edge.from); used.add(edge.to); }
    });
    var ids = ctx.mode.nodeIds.filter(function (id) { return used.has(id); });
    var width = Math.max(760, 210 + ids.length * 174);
    var height = 96 + rows.length * 58;
    var svg = root(ctx, width, height, "按动作顺序展开的交互时序");
    arrow(ctx, svg, "sequence-arrow");
    var centers = {};
    var header = ctx.svg("g", { "data-testid": "sequence-header" });
    header.appendChild(ctx.svg("rect", { class: "sequence-header__background", x: 0, y: 0, width: width, height: 72 }));
    ids.forEach(function (id, index) {
      var x = 210 + index * 174 + 87;
      centers[id] = x;
      svg.appendChild(ctx.svg("line", { class: "sequence-lifeline", x1: x, y1: 70, x2: x, y2: height - 16 }));
      var currentNode = ctx.node(id);
      var group = ctx.svg("g", { class: "sequence-participant" + (ctx.selectedNodeId === id ? " is-selected" : ""), role: "button", tabindex: 0, "data-node-id": id, "data-testid": "sequence-node-" + id, "aria-label": currentNode.label });
      title(ctx, group, currentNode.label + " · " + ctx.snapshot.nodes[id].label);
      group.appendChild(ctx.svg("rect", { class: "sequence-participant__body", x: x - 81, y: 12, width: 162, height: 54, rx: 8 }));
      group.appendChild(ctx.svg("text", { class: "trace-step__title", x: x, y: 34, "text-anchor": "middle" }, ctx.shorten(currentNode.label, 22)));
      group.appendChild(ctx.svg("text", { class: "sequence-state", x: x, y: 53, "text-anchor": "middle" }, ctx.shorten(ctx.snapshot.nodes[id].label, 24)));
      header.appendChild(group);
    });
    rows.forEach(function (row, index) {
      var y = 102 + index * 58;
      var current = row.stepIndex === ctx.stepIndex && ctx.phase === "flow" && row.beatIndex === ctx.beatIndex;
      var group = ctx.svg("g", { class: "sequence-row" + (current ? " is-current" : "") + (row.beat.kind === "disconnect" ? " is-disconnect" : ""), "data-trace-step": row.stepIndex, "data-trace-beat": row.beatIndex });
      group.appendChild(ctx.svg("rect", { class: "sequence-row__background", x: 4, y: y - 26, width: width - 8, height: 54, rx: 6 }));
      if (row.beatIndex === 0) {
        var step = ctx.svg("g", { class: stepClass(ctx, row.stepIndex), "data-step-index": row.stepIndex, "data-testid": "sequence-step-" + row.step.id, role: "button", tabindex: 0, "aria-label": "演示 " + row.step.title });
        title(ctx, step, row.step.title);
        step.appendChild(ctx.svg("rect", { class: "sequence-step__hit", x: 8, y: y - 23, width: 194, height: 49, rx: 6 }));
        step.appendChild(ctx.svg("text", { class: "trace-step__title", x: 18, y: y - 4 }, (row.stepIndex + 1) + ". " + ctx.shorten(row.step.title, 23)));
        step.appendChild(ctx.svg("text", { class: "trace-step__meta", x: 18, y: y + 15 }, stepStatus(ctx, row.stepIndex)));
        group.appendChild(step);
      }
      var beat = row.beat;
      var callPhase = row.stepIndex < ctx.stepIndex || (row.stepIndex === ctx.stepIndex && ctx.phase === "applied") ? "applied" : "ready";
      var callUse = FlowAtlasPorts.atBeat(ctx.calls(row.stepIndex), row.beatIndex);
      var beatLabel = FlowAtlasPorts.beatLabel(ctx.calls(row.stepIndex), row.beatIndex, callPhase) || beat.label;
      if (callUse) {
        group.setAttribute("data-call-id", callUse.call.id);
        group.setAttribute("data-call-role", callUse.response ? "response" : "request");
        group.setAttribute("data-result-known", String(callPhase === "applied"));
        if (callUse.response) group.classList.add("is-response");
      }
      title(ctx, group, labels[beat.kind] + " · " + beatLabel);
      if (beat.edgeId) {
        var edge = ctx.edge(beat.edgeId);
        var from = centers[edge.from], to = centers[edge.to];
        var direction = beat.direction === "reverse" ? -1 : 1;
        var attrs = { "data-direction": beat.direction, class: "map-edge__line sequence-transfer", d: "M " + from + " " + y + " L " + to + " " + y };
        if (beat.kind === "transfer") attrs[direction === 1 ? "marker-end" : "marker-start"] = "url(#sequence-arrow)";
        var link = ctx.svg("g", { "data-edge-id": edge.id });
        link.appendChild(ctx.svg("path", attrs));
        title(ctx, link, beat.label + " · " + edge.transport);
        group.appendChild(link);
        var middle = (from + to) / 2;
        var labelAttrs = { class: "sequence-beat-label", x: middle, y: y - 9, "text-anchor": "middle" };
        if (callUse && !callUse.response) Object.assign(labelAttrs, { class: "sequence-beat-label sequence-port", "data-port-id": callUse.call.port.id, "data-testid": "sequence-port-" + row.step.id, tabindex: 0, role: "button", "aria-label": "查看 " + callUse.call.port.label + " 契约" });
        group.appendChild(ctx.svg("text", labelAttrs, ctx.shorten(beatLabel, Math.max(18, Math.abs(from - to) / 7))));
        if (beat.kind === "disconnect") group.appendChild(ctx.svg("text", { class: "sequence-break", x: middle, y: y + 6, "text-anchor": "middle", "data-testid": current ? "flow-break" : "sequence-break" }, "×"));
        if (current && beat.kind === "transfer") {
          var packet = ctx.svg("g", { class: "event-flow event-flow--transfer", "data-testid": "flow-marker", "pointer-events": "none", "aria-label": beat.label });
          packet.appendChild(ctx.svg("circle", { class: "event-flow__halo", r: 14 }));
          packet.appendChild(ctx.svg("circle", { class: "event-flow__packet", r: 7 }));
          packet.appendChild(ctx.svg("path", { class: "event-flow__arrow", d: "M -2 -3 L 2 0 L -2 3" }));
          group.appendChild(packet);
        }
      } else {
        var x = centers[beat.nodeId];
        group.appendChild(ctx.svg("rect", { class: "sequence-local", x: x - 7, y: y - 17, width: 14, height: 32, rx: 4 }));
        group.appendChild(ctx.svg("text", { class: "sequence-local-label", x: x + 14, y: y + 4, "data-testid": current ? "flow-cue" : "sequence-local", "aria-label": labels[beat.kind] + " · " + beat.label }, (symbols[beat.kind] || "·") + " " + ctx.shorten(beatLabel, 27)));
      }
      svg.appendChild(group);
    });
    svg.appendChild(header);
    return { svg: svg, width: width };
  }

  global.FlowAtlasViews = Object.freeze({ render: function (view, context) { return view === "flow" ? renderFlow(context) : renderSequence(context); } });
}(globalThis));
