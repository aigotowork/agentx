(function () {
  "use strict";

  var modelElement = document.getElementById("atlas-model");
  var refs = {
    title: document.getElementById("atlas-title"),
    scope: document.getElementById("atlas-scope"),
    mode: document.getElementById("mode-select"),
    scenario: document.getElementById("scenario-select"),
    map: document.getElementById("atlas-map"),
    mapSummary: document.getElementById("map-summary"),
    inspector: document.getElementById("inspector-content"),
    inspectorContext: document.getElementById("inspector-context"),
    step: document.getElementById("step-content"),
    stepCounter: document.getElementById("step-counter"),
    facts: document.getElementById("facts-content"),
    sourceSummary: document.getElementById("source-summary"),
    previous: document.getElementById("step-prev"),
    next: document.getElementById("step-next"),
    reset: document.getElementById("step-reset"),
    play: document.getElementById("step-play"),
    speed: document.getElementById("speed-select"),
    views: document.getElementById("view-controls")
  };

  var model = readModel();
  var state = {
    modeId: null,
    scenarioId: null,
    view: "relations",
    speed: 0.5,
    stepIndex: -1,
    selectedNodeId: null,
    selectedModuleId: null,
    selectedEdgeId: null,
    selectedPortId: null,
    playing: false
  };
  // Presentation-only time; canonical steps and patches remain the source of facts.
  var animationFrame = null;
  var transition = null;
  var autoplay = false;
  var motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
  var disclosure = { open: false, kind: "object", key: null, opener: null };
  var phaseLabels = { ready: "就绪", flow: "事件演示", handling: "处理", applied: "已应用" };
  var beatLabels = { transfer: "传输", disconnect: "连接断开", local: "本地动作", observe: "观察", retain: "保留运行" };
  var toneLabels = {
    neutral: "待命",
    running: "运行中",
    busy: "处理中",
    waiting: "等待中",
    stopped: "已停止",
    unknown: "未知",
    stored: "已保存"
  };
  var kindLabels = {
    process: "进程",
    group: "进程组",
    "process-group": "进程组",
    store: "存储",
    logical: "逻辑对象",
    actor: "参与方",
    service: "服务"
  };
  var factLabels = Object.create(null);
  array(model.factDefinitions).forEach(function (fact) { factLabels[fact.id] = fact.label; });
  var viewLabels = { relations: "对象、Port 与状态", flow: "场景流程", sequence: "交互时序" };
  var viewScroll = {};
  var lastSequenceFocus = null;
  var layoutUI = null;
  var viewport = null;


  function readModel() {
    try {
      var parsed = JSON.parse(modelElement ? modelElement.textContent : "{}");
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch (error) {
      return { title: "流程全景图", scope: "源模型无法读取", sources: [], nodes: [], edges: [], modes: [], scenarios: [] };
    }
  }

  function array(value) { return Array.isArray(value) ? value : []; }
  function byId(collection, id) { return array(collection).find(function (item) { return item && item.id === id; }) || null; }
  function mode() { return byId(model.modes, state.modeId); }
  function scenario() { return byId(model.scenarios, state.scenarioId); }
  function node(id) { return byId(model.nodes, id); }
  function edge(id) { return byId(model.edges, id); }
  function source(id) { return byId(model.sources, id); }
  function capability(id) { return byId(model.ports, id); }
  function currentCalls() { return FlowAtlasPorts.forStep(model, state.scenarioId, state.stepIndex); }
  function callStatus(call) { return FlowAtlasPorts.status(call, eventPhase(), transition ? transition.beatIndex : -1); }
  function callUI() { return { element: element, row: appendDetailRow, sources: appendSourceList }; }
  function activeCall() {
    var calls = currentCalls();
    if (!calls.length) return null;
    var beatIndex = transition ? transition.beatIndex : -1;
    return calls.slice().reverse().find(function (call) { return call.requestBeat <= beatIndex; }) || calls[0];
  }


  function element(tag, className, text) {
    var item = document.createElement(tag);
    if (className) item.className = className;
    if (text !== undefined && text !== null) item.textContent = String(text);
    return item;
  }

  function svgElement(tag, attributes, text) {
    var item = document.createElementNS("http://www.w3.org/2000/svg", tag);
    Object.keys(attributes || {}).forEach(function (key) { item.setAttribute(key, attributes[key]); });
    if (text !== undefined && text !== null) item.textContent = String(text);
    return item;
  }

  function foldedState(index) {
    return FlowAtlas.fold(model, state.scenarioId, index === undefined ? state.stepIndex : index);
  }

  function visibleState() {
    return transition && transition.phase !== "applied" ? transition.before : foldedState();
  }

  function eventPhase() { return transition ? transition.phase : (currentStep() ? "applied" : "ready"); }

  function currentBeat() {
    return transition && transition.phase === "flow" ? transition.beats[transition.beatIndex] : null;
  }

  // Authored beats own transport meaning and direction. A missing annotation has
  // a local fallback rather than guessing which edge delivered a payload.
  function beatsFor(step) {
    var beats = array(step && step.beats).filter(function (beat) {
      return beat && Object.prototype.hasOwnProperty.call(beatLabels, beat.kind) &&
        (!beat.edgeId || array(step.edgeIds).indexOf(beat.edgeId) >= 0);
    });
    return beats.length ? beats : [{ kind: "local", nodeId: step.actor, label: step.event || step.title }];
  }

  function changesForStep() {
    return (currentStep() ? FlowAtlas.changes(model, state.scenarioId, state.stepIndex) : []).map(function (change) {
      if (change.kind === "node" && change.before === change.after && change.beforeTone !== change.afterTone) {
        return Object.assign({}, change, { before: change.before + " · " + toneLabels[change.beforeTone], after: change.after + " · " + toneLabels[change.afterTone] });
      }
      return change;
    });
  }

  function modeNodeIds() { return array(mode() && mode().nodeIds); }
  function modeEdgeIds() { return array(mode() && mode().edgeIds); }
  function currentStep() {
    var currentScenario = scenario();
    return currentScenario && state.stepIndex >= 0 ? array(currentScenario.steps)[state.stepIndex] || null : null;
  }

  function stateForNode(id, folded) {
    var value = folded.nodes[id];
    return value || { label: "未提供", tone: "unknown" };
  }

  function labelFor(id) {
    var item = node(id);
    return item ? item.label : (id || "未指明");
  }

  function modeScenarios(modeId) {
    return array(model.scenarios).filter(function (item) { return item.modeId === modeId; });
  }

  function defaultScenarioFor(modeId) {
    var items = modeScenarios(modeId);
    return byId(items, model.defaults && model.defaults.scenarioId) || items[0] || null;
  }

  function setOptions(select, items, selectedId, label) {
    select.replaceChildren();
    items.forEach(function (item) {
      var option = element("option", "", item.label || item.id);
      option.value = item.id;
      option.selected = item.id === selectedId;
      select.appendChild(option);
    });
    select.setAttribute("aria-label", label);
  }

  function readHash() {
    stopPlayback();
    var params = new URLSearchParams(window.location.hash.replace(/^#/, ""));
    var requestedMode = byId(model.modes, params.get("mode"));
    state.modeId = requestedMode ? requestedMode.id : FlowAtlas.defaultSelection(model).modeId;
    var requestedScenario = byId(modeScenarios(state.modeId), params.get("scenario"));
    state.scenarioId = requestedScenario ? requestedScenario.id : (defaultScenarioFor(state.modeId) || {}).id;
    state.view = Object.prototype.hasOwnProperty.call(viewLabels, params.get("view")) ? params.get("view") : "relations";
    var requestedSpeed = Number(params.get("speed"));
    state.speed = [0.25, 0.5, 1, 2].indexOf(requestedSpeed) >= 0 ? requestedSpeed : 0.5;
    var currentScenario = scenario();
    var requestedStep = Number.parseInt(params.get("step"), 10);
    state.stepIndex = Number.isFinite(requestedStep) && requestedStep >= -1 && requestedStep < array(currentScenario && currentScenario.steps).length ? requestedStep : -1;
    var currentMode = mode();
    state.selectedNodeId = modeNodeIds().indexOf(params.get("node")) >= 0 ? params.get("node") : (modeNodeIds(currentMode)[0] || null);
    state.selectedModuleId = null;
    state.selectedEdgeId = null;
    state.selectedPortId = null;
  }

  function writeHash() {
    var params = new URLSearchParams();
    if (state.modeId) params.set("mode", state.modeId);
    if (state.scenarioId) params.set("scenario", state.scenarioId);
    params.set("step", String(state.stepIndex));
    params.set("view", state.view);
    params.set("speed", String(state.speed));
    if (state.selectedNodeId) params.set("node", state.selectedNodeId);
    window.history.replaceState(null, "", "#" + params.toString());
  }

  function setMode(nextId) {
    var nextMode = byId(model.modes, nextId);
    if (!nextMode) return;
    stopPlayback();
    closeDisclosure(false);
    state.modeId = nextMode.id;
    state.scenarioId = (defaultScenarioFor(nextMode.id) || {}).id || null;
    state.stepIndex = -1;
    state.selectedNodeId = modeNodeIds()[0] || null;
    state.selectedModuleId = null;
    state.selectedEdgeId = null;
    state.selectedPortId = null;
    writeHash();
    render();
  }

  function setScenario(nextId) {
    var nextScenario = byId(modeScenarios(state.modeId), nextId);
    if (!nextScenario) return;
    stopPlayback();
    closeDisclosure(false);
    state.scenarioId = nextScenario.id;
    state.stepIndex = -1;
    state.selectedNodeId = modeNodeIds()[0] || null;
    state.selectedModuleId = null;
    state.selectedEdgeId = null;
    state.selectedPortId = null;
    writeHash();
    render();
  }

  function setStep(nextIndex, preserveSelection, animate, continuous) {
    stopPlayback();
    var currentScenario = scenario();
    var steps = array(currentScenario && currentScenario.steps);
    state.stepIndex = Math.max(-1, Math.min(nextIndex, steps.length - 1));
    if (!preserveSelection) {
      var step = currentStep();
      if (step && modeNodeIds().indexOf(step.actor) >= 0) state.selectedNodeId = step.actor;
      state.selectedModuleId = null;
      state.selectedEdgeId = null;
    }
    if (animate && currentStep()) {
      transition = {
        phase: "flow", beats: beatsFor(currentStep()), beatIndex: 0,
        elapsed: 0, stamp: null, before: foldedState(state.stepIndex - 1)
      };
      state.playing = true;
      autoplay = Boolean(continuous);
    }
    writeHash();
    render();
    scheduleFrame();
  }

  // Cancellation discards the only owned frame and all obsolete staged state.
  function stopPlayback() {
    if (layoutUI) layoutUI.cancel(false);
    if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
    animationFrame = null;
    transition = null;
    autoplay = false;
    state.playing = false;
    updatePlayButton();
  }

  function pausePlayback() {
    if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
    animationFrame = null;
    state.playing = false;
    if (transition) transition.stamp = null;
    renderStep();
    updatePlayButton();
  }

  function togglePlayback() {
    if (layoutUI) layoutUI.cancel();
    var steps = array(scenario() && scenario().steps);
    if (!steps.length) return;
    if (state.playing) {
      pausePlayback();
      return;
    }
    if (!transition || (transition.phase === "applied" && !autoplay)) {
      setStep(state.stepIndex >= steps.length - 1 ? 0 : state.stepIndex + 1, false, true, true);
      return;
    }
    state.playing = true;
    transition.stamp = null;
    renderStep();
    updatePlayButton();
    scheduleFrame();
  }

  function phaseDuration() {
    if (!transition) return 0;
    if (transition.phase === "flow") return currentBeat().kind === "transfer" ? 1200 : 1000;
    return transition.phase === "handling" ? 900 : 1400;
  }

  function scheduleFrame() {
    if (state.playing && transition && animationFrame === null) animationFrame = window.requestAnimationFrame(tick);
  }

  function tick(stamp) {
    animationFrame = null;
    if (!state.playing || !transition) return;
    if (transition.stamp !== null) transition.elapsed += (stamp - transition.stamp) * state.speed;
    transition.stamp = stamp;
    if (transition.elapsed >= phaseDuration()) {
      transition.elapsed = 0;
      if (transition.phase === "flow") {
        if (transition.beatIndex + 1 < transition.beats.length) transition.beatIndex += 1;
        else transition.phase = "handling";
      } else if (transition.phase === "handling") {
        transition.phase = "applied";
        if (!autoplay) state.playing = false;
      } else {
        var steps = array(scenario() && scenario().steps);
        if (autoplay && state.stepIndex + 1 < steps.length) {
          setStep(state.stepIndex + 1, false, true, true);
          return;
        }
        state.playing = false;
        autoplay = false;
      }
      render();
    }
    updateFlowMarker();
    scheduleFrame();
  }

  function updatePlayButton() {
    if (!refs.play) return;
    refs.play.setAttribute("aria-pressed", String(state.playing));
    refs.play.querySelector(".play-button__icon").textContent = state.playing ? "Ⅱ" : "▶";
    var resumable = transition && transition.phase !== "applied";
    refs.play.querySelector(".play-button__label").textContent = state.playing ? "暂停" : (resumable ? "继续" : "播放");
    refs.play.setAttribute("aria-label", state.playing ? "暂停行为演示" : (resumable ? "继续行为演示" : "自动播放步骤"));
  }

  function layoutForMode(modeId) {
    var current = byId(model.modes, modeId);
    // Bounded generic grid; optional model hints refine complex domain topology.
    var nodes = {};
    var ids = array(current && current.nodeIds);
    var columns = Math.min(3, ids.length);
    var rowHeights = [];
    ids.forEach(function (id, index) {
      var row = Math.floor(index / columns);
      rowHeights[row] = Math.max(rowHeights[row] || 0, nodeBox({ x: 0, y: 0 }, node(id)).height);
    });
    ids.forEach(function (id, index) {
      var row = Math.floor(index / columns);
      var y = 42 + rowHeights.slice(0, row).reduce(function (sum, height) { return sum + height + 90; }, 0);
      nodes[id] = { x: 24 + (index % columns) * 290, y: y };
    });
    var hints = current && current.layout || {};
    Object.keys(hints.nodes || {}).forEach(function (id) { nodes[id] = hints.nodes[id]; });
    var bounds = ids.map(function (id) { return nodeBox(nodes[id], node(id)); });
    var width = Math.max.apply(null, bounds.map(function (box) { return box.x + box.width + 16; }));
    var height = Math.max.apply(null, bounds.map(function (box) { return box.y + box.height + 10; }));
    return { width: Math.max(hints.width || 660, width), height: Math.max(hints.height || 320, height), nodes: nodes, routes: hints.routes || {} };
  }

  function nodeBox(position, currentNode) {
    return FlowAtlasCanvas.box(model, currentNode, position);
  }

  // Coordinates and label shortening are private view data; source facts remain unchanged.
  function shortMapText(value, maxUnits) {
    var text = String(value || "");
    var chars = Array.from(text);
    var units = chars.reduce(function (sum, char) { return sum + (/[^\x00-\x7f]/.test(char) ? 2 : 1); }, 0);
    if (units <= maxUnits) return text;
    var result = "";
    var used = 0;
    for (var i = 0; i < chars.length; i += 1) {
      var size = /[^\x00-\x7f]/.test(chars[i]) ? 2 : 1;
      if (used + size > maxUnits - 2) break;
      result += chars[i];
      used += size;
    }
    return result + "…";
  }

  function port(box, side, ratio) {
    var fraction = ratio === undefined ? 0.5 : ratio;
    if (side === "top" || side === "bottom") return [box.x + box.width * fraction, box.y + (side === "bottom" ? box.height : 0)];
    return [box.x + (side === "right" ? box.width : 0), box.y + box.height * fraction];
  }

  function edgeGeometry(currentEdge, from, to, layout) {
    var route = (layout.routes || {})[currentEdge.id];
    if (!route) {
      var horizontal = Math.abs(to.x - from.x) > Math.abs(to.y - from.y);
      var startSide = horizontal ? (to.x > from.x ? "right" : "left") : (to.y > from.y ? "bottom" : "top");
      var endSide = horizontal ? (to.x > from.x ? "left" : "right") : (to.y > from.y ? "top" : "bottom");
      var origin = port(from, startSide);
      var destination = port(to, endSide);
      var midpoint = [(origin[0] + destination[0]) / 2, (origin[1] + destination[1]) / 2];
      var gutter = [(origin[0] + destination[0]) / 2, Math.min(from.y, to.y) - 22];
      route = horizontal ? { start: startSide, end: endSide, via: [[gutter[0] - 16, origin[1]], [gutter[0] - 16, gutter[1]], [gutter[0] + 16, gutter[1]], [gutter[0] + 16, destination[1]]], label: gutter } : { start: startSide, end: endSide, via: [], label: midpoint };

    }
    var points = [port(from, route.start, route.startRatio)].concat(route.via, [port(to, route.end, route.endRatio)]);
    return { path: points.map(function (point, index) { return (index ? "L " : "M ") + point[0] + " " + point[1]; }).join(" "), label: route.label, points: points.map(function (p) { return { x: p[0], y: p[1] }; }) };
  }

  function drawNode(svg, currentNode, position, folded, selectedModuleId, activeActor, anchors) {
    var box = nodeBox(position, currentNode);
    var currentState = stateForNode(currentNode.id, folded);
    var step = currentStep();
    var isActor = Boolean(step && step.actor === currentNode.id);
    var isHandler = Boolean(step && step.handler === currentNode.id);
    var changed = eventPhase() === "applied" && changesForStep().some(function (change) { return change.kind === "node" && change.id === currentNode.id; });
    var isPinned = layoutUI && layoutUI.pinned(currentNode.id);
    var group = svgElement("g", {
      class: "map-node map-node--" + (currentNode.kind || "process") + (state.selectedNodeId === currentNode.id ? " is-selected" : "") + (activeActor === currentNode.id ? " is-active" : "") + (isHandler ? " is-handler" : "") + (changed ? " is-changed" : ""),
      "data-node-id": currentNode.id,
      "data-event-role": isActor && isHandler ? "actor-handler" : (isActor ? "actor" : (isHandler ? "handler" : "")),
      "data-testid": "map-node-" + currentNode.id,
      "data-layout-pinned": String(Boolean(isPinned)),
      tabindex: "0",
      role: "button",
      "aria-label": currentNode.label + "，" + (kindLabels[currentNode.kind] || currentNode.kind || "对象") + "，" + currentState.label
    });
    group.appendChild(svgElement("title", {}, currentNode.label + " · " + currentState.label));
    if (isActor || isHandler) group.appendChild(svgElement("text", { class: "map-node__role", x: box.x + 12, y: box.y - 7 }, isActor && isHandler ? "动作 / 处理" : (isActor ? "动作方" : "处理方")));
    group.appendChild(svgElement("rect", { class: "map-node__body", x: box.x, y: box.y, width: box.width, height: box.height, rx: 10 }));
    var heading = svgElement("g", { class: "layout-handle" + (isPinned ? " is-pinned" : ""), "data-layout-handle": currentNode.id, "data-testid": "layout-handle-" + currentNode.id, tabindex: 0, role: "button", "aria-keyshortcuts": "ArrowUp ArrowDown ArrowLeft ArrowRight P", "aria-label": "移动 " + currentNode.label + (isPinned ? "，已固定" : "") + "；方向键移动 10 单位，Shift 加方向键 1 单位，P 固定或取消固定，Escape 取消拖动" });
    heading.appendChild(svgElement("title", {}, "拖动标题移动 · 方向键 10 / Shift 1 · P 固定 · Esc 取消"));
    heading.appendChild(svgElement("rect", { class: "map-node__head", x: box.x + 1, y: box.y + 1, width: box.width - 2, height: 46, rx: 9 }));
    heading.appendChild(svgElement("circle", { class: "map-node__state-dot state-dot--" + (currentState.tone || "unknown"), cx: box.x + 15, cy: box.y + 17, r: 4 }));
    heading.appendChild(svgElement("text", { class: "map-node__title", x: box.x + 27, y: box.y + 21 }, shortMapText(currentNode.label, 24)));
    heading.appendChild(svgElement("text", { class: "layout-grip", x: box.x + box.width - 17, y: box.y + 19, "text-anchor": "middle", "aria-hidden": "true" }, isPinned ? "◆" : "⠿"));
    var kind = kindLabels[currentNode.kind] || currentNode.kind || "对象";
    heading.appendChild(svgElement("text", { class: "map-node__kind", x: box.x + 12, y: box.y + 39 }, kind));
    var stateOffset = kind.length > 2 ? 66 : 44;
    var objectPorts = FlowAtlasPorts.owned(model, currentNode.id);
    var stateUnits = objectPorts.length ? (kind.length > 2 ? 12 : 17) : (kind.length > 2 ? 20 : 24);
    heading.appendChild(svgElement("text", { class: "map-node__state", x: box.x + stateOffset, y: box.y + 39 }, shortMapText(currentState.label || toneLabels[currentState.tone] || "未提供", stateUnits)));
    group.appendChild(heading);
    (objectPorts.length ? [] : array(currentNode.modules)).forEach(function (module, index) {
      var moduleY = box.y + 52 + array(currentNode.stateFacts).length * 36 + index * 22;
      var moduleGroup = svgElement("g", {
        class: "map-node__module" + (selectedModuleId === module.id ? " is-selected" : ""),
        "data-module-id": module.id,
        "data-node-id": currentNode.id,
        "data-testid": "module-" + module.id,
        tabindex: "0",
        role: "button",
        "aria-label": module.label + "，" + module.path
      });
      moduleGroup.appendChild(svgElement("title", {}, module.label + " · " + module.path));
      moduleGroup.appendChild(svgElement("rect", { class: "map-node__module-bg", x: box.x + 10, y: moduleY, width: box.width - 20, height: 19, rx: 5 }));
      moduleGroup.appendChild(svgElement("text", { class: "map-node__module-label", x: box.x + 16, y: moduleY + 13 }, shortMapText(module.label, 26)));
      group.appendChild(moduleGroup);
    });
    FlowAtlasCanvas.drawNode({ model: model, svg: svgElement, shorten: shortMapText, snapshot: folded, changes: changesForStep(), phase: eventPhase(), group: group, activePortId: activeCall() && activeCall().port.id }, currentNode, box, anchors);
    if (objectPorts.length) {
      var active = activeCall();
      var currentPort = active && active.target.id === currentNode.id ? active.port : null;
      var portGroup = svgElement("g", { class: "map-port" + (currentPort ? " is-active" : ""), "data-node-ports": currentNode.id, "data-testid": "map-ports-" + currentNode.id, tabindex: 0, role: "button", "aria-label": currentNode.label + " · " + objectPorts.length + " 个 Port" });
      var x = box.x + box.width - 48;
      var y = box.y + 26;
      portGroup.appendChild(svgElement("rect", { class: "map-port__body", x: x, y: y, width: 40, height: 24, rx: 5 }));
      portGroup.appendChild(svgElement("text", { class: "map-port__label", x: x + 6, y: y + 15 }, "◇ " + objectPorts.length));
      portGroup.appendChild(svgElement("title", {}, objectPorts.map(function (item) { return item.label; }).join(" · ")));
      group.appendChild(portGroup);
    }
    svg.appendChild(group);
    return box;
  }

  function drawEdge(svg, currentEdge, boxes, activeEdges, contextual, layout) {
    var fromBox = boxes[currentEdge.from];
    var toBox = boxes[currentEdge.to];
    if (!fromBox || !toBox) return FlowAtlasLayout.routeDiagnostics("edge:" + currentEdge.id, null, boxes);
    var active = activeEdges.indexOf(currentEdge.id) >= 0;
    var selected = state.selectedEdgeId === currentEdge.id;
    var broken = brokenEdgeIds().indexOf(currentEdge.id) >= 0;
    var group = svgElement("g", { class: "map-edge" + (active ? " is-active" : "") + (selected ? " is-selected" : "") + (broken ? " is-broken" : ""), "data-edge-id": currentEdge.id, "data-testid": "map-edge-" + currentEdge.id, tabindex: "0", role: "button", "aria-label": currentEdge.label + "，" + currentEdge.transport + (broken ? "，连接断开示意" : "") });
    if (contextual) group.classList.add("is-context");
    var geometry = edgeGeometry(currentEdge, fromBox, toBox, layout);
    var issues = FlowAtlasLayout.routeDiagnostics("edge:" + currentEdge.id, geometry.points, boxes);
    group.setAttribute("data-route-status", issues.length ? issues[0].code : "clear");
    if (issues.length) group.classList.add("has-layout-diagnostic");
    group.appendChild(svgElement("title", {}, currentEdge.label + " · " + currentEdge.transport + (issues.length ? " · " + issues[0].message : "")));
    group.appendChild(svgElement("path", { class: "map-edge__hit", d: geometry.path }));
    group.appendChild(svgElement("path", { class: "map-edge__line", d: geometry.path }));
    var use = transition ? FlowAtlasPorts.atBeat(currentCalls(), transition.beatIndex) : null;
    var operation = use && currentBeat() && currentBeat().edgeId === currentEdge.id ? FlowAtlasPorts.beatLabel(currentCalls(), transition.beatIndex, eventPhase()) : null;
    var mapLabel = shortMapText(operation || currentEdge.transport || currentEdge.label, 26);
    var labelWidth = Math.max(48, Array.from(mapLabel).reduce(function (sum, char) { return sum + (/[^\x00-\x7f]/.test(char) ? 11 : 6); }, 16));
    if (!contextual) {
      group.appendChild(svgElement("rect", { class: "map-edge__label-bg", x: geometry.label[0] - labelWidth / 2, y: geometry.label[1] - 11, width: labelWidth, height: 18, rx: 5 }));
      group.appendChild(svgElement("text", { class: "map-edge__label", x: geometry.label[0], y: geometry.label[1] + 1, "text-anchor": "middle" }, mapLabel));
    }
    svg.appendChild(group);
    return issues;
  }

  function brokenEdgeIds() {
    var step = currentStep();
    if (!step) return [];
    var beats = transition ? transition.beats : beatsFor(step);
    var visibleBeats = transition && transition.phase === "flow" ? beats.slice(0, transition.beatIndex + 1) : beats;
    return visibleBeats.filter(function (beat) { return beat.kind === "disconnect" && beat.edgeId; }).map(function (beat) { return beat.edgeId; });
  }

  function flowCaption(group, label, kind) {
    var text = shortMapText(label, 32);
    var width = Math.max(86, Array.from(text).reduce(function (sum, char) { return sum + (/[^\x00-\x7f]/.test(char) ? 12 : 7); }, 20));
    group.appendChild(svgElement("rect", { class: "event-flow__caption-bg", x: -width / 2, y: -35, width: width, height: 23, rx: 6 }));
    group.appendChild(svgElement("text", { class: "event-flow__caption", x: 0, y: -19, "text-anchor": "middle" }, text));
    group.appendChild(svgElement("title", {}, beatLabels[kind] + " · " + label + "（行为演示）"));
  }

  function positionFlow(group, point) {
    group.setAttribute("transform", "translate(" + point.x + " " + point.y + ")");
    var background = group.querySelector(".event-flow__caption-bg");
    var caption = group.querySelector(".event-flow__caption");
    if (background) background.setAttribute("y", point.y < 42 ? 14 : -35);
    if (caption) caption.setAttribute("y", point.y < 42 ? 30 : -19);
    var payload = group.querySelector(".canvas-payload");
    if (payload) {
      var height = Number(payload.getAttribute("data-caption-height"));
      payload.setAttribute("transform", "translate(0 " + (point.y < height + 14 ? 14 : -height - 14) + ")");
    }
  }

  function drawFlow(svg, boxes) {
    // Persistent break glyphs are not packets and never claim a delivered event.
    brokenEdgeIds().forEach(function (id) {
      var path = svg.querySelector('[data-edge-id="' + id + '"] .map-edge__line');
      if (!path) return;
      var point = path.getPointAtLength(path.getTotalLength() * 0.5);
      var cut = svgElement("g", { class: "event-break", transform: "translate(" + point.x + " " + point.y + ")", "data-testid": "flow-break", "aria-label": "连接断开示意" });
      cut.appendChild(svgElement("circle", { r: 13, class: "event-break__background" }));
      cut.appendChild(svgElement("path", { d: "M -6 -6 L 6 6 M -6 6 L 6 -6", class: "event-break__cross" }));
      svg.appendChild(cut);
    });
    var beat = currentBeat();
    var step = currentStep();
    if (!step || eventPhase() === "applied" || eventPhase() === "ready") return;
    if (!beat) beat = { kind: "local", nodeId: step.handler, label: "处理 · " + labelFor(step.handler) };
    var group = svgElement("g", { class: "event-flow event-flow--" + beat.kind, "aria-label": beat.label + "（行为演示）", "pointer-events": "none" });
    var cuePosition = null;
    if (beat.kind === "transfer" && beat.edgeId) {
      group.setAttribute("data-testid", "flow-marker");
      group.appendChild(svgElement("circle", { class: "event-flow__halo", r: 14 }));
      group.appendChild(svgElement("circle", { class: "event-flow__packet", r: 7 }));
      group.appendChild(svgElement("path", { class: "event-flow__arrow", d: "M -2 -3 L 2 0 L -2 3" }));
    } else {
      group.setAttribute("data-testid", "flow-cue");
      var box = boxes[beat.nodeId || step.handler || step.actor];
      if (beat.edgeId) {
        var path = svg.querySelector('[data-edge-id="' + beat.edgeId + '"] .map-edge__line');
        if (path) {
          var point = path.getPointAtLength(path.getTotalLength() * 0.5);
          cuePosition = point;
        }
      } else if (box) cuePosition = { x: box.x + box.width - 16, y: box.y + 17 };
      group.appendChild(svgElement("circle", { class: "event-flow__local-ring", r: 14 }));
      var glyph = { disconnect: "×", local: "⚙", observe: "◎", retain: "Ⅱ" }[beat.kind] || "·";
      group.appendChild(svgElement("text", { class: "event-flow__glyph", x: 0, y: 5, "text-anchor": "middle" }, glyph));
    }
    var use = transition && FlowAtlasPorts.atBeat(currentCalls(), transition.beatIndex);
    var caption = use && beat.kind === "transfer" ? FlowAtlasCanvas.payload(use.call, use.response) : (transition && FlowAtlasPorts.beatLabel(currentCalls(), transition.beatIndex, eventPhase()) || beat.label);
    group.setAttribute("data-payload", caption);
    if (use) group.setAttribute("data-port-id", use.call.port.id);
    if (use && beat.kind === "transfer") FlowAtlasCanvas.caption({ svg: svgElement, shorten: shortMapText }, group, use.call, use.response);
    else flowCaption(group, caption, beat.kind);
    if (cuePosition) positionFlow(group, cuePosition);
    svg.appendChild(group);
  }

  function updateFlowMarker() {
    var beat = currentBeat();
    var marker = refs.map.querySelector('[data-testid="flow-marker"]');
    if (!beat || !marker || !beat.edgeId) return;
    var use = FlowAtlasPorts.atBeat(currentCalls(), transition.beatIndex);
    var callPath = state.view === "relations" && use ? refs.map.querySelector('[data-call-link="' + state.stepIndex + ':' + use.call.id + '"] .canvas-call__line') : null;
    var direction = callPath ? (use.response ? "reverse" : "forward") : beat.direction;
    var selector = state.view === "sequence" ? '[data-trace-step="' + state.stepIndex + '"][data-trace-beat="' + transition.beatIndex + '"] .map-edge__line' : '[data-edge-id="' + beat.edgeId + '"] .map-edge__line';
    var path = callPath || refs.map.querySelector(selector);
    if (!path) return;
    var progress = motionPreference.matches ? 0.5 : Math.max(0.02, Math.min(0.98, transition.elapsed / phaseDuration()));
    if (direction === "reverse") progress = 1 - progress;
    var distance = path.getTotalLength() * progress;
    var point = path.getPointAtLength(distance);
    var toward = path.getPointAtLength(distance + (direction === "reverse" ? -1 : 1));
    var angle = Math.atan2(toward.y - point.y, toward.x - point.x) * 180 / Math.PI;
    positionFlow(marker, point);
    marker.querySelector(".event-flow__arrow").setAttribute("transform", "rotate(" + angle + ")");
    marker.setAttribute("data-progress", String(transition.elapsed / phaseDuration()));
    marker.setAttribute("data-direction", beat.direction || "forward");
    marker.setAttribute("data-path-direction", direction || "forward");
    marker.setAttribute("data-edge-id", beat.edgeId);
  }

  function renderRelations() {
    var currentMode = mode();
    var currentScenario = scenario();
    var folded = visibleState();
    var base = layoutForMode(state.modeId);
    var sizes = {};
    modeNodeIds().forEach(function (id) { sizes[id] = nodeBox({ x: 0, y: 0 }, node(id)); });
    var layout = layoutUI ? layoutUI.project(base, sizes, model.edges) : Object.assign({ x: 0, y: 0 }, base);
    var positions = layout.nodes;
    var svg = svgElement("svg", { viewBox: layout.x + " " + layout.y + " " + layout.width + " " + layout.height, width: layout.width, height: layout.height, role: "group", "aria-label": (currentMode && currentMode.label || "运行") + "关系图" });
    var defs = svgElement("defs");
    var marker = svgElement("marker", { id: "edge-arrow", viewBox: "0 0 10 10", refX: "9", refY: "5", markerWidth: "6", markerHeight: "6", orient: "auto-start-reverse" });
    marker.appendChild(svgElement("path", { d: "M 0 0 L 10 5 L 0 10 z", fill: "#b7c5d8" }));
    defs.appendChild(marker);
    var callArrow = svgElement("marker", { id: "canvas-call-arrow", viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 6, markerHeight: 6, orient: "auto-start-reverse" });
    callArrow.appendChild(svgElement("path", { d: "M 0 0 L 10 5 L 0 10 z", class: "canvas-call__arrow" }));
    defs.appendChild(callArrow);
    svg.appendChild(defs);
    var boxes = {};
    modeNodeIds().forEach(function (id) {
      var currentNode = node(id);
      if (currentNode) boxes[id] = nodeBox(positions[id] || { x: 24, y: 24 }, currentNode);
    });
    var beat = currentBeat();
    var calls = [];
    array(currentScenario && currentScenario.steps).forEach(function (_, index) {
      FlowAtlas.calls(model, state.scenarioId, index).forEach(function (call) { calls.push({ call: call, stepIndex: index }); });
    });
    var anchors = FlowAtlasCanvas.pins(model, currentMode, layout, boxes);
    var callEdges = calls.map(function (entry) { return beatsFor(currentScenario.steps[entry.stepIndex])[entry.call.requestBeat].edgeId; });
    var activeEdges = beat && beat.edgeId ? [beat.edgeId] : (eventPhase() === "applied" ? array(currentStep() && currentStep().edgeIds) : []);
    var diagnostics = FlowAtlasLayout.overlaps(boxes);
    modeEdgeIds().forEach(function (id) {
      var currentEdge = edge(id);
      if (currentEdge) diagnostics = diagnostics.concat(drawEdge(svg, currentEdge, boxes, activeEdges, callEdges.indexOf(id) >= 0 && (!beat || beat.kind !== "disconnect" || beat.edgeId !== id), layout));
    });
    diagnostics = diagnostics.concat(FlowAtlasCanvas.drawCalls({ svg: svgElement, currentCalls: currentCalls(), beatIndex: transition ? transition.beatIndex : -1, stepIndex: state.stepIndex, routeDiagnostics: FlowAtlasLayout.routeDiagnostics }, svg, calls, boxes, anchors));
    var activeActor = beat && beat.nodeId || (currentStep() && (eventPhase() === "handling" ? currentStep().handler : currentStep().actor));
    modeNodeIds().forEach(function (id) {
      var currentNode = node(id);
      if (currentNode) drawNode(svg, currentNode, positions[id] || { x: 24, y: 24 }, folded, state.selectedModuleId, activeActor, anchors);
    });
    diagnostics.filter(function (item) { return item.code === "node-overlap"; }).forEach(function (item) {
      item.nodeIds.forEach(function (id) {
        var target = svg.querySelector('[data-node-id="' + id + '"]');
        if (target) { target.classList.add("has-layout-diagnostic"); target.setAttribute("data-layout-diagnostic", "node-overlap"); }
      });
    });
    refs.map.style.setProperty("--diagram-width", layout.width + "px");
    refs.map.replaceChildren(svg);
    drawFlow(svg, boxes);
    if (viewport) viewport.mount(svg, [state.view, state.modeId, state.scenarioId].join("/"));
    updateFlowMarker();
    refs.map.setAttribute("aria-label", (currentMode && currentMode.label || "运行") + "关系图，" + (currentScenario && currentScenario.label || "场景"));
    if (layoutUI) layoutUI.diagnostics(diagnostics);
  }

  function renderMap() {
    refs.map.setAttribute("data-view", state.view);
    if (state.view === "relations") renderRelations();
    else {
      var projection = FlowAtlasViews.render(state.view, {
        model: model, mode: mode(), scenario: scenario(), stepIndex: state.stepIndex, viewportWidth: refs.map.clientWidth,
        phase: eventPhase(), beatIndex: transition ? transition.beatIndex : -1,
        beat: currentBeat(), selectedNodeId: state.selectedNodeId,
        snapshot: visibleState(), svg: svgElement, shorten: shortMapText,
        node: node, edge: edge, label: labelFor, beats: beatsFor,
        calls: function (index) { return FlowAtlasPorts.forStep(model, state.scenarioId, index); },
        changes: function (index) { return FlowAtlas.changes(model, state.scenarioId, index); }
      });
      refs.map.style.setProperty("--diagram-width", projection.width + "px");
      refs.map.replaceChildren(projection.svg);
      if (viewport) viewport.mount(projection.svg, [state.view, state.modeId, state.scenarioId].join("/"));
      refs.map.setAttribute("aria-label", viewLabels[state.view]);
      updateFlowMarker();
      revealSequenceBeat();
    }
    updateSequenceHeader();
    renderViewControls();
    if (layoutUI) layoutUI.refresh();
  }

  function updateSequenceHeader() {
    if (viewport) return;
    var header = refs.map.querySelector('[data-testid="sequence-header"]');
    if (header) header.setAttribute("transform", "translate(0 " + refs.map.scrollTop + ")");
  }

  function revealSequenceBeat() {
    if (state.view !== "sequence" || !transition || transition.phase !== "flow") return;
    var key = state.scenarioId + "/" + state.stepIndex + "/" + transition.beatIndex;
    if (key === lastSequenceFocus) return;
    lastSequenceFocus = key;
    var row = refs.map.querySelector('[data-trace-step="' + state.stepIndex + '"][data-trace-beat="' + transition.beatIndex + '"]');
    if (!row) return;
    if (viewport) { viewport.reveal(row); return; }
    var item = row.getBoundingClientRect(), region = refs.map.getBoundingClientRect();
    if (item.bottom > region.bottom || item.top < region.top + 76) refs.map.scrollTop += item.top - region.top - 96;
    var line = row.querySelector(".map-edge__line") || row.querySelector(".sequence-local");
    if (line) {
      var content = line.getBoundingClientRect();
      if (content.right < region.left || content.left > region.right) refs.map.scrollLeft += (content.left + content.right) / 2 - (region.left + region.right) / 2;
    }
  }

  function renderViewControls() {
    refs.views.querySelectorAll("[data-view]").forEach(function (button) {
      button.setAttribute("aria-pressed", String(button.getAttribute("data-view") === state.view));
    });
    document.getElementById("map-heading").textContent = viewLabels[state.view];
    var legend = document.querySelector(".map-legend");
    legend.hidden = state.view !== "relations";
    legend.replaceChildren();
    var kinds = [];
    modeNodeIds().forEach(function (id) { if (kinds.indexOf(node(id).kind) < 0) kinds.push(node(id).kind); });
    kinds.forEach(function (kind) {
      var item = element("span"), swatch = element("i", "legend-swatch legend-swatch--" + (kind === "store" || kind === "logical" ? kind : "process"));
      swatch.setAttribute("aria-hidden", "true");
      item.appendChild(swatch);
      item.appendChild(document.createTextNode(kindLabels[kind] || kind));
      legend.appendChild(item);
    });
    refs.speed.value = String(state.speed);
  }

  function setView(view) {
    if (!Object.prototype.hasOwnProperty.call(viewLabels, view) || view === state.view) return;
    if (layoutUI) layoutUI.cancel(false);
    viewScroll[state.view] = { left: refs.map.scrollLeft, top: refs.map.scrollTop };
    state.view = view;
    writeHash();
    lastSequenceFocus = null;
    renderMap();
    var scroll = viewScroll[view] || { left: 0, top: 0 };
    refs.map.scrollLeft = scroll.left;
    refs.map.scrollTop = scroll.top;
    lastSequenceFocus = null;
    revealSequenceBeat();
    updateSequenceHeader();
  }

  function setSpeed(speed) {
    if ([0.25, 0.5, 1, 2].indexOf(speed) < 0) return;
    if (transition && state.playing && transition.stamp !== null) {
      var now = performance.now();
      transition.elapsed += (now - transition.stamp) * state.speed;
      transition.stamp = now;
    }
    state.speed = speed;
    writeHash();
    updateFlowMarker();
  }

  function appendDetailRow(parent, label, value, className) {
    var row = element("div", "detail-row");
    row.appendChild(element("dt", "", label));
    row.appendChild(element("dd", className || "", value || "未提供"));
    parent.appendChild(row);
  }

  function appendSourceList(parent, sourceIds) {
    var sources = [];
    array(sourceIds).forEach(function (id) {
      var item = source(id);
      if (item && sources.every(function (current) { return current.id !== item.id; })) sources.push(item);
    });
    var section = element("section", "inspector-section");
    section.appendChild(element("h4", "", "源码依据"));
    var list = element("ul", "source-list");
    if (!sources.length) list.appendChild(element("li", "source-item__basis", "模型未提供来源"));
    sources.forEach(function (item) {
      var row = element("li", "source-item");
      var main = element("div", "source-item__main");
      main.appendChild(element("span", "source-item__label", item.label || item.id));
      main.appendChild(element("span", "source-path", item.path || "未提供路径"));
      main.appendChild(element("span", "source-item__basis", item.basis || "source"));
      var copy = element("button", "copy-button", "复制");
      copy.type = "button";
      copy.setAttribute("data-testid", "source-copy-" + item.id);
      copy.setAttribute("aria-label", "复制 " + (item.path || "源码路径"));
      copy.addEventListener("click", function () { copyPath(item.path || "", copy); });
      row.appendChild(main);
      row.appendChild(copy);
      list.appendChild(row);
    });
    section.appendChild(list);
    parent.appendChild(section);
  }

  function copyPath(path, button) {
    if (!path) return;
    var done = function (success) {
      button.textContent = success ? "已复制" : "复制失败";
      window.setTimeout(function () { button.textContent = "复制"; }, 1400);
    };
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(path).then(function () { done(true); }, function () { fallbackCopy(path, done); });
      return;
    }
    fallbackCopy(path, done);
  }

  function fallbackCopy(path, done) {
    var textarea = element("textarea");
    textarea.value = path;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    var success = false;
    try { success = document.execCommand("copy"); } catch (error) { success = false; }
    document.body.removeChild(textarea);
    done(success);
  }

  function renderNodeInspector(currentNode, folded) {
    var currentState = stateForNode(currentNode.id, folded);
    var wrapper = element("div");
    var title = element("div", "object-title");
    var titleInfo = element("div");
    titleInfo.appendChild(element("h3", "", currentNode.label));
    titleInfo.appendChild(element("div", "object-kind", kindLabels[currentNode.kind] || currentNode.kind || "对象"));
    title.appendChild(titleInfo);
    title.appendChild(element("span", "tone-badge tone-badge--" + (currentState.tone || "unknown"), currentState.label || toneLabels[currentState.tone] || "未提供"));
    wrapper.appendChild(title);
    var details = element("dl", "detail-list");
    appendDetailRow(details, "所有者", currentNode.owner);
    appendDetailRow(details, "可执行", currentNode.executable, "mono");
    appendDetailRow(details, "生命周期", currentNode.lifetime);
    wrapper.appendChild(details);
    if (array(currentNode.capabilities).length) {
      var capabilities = element("section", "inspector-section");
      capabilities.appendChild(element("h4", "", "能力"));
      var list = element("ul", "capability-list");
      currentNode.capabilities.forEach(function (item) { list.appendChild(element("li", "", item)); });
      capabilities.appendChild(list);
      wrapper.appendChild(capabilities);
    }
    if (array(currentNode.modules).length) {
      var modules = element("section", "inspector-section");
      modules.appendChild(element("h4", "", "进程内模块"));
      var moduleList = element("div", "module-list");
      currentNode.modules.forEach(function (item) {
        var button = element("button", "module-button", "");
        button.type = "button";
        button.setAttribute("aria-current", String(state.selectedModuleId === item.id));
        button.setAttribute("data-inspector-module", item.id);
        button.setAttribute("data-testid", "inspector-module-" + item.id);
        button.appendChild(element("span", "module-button__label", item.label));
        button.appendChild(element("span", "module-button__path", item.path));
        button.addEventListener("click", function () {
          state.selectedNodeId = currentNode.id;
          state.selectedModuleId = item.id;
          state.selectedEdgeId = null;
          writeHash();
          render();
        });
        moduleList.appendChild(button);
      });
      modules.appendChild(moduleList);
      wrapper.appendChild(modules);
    }
    appendPortList(wrapper, currentNode);
    appendSourceList(wrapper, currentNode.sourceIds);
    return wrapper;
  }

  function appendPortList(parent, currentNode) {
    var ports = FlowAtlasPorts.owned(model, currentNode.id);
    if (!ports.length) return;
    var section = element("section", "inspector-section");
    section.appendChild(element("h4", "", "Ports · " + ports.length));
    ports.forEach(function (item) {
      var button = element("button", "port-list-item", "◇ " + item.label);
      button.type = "button";
      button.setAttribute("data-testid", "port-select-" + item.id);
      button.setAttribute("aria-label", "查看 " + item.label + " 契约");
      button.appendChild(element("span", "port-list-kind", FlowAtlasPorts.kinds[item.kind]));
      button.addEventListener("click", function () { selectPort(item.id, button); });
      section.appendChild(button);
    });
    parent.appendChild(section);
  }

  function renderModuleInspector(currentNode, currentModule) {
    var wrapper = element("div");
    var title = element("div", "object-title");
    var titleInfo = element("div");
    titleInfo.appendChild(element("h3", "", currentModule.label));
    titleInfo.appendChild(element("div", "object-kind", "进程内模块 · " + currentNode.label));
    title.appendChild(titleInfo);
    title.appendChild(element("span", "tone-badge tone-badge--neutral", "模块"));
    wrapper.appendChild(title);
    var details = element("dl", "detail-list");
    appendDetailRow(details, "所属进程", currentNode.label);
    appendDetailRow(details, "源码路径", currentModule.path, "mono");
    wrapper.appendChild(details);
    var description = element("section", "inspector-section");
    description.appendChild(element("h4", "", "职责"));
    description.appendChild(element("div", "detail-description", currentModule.description || "模型未提供描述"));
    wrapper.appendChild(description);
    var back = element("button", "copy-button", "查看所属进程");
    back.type = "button";
    back.setAttribute("data-testid", "inspector-back");
    back.addEventListener("click", function () { state.selectedModuleId = null; render(); });
    wrapper.appendChild(back);
    var moduleSource = array(model.sources).filter(function (item) { return item.path === currentModule.path; }).map(function (item) { return item.id; });
    appendSourceList(wrapper, moduleSource.length ? moduleSource : currentNode.sourceIds);
    return wrapper;
  }

  function renderEdgeInspector(currentEdge) {
    var wrapper = element("div");
    var title = element("div", "object-title");
    var titleInfo = element("div");
    titleInfo.appendChild(element("h3", "", currentEdge.label));
    titleInfo.appendChild(element("div", "object-kind", "传输通道"));
    title.appendChild(titleInfo);
    title.appendChild(element("span", "tone-badge tone-badge--busy", "通道"));
    wrapper.appendChild(title);
    var details = element("dl", "detail-list");
    appendDetailRow(details, "起点", labelFor(currentEdge.from));
    appendDetailRow(details, "终点", labelFor(currentEdge.to));
    appendDetailRow(details, "传输", currentEdge.transport);
    appendDetailRow(details, "用途", currentEdge.purpose);
    wrapper.appendChild(details);
    var step = currentStep();
    if (step && array(step.edgeIds).indexOf(currentEdge.id) >= 0) {
      var context = element("section", "inspector-section");
      context.appendChild(element("h4", "", "当前步骤"));
      context.appendChild(element("div", "detail-description", step.title + " · " + (step.handler ? labelFor(step.handler) : "处理者未指明")));
      wrapper.appendChild(context);
    }
    appendSourceList(wrapper, currentEdge.sourceIds);
    return wrapper;
  }

  function renderInspector() {
    refs.inspectorPanel.hidden = !disclosure.open;
    updateDisclosureControls();
    if (!disclosure.open) return;
    var currentNode = node(state.selectedNodeId);
    var currentModule = currentNode && array(currentNode.modules).find(function (item) { return item.id === state.selectedModuleId; });
    var currentEdge = edge(state.selectedEdgeId);
    refs.inspector.replaceChildren();
    refs.inspectorHeading.textContent = disclosure.kind === "object" ? "对象检查" : (disclosure.kind === "event" ? "事件详情" : (disclosure.kind === "port" ? "Port 契约" : (disclosure.kind === "calls" ? "交互与结果" : "状态前后")));
    if (disclosure.kind === "port") {
      var selectedPort = capability(disclosure.key);
      refs.inspectorContext.textContent = selectedPort ? FlowAtlasPorts.kinds[selectedPort.kind] : "Port";
      if (selectedPort) refs.inspector.appendChild(FlowAtlasPorts.showContract(selectedPort, node(selectedPort.nodeId), callUI()));
      return;
    }
    if (disclosure.kind === "calls") {
      refs.inspectorContext.textContent = eventPhase() === "applied" ? "结果已知" : "尚未应用";
      currentCalls().forEach(function (call) { refs.inspector.appendChild(FlowAtlasPorts.showCall(call, callStatus(call), callUI())); });
      return;
    }
    if (disclosure.kind === "event") {
      refs.inspectorContext.textContent = "行为演示";
      refs.inspector.appendChild(renderEventInspector());
      return;
    }
    if (disclosure.kind === "changes" || disclosure.kind === "fact") {
      refs.inspectorContext.textContent = eventPhase() === "applied" ? "已应用" : "待应用";
      refs.inspector.appendChild(renderChangesInspector());
      return;
    }
    if (currentModule && currentNode) {
      refs.inspectorContext.textContent = "模块";
      refs.inspector.appendChild(renderModuleInspector(currentNode, currentModule));
    } else if (currentEdge) {
      refs.inspectorContext.textContent = "通道";
      refs.inspector.appendChild(renderEdgeInspector(currentEdge));
    } else if (currentNode) {
      refs.inspectorContext.textContent = kindLabels[currentNode.kind] || "对象";
      refs.inspector.appendChild(renderNodeInspector(currentNode, visibleState()));
    } else {
      refs.inspectorContext.textContent = "未选择";
      refs.inspector.appendChild(element("div", "inspector-empty", "选择地图中的对象查看它的边界、能力和来源。"));
    }
  }

  function renderEventInspector() {
    var wrapper = element("div", "event-details");
    var step = currentStep();
    var currentScenario = scenario();
    wrapper.appendChild(element("h3", "step-title", step ? step.title : (currentScenario && currentScenario.label || "选择一个场景")));
    if (!step) {
      wrapper.appendChild(element("p", "detail-description", currentScenario && currentScenario.question || ""));
      return wrapper;
    }
    var details = element("dl", "detail-list");
    appendDetailRow(details, "事件", step.event, "mono");
    appendDetailRow(details, "动作方", labelFor(step.actor));
    appendDetailRow(details, "处理方", labelFor(step.handler));
    if (step.certainty) appendDetailRow(details, "依据", step.certainty);
    wrapper.appendChild(details);
    var beats = element("ol", "beat-list");
    beatsFor(step).forEach(function (beat, index) {
      var item = element("li", "", beatLabels[beat.kind] + " · " + beat.label);
      if (transition && transition.phase === "flow" && index === transition.beatIndex) item.setAttribute("aria-current", "step");
      beats.appendChild(item);
    });
    wrapper.appendChild(beats);
    var actions = element("ul", "action-list");
    array(step.actions).forEach(function (action) { actions.appendChild(element("li", "", action)); });
    wrapper.appendChild(actions);
    currentCalls().forEach(function (call) { wrapper.appendChild(FlowAtlasPorts.showCall(call, callStatus(call), callUI())); });
    if (step.note) wrapper.appendChild(element("aside", "step-note", step.note));
    appendSourceList(wrapper, step.sourceIds);
    return wrapper;
  }

  function renderChangesInspector() {
    var wrapper = element("div", "changes-details");
    var changes = changesForStep();
    var visible = visibleState();
    if (disclosure.kind === "fact") {
      wrapper.appendChild(element("h3", "step-title", factLabels[disclosure.key] || "状态"));
      wrapper.appendChild(element("p", "fact-current", "当前 · " + (visible.facts[disclosure.key] || "未提供")));
      changes = changes.filter(function (change) { return change.kind === "fact" && change.id === disclosure.key; });
    }
    if (!changes.length) wrapper.appendChild(element("p", "detail-description", "本步骤未改写此处事实。"));
    changes.forEach(function (change) {
      var section = element("section", "change-detail");
      section.appendChild(element("h4", "", change.label));
      var values = element("dl", "detail-list");
      appendDetailRow(values, "之前", change.before);
      appendDetailRow(values, eventPhase() === "applied" ? "之后" : "待应用", change.after);
      section.appendChild(values);
      wrapper.appendChild(section);
    });
    return wrapper;
  }

  function disclosureButton(text, testId, kind, key) {
    var button = element("button", "disclosure-button", text);
    button.type = "button";
    if (testId) button.setAttribute("data-testid", testId);
    button.setAttribute("aria-controls", "inspector");
    button.setAttribute("data-disclosure-kind", kind);
    button.setAttribute("data-disclosure-key", key || "");
    button.setAttribute("aria-expanded", String(disclosure.open && disclosure.kind === kind && disclosure.key === (key || null)));
    button.addEventListener("click", function () { openDisclosure(kind, key, button); });
    return button;
  }

  function renderStep() {
    var previousDiff = refs.step.querySelector('[data-testid="state-diff"]');
    var diffScroll = previousDiff ? previousDiff.scrollLeft : 0;
    var currentScenario = scenario();
    var steps = array(currentScenario && currentScenario.steps);
    var step = currentStep();
    var preview = step || steps[0];
    refs.step.replaceChildren();
    refs.stepCounter.textContent = (state.stepIndex < 0 ? "开始" : (state.stepIndex + 1) + " / " + steps.length);
    refs.previous.disabled = state.stepIndex < 0;
    refs.next.disabled = !steps.length || state.stepIndex >= steps.length - 1;
    var titleRow = element("div", "step-title-row");
    var title = element("h3", "step-title", step ? step.title : (preview ? "下一步 · " + preview.title : "选择一个场景"));
    title.title = title.textContent;
    titleRow.appendChild(title);
    var phase = element("span", "event-phase");
    phase.setAttribute("data-testid", "event-phase");
    phase.setAttribute("data-phase", eventPhase());
    phase.setAttribute("role", "status");
    phase.setAttribute("aria-live", "polite");
    var beat = currentBeat();
    phase.setAttribute("data-beat-kind", beat ? beat.kind : "");
    var phaseText = beat ? beatLabels[beat.kind] + " " + (transition.beatIndex + 1) + "/" + transition.beats.length : phaseLabels[eventPhase()];
    phase.textContent = "行为演示 · " + phaseText + (transition && !state.playing && eventPhase() !== "applied" ? " · 已暂停" : "");
    titleRow.appendChild(phase);
    var replay = element("button", "disclosure-button event-replay", "↻ 重播");
    replay.type = "button";
    replay.disabled = !step;
    replay.setAttribute("data-testid", "event-replay");
    replay.setAttribute("aria-label", "重播当前事件");
    replay.addEventListener("click", function () { setStep(state.stepIndex, false, true, false); });
    titleRow.appendChild(replay);
    titleRow.appendChild(disclosureButton("ⓘ 详情", "event-details", "event"));
    refs.step.appendChild(titleRow);
    var context = element("div", "step-context");
    var call = activeCall();
    var route = element("span", "event-route", call ? call.from.label + " → " + call.target.label : (preview ? "动作 " + labelFor(preview.actor) + " → 处理 " + labelFor(preview.handler) : "尚未执行事件"));
    route.title = route.textContent;
    context.appendChild(route);
    var event;
    if (call) {
      var callState = callStatus(call);
      event = disclosureButton(FlowAtlasPorts.kinds[call.port.kind] + " · " + call.port.label + " · " + callState.label, "current-call", "calls");
      event.classList.add("current-call", "current-call--" + callState.stage);
      event.setAttribute("data-call-stage", callState.stage);
      event.setAttribute("data-result-known", String(callState.known));
      event.setAttribute("aria-label", call.from.label + " → " + call.target.label + "." + call.port.label + " · " + callState.label);
    } else event = element("span", "step-event", beat ? beat.label : (preview && preview.event || "点击播放开始"));
    event.title = event.textContent;
    context.appendChild(event);
    refs.step.appendChild(context);
    renderDiff();
    refs.step.querySelector('[data-testid="state-diff"]').scrollLeft = diffScroll;
  }

  function renderDiff() {
    var strip = element("div", "state-diff");
    strip.setAttribute("data-testid", "state-diff");
    strip.setAttribute("data-applied", String(eventPhase() === "applied"));
    strip.setAttribute("aria-label", eventPhase() === "applied" ? "已应用的状态变化" : "待应用的状态变化");
    var changes = changesForStep();
    strip.appendChild(element("span", "state-diff__phase", eventPhase() === "applied" ? "已应用" : "待应用"));
    if (!changes.length) strip.appendChild(element("span", "state-diff__empty", currentStep() ? "此步保留既有状态" : "—"));
    changes.forEach(function (change) {
      var button = disclosureButton("", "diff-" + change.kind + "-" + change.id, "changes");
      button.classList.add("state-diff__item");
      button.title = change.label + "：" + change.before + " → " + change.after;
      button.setAttribute("aria-label", button.title);
      button.appendChild(element("strong", "state-diff__label", change.label));
      button.appendChild(element("span", "state-diff__before", change.before));
      button.appendChild(element("span", "state-diff__arrow", "→"));
      button.appendChild(element("span", "state-diff__after", change.after));
      strip.appendChild(button);
    });
    refs.step.appendChild(strip);
  }

  function renderFacts() {
    var folded = visibleState();
    var changes = changesForStep();
    refs.facts.replaceChildren();
    Object.keys(factLabels).forEach(function (key) {
      var changed = changes.some(function (change) { return change.kind === "fact" && change.id === key; });
      var card = disclosureButton("", "fact-" + key, "fact", key);
      card.className = "fact-card" + (changed ? " is-changed" : "");
      card.setAttribute("data-changed", String(changed));
      card.title = factLabels[key] + "：" + (folded.facts[key] || "未提供");
      card.appendChild(element("div", "fact-label", factLabels[key]));
      card.appendChild(element("div", "fact-value", folded.facts[key] || "未提供"));
      refs.facts.appendChild(card);
    });
    refs.facts.style.setProperty("--fact-count", Object.keys(factLabels).length);
    refs.facts.setAttribute("data-phase", eventPhase());
  }

  function render() {
    var focusedId = document.activeElement && document.activeElement.getAttribute("data-testid");
    var currentMode = mode();
    var currentScenario = scenario();
    refs.title.textContent = model.title || "流程全景图";
    refs.scope.textContent = model.scope || "源码行为模型";
    setOptions(refs.mode, array(model.modes), state.modeId, "选择运行方式");
    setOptions(refs.scenario, modeScenarios(state.modeId), state.scenarioId, "选择行为场景");
    refs.mapSummary.textContent = (currentMode && currentMode.label || "运行方式") + " · " + (currentScenario && currentScenario.label || "行为场景");
    refs.sourceSummary.textContent = array(model.sources).length + " 条来源 · " + array(model.nodes).length + " 个对象 · " + array(model.edges).length + " 条通道";
    renderMap();
    renderInspector();
    renderStep();
    renderFacts();
    updatePlayButton();
    if (focusedId && (!document.activeElement || document.activeElement === document.body)) {
      var focused = document.querySelector('[data-testid="' + focusedId + '"]');
      if (focused) focused.focus({ preventScroll: true });
    }
  }

  function selectPort(id, opener) {
    var item = capability(id);
    if (!item) return;
    state.selectedPortId = id;
    state.selectedNodeId = item.nodeId;
    state.selectedModuleId = null;
    state.selectedEdgeId = null;
    writeHash();
    openDisclosure("port", id, opener || document.activeElement);
    renderMap();
  }

  function selectNode(id) {
    if (!node(id)) return;
    state.selectedNodeId = id;
    state.selectedModuleId = null;
    state.selectedEdgeId = null;
    state.selectedPortId = null;
    writeHash();
    openDisclosure("object", null, document.activeElement);
    render();
  }

  function selectModule(nodeId, moduleId) {
    var currentNode = node(nodeId);
    if (!currentNode || !array(currentNode.modules).some(function (item) { return item.id === moduleId; })) return;
    state.selectedNodeId = nodeId;
    state.selectedModuleId = moduleId;
    state.selectedEdgeId = null;
    writeHash();
    openDisclosure("object", null, document.activeElement);
    render();
  }

  function selectEdge(id) {
    if (!edge(id)) return;
    state.selectedEdgeId = id;
    state.selectedModuleId = null;
    writeHash();
    openDisclosure("object", null, document.activeElement);
    render();
  }

  function onMapClick(event) {
    if (layoutUI && layoutUI.click(event)) return;
    var factTarget = event.target.closest ? event.target.closest("[data-fact-id]") : null;
    if (factTarget) { openDisclosure("fact", factTarget.getAttribute("data-fact-id"), factTarget); return; }
    var portTarget = event.target.closest ? event.target.closest("[data-port-id]") : null;
    if (portTarget) { selectPort(portTarget.getAttribute("data-port-id"), portTarget); return; }
    var portsTarget = event.target.closest ? event.target.closest("[data-node-ports]") : null;
    if (portsTarget) { selectNode(portsTarget.getAttribute("data-node-ports")); return; }
    var stepTarget = event.target.closest ? event.target.closest("[data-step-index]") : null;
    if (stepTarget) { setStep(Number(stepTarget.getAttribute("data-step-index")), false, true, false); return; }
    var moduleTarget = event.target.closest ? event.target.closest("[data-module-id]") : null;
    if (moduleTarget) { selectModule(moduleTarget.getAttribute("data-node-id"), moduleTarget.getAttribute("data-module-id")); return; }
    var edgeTarget = event.target.closest ? event.target.closest("[data-edge-id]") : null;
    if (edgeTarget) { selectEdge(edgeTarget.getAttribute("data-edge-id")); return; }
    var nodeTarget = event.target.closest ? event.target.closest("[data-node-id]") : null;
    if (nodeTarget) selectNode(nodeTarget.getAttribute("data-node-id"));
  }

  function onMapKeydown(event) {
    if (layoutUI && layoutUI.keydown(event)) return;
    if (event.key !== "Enter" && event.key !== " ") return;
    var target = event.target;
    if (target.hasAttribute("data-layout-handle")) { event.preventDefault(); selectNode(target.getAttribute("data-layout-handle")); return; }
    if (target.hasAttribute("data-fact-id")) { event.preventDefault(); openDisclosure("fact", target.getAttribute("data-fact-id"), target); return; }
    if (target.hasAttribute("data-port-id")) { event.preventDefault(); selectPort(target.getAttribute("data-port-id"), target); return; }
    if (target.hasAttribute("data-node-ports")) { event.preventDefault(); selectNode(target.getAttribute("data-node-ports")); return; }
    if (target.hasAttribute("data-step-index")) { event.preventDefault(); setStep(Number(target.getAttribute("data-step-index")), false, true, false); return; }
    if (target.getAttribute("data-module-id")) selectModule(target.getAttribute("data-node-id"), target.getAttribute("data-module-id"));
    else if (target.getAttribute("data-edge-id")) selectEdge(target.getAttribute("data-edge-id"));
    else if (target.getAttribute("data-node-id")) selectNode(target.getAttribute("data-node-id"));
    if (target.getAttribute("data-node-id") || target.getAttribute("data-edge-id")) event.preventDefault();
  }

  // Reuse the template's inspector locally; its hidden state never consumes map
  // width. Nonmodal disclosure preserves map, playback and keyboard access.
  function setupWorkspace() {
    refs.inspectorPanel = document.getElementById("inspector");
    refs.inspectorHeading = document.getElementById("inspector-heading");
    refs.inspectorPanel.hidden = true;
    refs.inspectorPanel.setAttribute("role", "region");
    refs.inspectorPanel.tabIndex = -1;
    refs.closeDetail = element("button", "icon-button inspector-close", "×");
    refs.closeDetail.type = "button";
    refs.closeDetail.setAttribute("aria-label", "关闭详情");
    refs.closeDetail.setAttribute("data-testid", "detail-close");
    refs.closeDetail.addEventListener("click", function () { closeDisclosure(true); });
    refs.inspectorContext.parentNode.appendChild(refs.closeDetail);
    refs.detailToggle = element("button", "disclosure-button", "ⓘ 对象");
    refs.detailToggle.type = "button";
    refs.detailToggle.setAttribute("data-testid", "detail-toggle");
    refs.detailToggle.setAttribute("aria-controls", "inspector");
    refs.detailToggle.setAttribute("data-disclosure-kind", "object");
    refs.detailToggle.addEventListener("click", function () {
      if (disclosure.open && disclosure.kind === "object") closeDisclosure(true);
      else openDisclosure("object", null, refs.detailToggle);
    });
    refs.mapSummary.parentNode.appendChild(refs.detailToggle);
    refs.scope.classList.add("sr-only");
    document.querySelector(".atlas-heading").setAttribute("title", model.scope || "源码行为模型；非实时监控");
    document.querySelector(".facts-badge").textContent = "点击展开";
    document.querySelector(".map-meta__hint").classList.add("sr-only");
    document.getElementById("step-heading").textContent = "当前事件";
  }

  function openDisclosure(kind, key, opener) {
    disclosure.kind = kind;
    disclosure.key = key || null;
    disclosure.open = true;
    if (opener && opener !== document.body) {
      disclosure.opener = { element: opener, testId: opener.getAttribute("data-testid") };
    }
    renderInspector();
    // Explicit selection starts at the contract heading; phase refreshes retain scroll.
    refs.inspector.scrollTop = 0;
    refs.closeDetail.focus({ preventScroll: true });
  }

  function updateDisclosureControls() {
    document.querySelectorAll('[aria-controls="inspector"]').forEach(function (control) {
      var expanded = disclosure.open && control.getAttribute("data-disclosure-kind") === disclosure.kind &&
        (control.getAttribute("data-disclosure-key") || "") === (disclosure.key || "");
      control.setAttribute("aria-expanded", String(expanded));
    });
  }

  function closeDisclosure(restoreFocus) {
    disclosure.open = false;
    if (!refs.inspectorPanel) return;
    refs.inspectorPanel.hidden = true;
    updateDisclosureControls();
    if (restoreFocus) {
      var target = disclosure.opener && disclosure.opener.element;
      if (target && !target.isConnected && disclosure.opener.testId) {
        target = document.querySelector('[data-testid="' + disclosure.opener.testId + '"]');
      }
      (target && target.isConnected ? target : refs.detailToggle).focus({ preventScroll: true });
    }
  }

  function bind() {
    refs.views.addEventListener("click", function (event) { var target = event.target.closest("[data-view]"); if (target) setView(target.getAttribute("data-view")); });
    refs.speed.addEventListener("change", function () { setSpeed(Number(refs.speed.value)); });
    refs.mode.addEventListener("change", function () { setMode(refs.mode.value); });
    refs.scenario.addEventListener("change", function () { setScenario(refs.scenario.value); });
    refs.previous.addEventListener("click", function () { setStep(state.stepIndex - 1, false); });
    refs.next.addEventListener("click", function () { setStep(state.stepIndex + 1, false, true, false); });
    refs.reset.addEventListener("click", function () { setStep(-1, false); });
    refs.play.addEventListener("click", togglePlayback);
    refs.map.addEventListener("scroll", updateSequenceHeader, { passive: true });
    refs.map.addEventListener("click", onMapClick);
    refs.map.addEventListener("keydown", onMapKeydown);
    window.addEventListener("resize", function () { if (layoutUI) layoutUI.cancel(false); renderMap(); });
    window.addEventListener("hashchange", function () { closeDisclosure(false); readHash(); render(); });
    document.addEventListener("keydown", function (event) {
      if (event.key === "Escape" && disclosure.open) { event.preventDefault(); closeDisclosure(true); }
    });
    document.addEventListener("pointerdown", function (event) {
      if (!disclosure.open || refs.inspectorPanel.contains(event.target)) return;
      if (event.target.closest("[aria-controls='inspector'], #atlas-map")) return;
      closeDisclosure(false);
    });
    document.addEventListener("visibilitychange", function () {
      if (document.hidden && state.playing) pausePlayback();
    });
    motionPreference.addEventListener("change", function () { updateFlowMarker(); });
  }

  function showModelError() {
    if (array(model.modes).length) return;
    refs.map.replaceChildren(element("div", "atlas-map__empty", "源模型尚未提供可展示的运行方式。"));
    refs.inspector.replaceChildren(element("div", "inspector-empty", "无法读取 atlas model。"));
  }

  setupWorkspace();
  readHash();
  layoutUI = FlowAtlasLayoutUI.create({
    model: model, map: refs.map, container: refs.mapSummary.parentNode,
    context: function () { return { view: state.view, modeId: state.modeId, scenarioId: state.scenarioId, selectedNodeId: state.selectedNodeId }; },
    label: labelFor, pause: pausePlayback, changed: renderMap,
    select: function (id) {
      if (state.selectedNodeId !== id) {
        state.selectedNodeId = id;
        state.selectedModuleId = null; state.selectedEdgeId = null; state.selectedPortId = null;
        if (disclosure.open) { disclosure.kind = "object"; disclosure.key = null; renderInspector(); }
      }
      writeHash();
    }
  });
  viewport = FlowAtlasViewport.create({ host: refs.map, controls: refs.map.parentNode, beforeNavigate: function () { if (layoutUI) layoutUI.cancel(); } });
  bind();
  render();
  showModelError();
}());
