/* DOM adapter for FlowAtlasLayout. The host owns playback and semantic state;
 * this adapter owns in-memory drafts, one pointer gesture and file exchange.
 */
(function (global) {
  "use strict";

  /** create({model,map,container,context,label,pause,changed,select}) -> adapter.
   * context() supplies {view,modeId,scenarioId,selectedNodeId}; project() seeds
   * each scope from default geometry. changed() repaints without folding facts.
   */
  function create(options) {
    var layout = FlowAtlasLayout;
    var digest = layout.identity(options.model);
    var sessions = new Map();
    var pressed = null;
    var suppressClick = false;
    var diagnostics = [];
    var controls = {};

    function make(tag, className, text) {
      var item = document.createElement(tag);
      if (className) item.className = className;
      if (text !== undefined) item.textContent = text;
      return item;
    }
    function key() {
      var ctx = options.context();
      return JSON.stringify([digest, ctx.view, ctx.modeId, ctx.scenarioId]);
    }
    function current() { return sessions.get(key()) || null; }
    function announce(text, failed) {
      controls.feedback.textContent = text;
      controls.feedback.classList.toggle("is-error", Boolean(failed));
      controls.feedback.hidden = !text;
      controls.live.textContent = text;
    }
    function focusHeading(id) {
      var handle = options.map.querySelector('[data-layout-handle="' + id + '"]');
      if (handle) handle.focus({ preventScroll: true });
    }
    function redraw() { options.changed(); refresh(); }

    function project(base, sizes, edges) {
      var ctx = options.context(), id = key();
      if (!ctx.modeId || !ctx.scenarioId) return Object.assign({ x: 0, y: 0 }, base);
      if (!sessions.has(id)) sessions.set(id, layout.create({ contentDigest: digest, scope: { view: ctx.view, modeId: ctx.modeId, scenarioId: ctx.scenarioId }, positions: base.nodes }));
      var result = layout.project(sessions.get(id), base, sizes, edges);
      // Freeze the canvas coordinate frame during a gesture. Expanding its
      // viewBox mid-drag would move a centered SVG underneath the pointer.
      if (pressed && pressed.dragging && pressed.key === id) Object.assign(result, pressed.viewport);
      return result;
    }

    function selected() {
      var session = current(), id = options.context().selectedNodeId;
      return session && Object.prototype.hasOwnProperty.call(session.document.positions, id) ? id : null;
    }

    function refresh() {
      var session = current(), id = selected(), enabled = options.context().view === "relations" && Boolean(session);
      controls.menu.hidden = options.context().view !== "relations";
      controls.diagnostics.hidden = !enabled || !diagnostics.length;
      controls.pin.disabled = !enabled || !id;
      controls.pin.setAttribute("aria-pressed", String(Boolean(id && layout.pinned(session, id))));
      controls.pin.textContent = id && layout.pinned(session, id) ? "◇ 取消固定" : "◆ 固定对象";
      controls.undo.disabled = !enabled || (!session.history.length && !session.gesture);
      controls.reset.disabled = !enabled;
      controls.export.disabled = !enabled;
      controls.import.disabled = !enabled;
      controls.moves.forEach(function (button) { button.disabled = !enabled || !id || layout.pinned(session, id); });
      controls.selection.textContent = id ? options.label(id) : "未选择对象";
      options.map.setAttribute("data-layout-dragging", String(Boolean(pressed && pressed.dragging)));
      options.map.setAttribute("data-layout-undo-count", String(session ? session.history.length : 0));
    }

    function releaseCapture(pointerId) {
      if (options.map.hasPointerCapture(pointerId)) options.map.releasePointerCapture(pointerId);
    }
    function cancel(repaint) {
      if (!pressed) return false;
      var previous = pressed;
      pressed = null;
      sessions.set(previous.key, layout.cancel(sessions.get(previous.key)));
      releaseCapture(previous.pointerId);
      if (previous.dragging) {
        suppressClick = true;
        announce("已取消移动");
        if (repaint !== false) { redraw(); focusHeading(previous.id); }
      }
      refresh();
      return true;
    }

    function pointerDown(event) {
      suppressClick = false;
      if (event.button !== 0 || event.isPrimary === false || pressed || options.context().view !== "relations") return;
      var handle = event.target.closest && event.target.closest("[data-layout-handle]");
      if (!handle || !options.map.contains(handle)) return;
      var session = current(), id = handle.getAttribute("data-layout-handle");
      if (!session) return;
      if (layout.pinned(session, id)) { announce(options.label(id) + "已固定；按 P 或在布局菜单取消固定"); return; }
      var svg = options.map.querySelector("svg"), matrix = svg && svg.getScreenCTM();
      if (!matrix) return;
      var bounds = svg.viewBox.baseVal;
      pressed = { id: id, key: key(), pointerId: event.pointerId, clientX: event.clientX, clientY: event.clientY, scrollLeft: options.map.scrollLeft, scrollTop: options.map.scrollTop, origin: session.document.positions[id], inverse: matrix.inverse(), viewport: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height }, dragging: false };
    }

    function pointerMove(event) {
      if (!pressed || event.pointerId !== pressed.pointerId) return;
      var dx = event.clientX - pressed.clientX, dy = event.clientY - pressed.clientY;
      if (!pressed.dragging && Math.hypot(dx, dy) < 6) return;
      if (key() !== pressed.key) { cancel(); return; }
      if (!pressed.dragging) {
        options.pause();
        options.select(pressed.id);
        sessions.set(pressed.key, layout.begin(sessions.get(pressed.key), pressed.id));
        pressed.dragging = true;
        options.map.setPointerCapture(pressed.pointerId);
      }
      event.preventDefault();
      dx += options.map.scrollLeft - pressed.scrollLeft;
      dy += options.map.scrollTop - pressed.scrollTop;
      var inverse = pressed.inverse;
      var point = { x: Math.max(-1000000, Math.min(1000000, pressed.origin.x + inverse.a * dx + inverse.c * dy)), y: Math.max(-1000000, Math.min(1000000, pressed.origin.y + inverse.b * dx + inverse.d * dy)) };
      sessions.set(pressed.key, layout.update(sessions.get(pressed.key), point));
      redraw();
    }

    function pointerUp(event) {
      if (!pressed || event.pointerId !== pressed.pointerId) return;
      if (!pressed.dragging) { pressed = null; return; }
      pointerMove(event);
      if (!pressed) return;
      var previous = pressed;
      pressed = null;
      sessions.set(previous.key, layout.commit(sessions.get(previous.key)));
      suppressClick = true;
      releaseCapture(previous.pointerId);
      announce("已移动 " + options.label(previous.id));
      redraw();
      focusHeading(previous.id);
    }

    function click(event) {
      if (!suppressClick || event.detail === 0) return false;
      suppressClick = false;
      event.preventDefault();
      return true;
    }

    function move(id, dx, dy) {
      cancel();
      var session = current();
      if (!session || !id) return;
      if (layout.pinned(session, id)) { announce("对象已固定；先取消固定"); return; }
      var origin = session.document.positions[id];
      var point = { x: Math.max(-1000000, Math.min(1000000, origin.x + dx)), y: Math.max(-1000000, Math.min(1000000, origin.y + dy)) };
      if (point.x === origin.x && point.y === origin.y) return;
      options.pause();
      options.select(id);
      sessions.set(key(), layout.move(session, id, point));
      announce("已移动 " + options.label(id));
      redraw();
    }

    function pin(id) {
      cancel(false);
      if (!id || !current()) return;
      sessions.set(key(), layout.togglePin(current(), id));
      announce(options.label(id) + (layout.pinned(current(), id) ? "已固定" : "已取消固定"));
      redraw();
    }

    function keydown(event) {
      if (event.key === "Escape" && pressed) { event.preventDefault(); cancel(); return true; }
      var handle = event.target.closest && event.target.closest("[data-layout-handle]");
      if (!handle || !options.map.contains(handle) || event.altKey || event.ctrlKey || event.metaKey) return false;
      var id = handle.getAttribute("data-layout-handle");
      if (event.key.toLowerCase() === "p") {
        event.preventDefault(); options.select(id); pin(id); focusHeading(id); return true;
      }
      var directions = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
      var direction = directions[event.key];
      if (!direction) return false;
      event.preventDefault();
      var step = event.shiftKey ? 1 : 10;
      move(id, direction[0] * step, direction[1] * step);
      focusHeading(id);
      return true;
    }

    function button(parent, id, label, action) {
      var item = make("button", "disclosure-button", label);
      item.type = "button";
      item.setAttribute("data-testid", id);
      item.addEventListener("click", action);
      parent.appendChild(item);
      return item;
    }

    function download() {
      cancel();
      if (!current()) return;
      try {
        var content = JSON.stringify(layout.exportDocument(current()), null, 2) + "\n";
        var url = URL.createObjectURL(new Blob([content], { type: "application/json" }));
        var anchor = make("a");
        anchor.href = url;
        anchor.download = "flow-atlas-" + options.context().modeId + "-" + options.context().scenarioId + "-layout.json";
        document.body.appendChild(anchor);
        anchor.click();
        anchor.remove();
        URL.revokeObjectURL(url);
        announce("布局已导出");
      } catch (error) { announce("导出失败：" + error.message, true); }
    }

    async function importFile() {
      var file = controls.file.files[0];
      if (!file) return;
      var targetKey = key(), before = current();
      try {
        if (file.size > 1048576) throw new Error("文件超过 1 MiB");
        var text = await file.text();
        if (key() !== targetKey || current() !== before) throw new Error("布局或场景已变化，请重新导入");
        var next = layout.importDocument(before, text);
        sessions.set(targetKey, next);
        announce("布局已导入");
        redraw();
      } catch (error) { announce("导入失败：" + error.message, true); }
      controls.file.value = "";
    }

    function setDiagnostics(items) {
      diagnostics = items;
      controls.diagnosticSummary.textContent = "⚠ " + items.length;
      controls.diagnosticSummary.setAttribute("aria-label", items.length + " 项布局诊断");
      controls.diagnosticList.replaceChildren();
      items.forEach(function (item) {
        var row = make("li", "layout-diagnostic", item.message);
        row.setAttribute("data-testid", "layout-diagnostic");
        row.setAttribute("data-diagnostic-code", item.code);
        if (item.routeId) row.setAttribute("data-route-id", item.routeId);
        controls.diagnosticList.appendChild(row);
      });
      refresh();
    }

    function setup() {
      var host = make("div", "layout-tools");
      host.setAttribute("data-testid", "layout-controls");
      controls.menu = make("details", "layout-disclosure");
      controls.menu.setAttribute("data-testid", "layout-menu");
      var summary = make("summary", "disclosure-button", "⠿ 布局");
      summary.setAttribute("aria-label", "布局操作");
      summary.setAttribute("data-testid", "layout-toggle");
      controls.menu.appendChild(summary);
      var panel = make("div", "layout-popover");
      controls.selection = make("strong", "layout-selection");
      panel.appendChild(controls.selection);
      var actions = make("div", "layout-actions");
      controls.pin = button(actions, "layout-pin", "◆ 固定对象", function () { pin(selected()); });
      controls.undo = button(actions, "layout-undo", "↶ 撤销", function () {
        if (cancel()) return;
        if (current()) { sessions.set(key(), layout.undo(current())); announce("已撤销布局操作"); redraw(); }
      });
      controls.reset = button(actions, "layout-reset", "↺ 重置", function () { cancel(false); if (current()) { sessions.set(key(), layout.reset(current())); announce("布局已重置，可撤销"); redraw(); } });
      panel.appendChild(actions);
      var movement = make("div", "layout-movement");
      movement.setAttribute("role", "group"); movement.setAttribute("aria-label", "移动所选对象");
      controls.moves = [["left", "←", -10, 0, "左移"], ["up", "↑", 0, -10, "上移"], ["down", "↓", 0, 10, "下移"], ["right", "→", 10, 0, "右移"]].map(function (item) {
        var control = button(movement, "layout-move-" + item[0], item[1], function () { move(selected(), item[2], item[3]); });
        control.setAttribute("aria-label", item[4] + "所选对象 10 单位");
        return control;
      });
      panel.appendChild(movement);
      panel.appendChild(make("div", "layout-key-help", "标题区拖动 · 方向键 10 · Shift 1 · P 固定 · Esc 取消"));
      var exchange = make("div", "layout-actions");
      controls.export = button(exchange, "layout-export", "↓ 导出 JSON", download);
      controls.import = button(exchange, "layout-import", "↑ 导入 JSON", function () { cancel(); controls.file.click(); });
      panel.appendChild(exchange);
      controls.file = make("input");
      controls.file.type = "file"; controls.file.accept = ".json,application/json"; controls.file.hidden = true;
      controls.file.setAttribute("aria-label", "选择布局 JSON");
      controls.file.setAttribute("data-testid", "layout-import-input");
      controls.file.addEventListener("change", importFile);
      panel.appendChild(controls.file);
      controls.feedback = make("div", "layout-feedback");
      controls.feedback.hidden = true;
      controls.feedback.setAttribute("data-testid", "layout-feedback");
      panel.appendChild(controls.feedback);
      controls.menu.appendChild(panel); host.appendChild(controls.menu);
      controls.diagnostics = make("details", "layout-disclosure");
      controls.diagnostics.setAttribute("data-testid", "layout-diagnostics");
      controls.diagnosticSummary = make("summary", "disclosure-button", "⚠ 0");
      controls.diagnostics.appendChild(controls.diagnosticSummary);
      controls.diagnosticList = make("ul", "layout-popover layout-diagnostics-list");
      controls.diagnostics.appendChild(controls.diagnosticList); host.appendChild(controls.diagnostics);
      controls.menu.addEventListener("toggle", function () { if (controls.menu.open) controls.diagnostics.open = false; });
      controls.diagnostics.addEventListener("toggle", function () { if (controls.diagnostics.open) controls.menu.open = false; });
      controls.live = make("span", "sr-only"); controls.live.setAttribute("role", "status"); controls.live.setAttribute("aria-live", "polite");
      host.appendChild(controls.live);
      options.container.appendChild(host);
      options.map.addEventListener("pointerdown", pointerDown);
      window.addEventListener("pointermove", pointerMove, { passive: false });
      window.addEventListener("pointerup", pointerUp);
      window.addEventListener("pointercancel", function (event) { if (pressed && event.pointerId === pressed.pointerId) cancel(); });
      options.map.addEventListener("lostpointercapture", function (event) { if (pressed && event.pointerId === pressed.pointerId) cancel(); });
      window.addEventListener("blur", function () { cancel(); });
      document.addEventListener("keydown", function (event) {
        if (event.key === "Escape" && pressed) { event.preventDefault(); event.stopImmediatePropagation(); cancel(); }
        else if (event.key === "Escape" && (controls.menu.open || controls.diagnostics.open)) {
          event.preventDefault(); controls.menu.open = false; controls.diagnostics.open = false; summary.focus({ preventScroll: true });
        }
      });
      document.addEventListener("pointerdown", function (event) {
        if (host.contains(event.target)) return;
        controls.menu.open = false; controls.diagnostics.open = false;
      });
      refresh();
    }

    setup();
    return Object.freeze({ project: project, pinned: function (id) { return Boolean(current() && layout.pinned(current(), id)); }, cancel: cancel, click: click, keydown: keydown, refresh: refresh, diagnostics: setDiagnostics });
  }

  global.FlowAtlasLayoutUI = Object.freeze({ create: create });
}(globalThis));
