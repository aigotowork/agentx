/* Shared camera/navigation for SVG projections. No domain reducer, layout editor or clock. */
(function (global) {
  'use strict';
  var MIN_ZOOM = 0.1, MAX_ZOOM = 4, NARROW_WIDTH = 600, READABLE_ZOOM = 0.85;
  function clamp(zoom) { return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom)); }
  function zoomAt(camera, zoom, point) {
    zoom = clamp(zoom);
    return { x: camera.x + point.x / camera.zoom - point.x / zoom, y: camera.y + point.y / camera.zoom - point.y / zoom, zoom: zoom };
  }
  function fit(bounds, size) {
    var zoom = Math.min(1, clamp(Math.min(Math.max(1, size.width - 48) / Math.max(1, bounds.width), Math.max(1, size.height - 48) / Math.max(1, bounds.height))));
    return { x: bounds.x + bounds.width / 2 - size.width / zoom / 2, y: bounds.y + bounds.height / 2 - size.height / zoom / 2, zoom: zoom };
  }
  function initial(bounds, size) {
    var fitted = fit(bounds, size);
    // A phone starts with readable objects, not an illegible miniature overview.
    // Explicit fit remains a full overview; subsequent mounts retain the user's camera.
    if (size.width < NARROW_WIDTH && fitted.zoom < READABLE_ZOOM) {
      return { x: bounds.x - 24 / READABLE_ZOOM, y: bounds.y - 24 / READABLE_ZOOM, zoom: READABLE_ZOOM };
    }
    return fitted;
  }

  /** create({host,controls,beforeNavigate?}) -> {mount(svg,scope),fit,reveal,state,dispose}.
   * Borrows host/controls. Camera is per scope, transient, never written into domain/layout data.
   * beforeNavigate cancels a host-owned node gesture before changing its coordinate frame.
   */
  function create(options) {
    var host = options.host, root = null, scope = null, states = new Map(), content = null;
    function measure() { var rect = host.getBoundingClientRect(); return { width: rect.width, height: rect.height }; }
    var size = measure(), gesture = null;
    var pointers = new Map(), space = false, suppressClick = false, listeners = [];
    var panel = document.createElement('div'); panel.className = 'spatial-controls';
    panel.setAttribute('role', 'group'); panel.setAttribute('aria-label', '画布视口'); panel.dataset.testid = 'viewport-controls';
    host.classList.add('spatial-viewport'); host.tabIndex = 0;
    host.setAttribute('aria-keyshortcuts', 'ArrowUp ArrowDown ArrowLeft ArrowRight + - 0 F');
    var help = '空白处拖动平移；滚轮缩放；空格或鼠标中键拖动；双指缩放；F 适应内容，0 原始比例';
    host.setAttribute('title', help);
    function camera() { return states.get(scope); }
    function notify() {
      if (!root || !camera()) return;
      var value = camera();
      root.setAttribute('viewBox', [value.x, value.y, Math.max(1, size.width) / value.zoom, Math.max(1, size.height) / value.zoom].join(' '));
      root.setAttribute('preserveAspectRatio', 'none');
      host.style.setProperty('--spatial-grid-step', (24 * value.zoom) + 'px');
      host.style.setProperty('--spatial-grid-x', (-value.x * value.zoom % (24 * value.zoom)) + 'px');
      host.style.setProperty('--spatial-grid-y', (-value.y * value.zoom % (24 * value.zoom)) + 'px');
      host.dataset.cameraX = String(value.x); host.dataset.cameraY = String(value.y); host.dataset.cameraZoom = String(value.zoom);
      percentage.textContent = Math.round(value.zoom * 100) + '%';
      percentage.setAttribute('aria-label', '当前缩放 ' + Math.round(value.zoom * 100) + '%，恢复100%');
      minus.disabled = value.zoom <= MIN_ZOOM; plus.disabled = value.zoom >= MAX_ZOOM;
    }
    function set(value) { states.set(scope, value); notify(); }
    function before() { if (options.beforeNavigate) options.beforeNavigate(); }
    function button(id, name, text, action) {
      var item = document.createElement('button'); item.type = 'button'; item.className = 'spatial-button';
      item.dataset.testid = id; item.textContent = text; item.setAttribute('aria-label', name); item.title = name;
      item.addEventListener('click', action); panel.appendChild(item); return item;
    }
    function zoom(factor, point) { if (!camera()) return; before(); set(zoomAt(camera(), camera().zoom * factor, point || { x: size.width / 2, y: size.height / 2 })); }
    function fitContent() { if (!content) return; cancel(); before(); set(fit(content, size)); }
    var minus = button('viewport-zoom-out', '缩小画布', '−', function () { zoom(1 / 1.2); });
    var percentage = button('viewport-actual', '恢复100%', '100%', function () { if (camera()) zoom(1 / camera().zoom); });
    var plus = button('viewport-zoom-in', '放大画布', '+', function () { zoom(1.2); });
    button('viewport-fit', '适应全部内容', '⛶', fitContent);
    var info = document.createElement('details'); info.className = 'spatial-help';
    var summary = document.createElement('summary'); summary.textContent = 'ⓘ'; summary.setAttribute('aria-label', '画布操作说明'); summary.title = '画布操作说明';
    var explanation = document.createElement('span'); explanation.textContent = help;
    info.append(summary, explanation); panel.append(info); options.controls.appendChild(panel);
    function on(target, type, handler, config) { target.addEventListener(type, handler, config); listeners.push(function () { target.removeEventListener(type, handler, config); }); }
    function point(event) { var rect = host.getBoundingClientRect(); return { x: event.clientX - rect.left, y: event.clientY - rect.top }; }
    function interactive(target) { return target.closest && target.closest('[role="button"],a,button,input,select,textarea,[data-layout-handle]'); }
    function endCapture(id) { if (host.hasPointerCapture(id)) host.releasePointerCapture(id); }
    function cancel() {
      if (!gesture) return;
      var previous = gesture; gesture = null; pointers.clear();
      set(previous.camera); previous.ids.forEach(endCapture); host.dataset.panning = 'false';
    }
    function down(event) {
      if (!root || event.button > 1) return;
      if (!gesture) suppressClick = false;
      if (event.pointerType === 'touch') pointers.set(event.pointerId, point(event));
      var forced = event.button === 1 || space;
      var allowed = forced || !interactive(event.target);
      if (event.pointerType === 'touch' && pointers.size === 2) {
        before();
        var pair = Array.from(pointers.values()), current = camera();
        var center = { x: (pair[0].x + pair[1].x) / 2, y: (pair[0].y + pair[1].y) / 2 };
        gesture = { kind: 'pinch', camera: current, center: center, distance: Math.max(1, Math.hypot(pair[0].x - pair[1].x, pair[0].y - pair[1].y)), ids: Array.from(pointers.keys()), moved: true };
      } else if (allowed && !gesture) {
        before(); gesture = { kind: 'pan', camera: camera(), origin: point(event), ids: [event.pointerId], moved: false };
        suppressClick = false;
      } else return;
      host.focus({ preventScroll: true });
      gesture.ids.forEach(function (id) { host.setPointerCapture(id); });
      event.preventDefault(); event.stopImmediatePropagation();
    }
    function move(event) {
      if (event.pointerType === 'touch' && pointers.has(event.pointerId)) pointers.set(event.pointerId, point(event));
      if (!gesture || gesture.ids.indexOf(event.pointerId) < 0) return;
      var value;
      if (gesture.kind === 'pinch') {
        var pair = gesture.ids.map(function (id) { return pointers.get(id); }); if (!pair[0] || !pair[1]) return;
        var center = { x: (pair[0].x + pair[1].x) / 2, y: (pair[0].y + pair[1].y) / 2 };
        var distance = Math.hypot(pair[0].x - pair[1].x, pair[0].y - pair[1].y);
        value = zoomAt(gesture.camera, gesture.camera.zoom * distance / gesture.distance, gesture.center);
        value.x -= (center.x - gesture.center.x) / value.zoom; value.y -= (center.y - gesture.center.y) / value.zoom;
      } else {
        var p = point(event), dx = p.x - gesture.origin.x, dy = p.y - gesture.origin.y;
        if (!gesture.moved && Math.hypot(dx, dy) < 3) return;
        gesture.moved = true;
        value = { x: gesture.camera.x - dx / gesture.camera.zoom, y: gesture.camera.y - dy / gesture.camera.zoom, zoom: gesture.camera.zoom };
      }
      host.dataset.panning = 'true'; set(value); event.preventDefault(); event.stopImmediatePropagation();
    }
    function up(event) {
      pointers.delete(event.pointerId);
      if (!gesture || gesture.ids.indexOf(event.pointerId) < 0) return;
      var previous = gesture; gesture = null; pointers.clear(); suppressClick = previous.moved;
      previous.ids.forEach(endCapture); host.dataset.panning = 'false';
      event.preventDefault(); event.stopImmediatePropagation();
    }
    on(host, 'pointerdown', down, true); on(host, 'pointermove', move, true); on(host, 'pointerup', up, true);
    on(host, 'pointercancel', function (event) { pointers.delete(event.pointerId); if (gesture && gesture.ids.includes(event.pointerId)) { cancel(); event.stopImmediatePropagation(); } }, true);
    on(host, 'lostpointercapture', function (event) { if (gesture && gesture.ids.includes(event.pointerId)) cancel(); });
    on(host, 'click', function (event) { if (suppressClick) { suppressClick = false; event.preventDefault(); event.stopImmediatePropagation(); } }, true);
    on(host, 'wheel', function (event) { if (!root) return; cancel(); zoom(Math.exp(-Math.max(-500, Math.min(500, event.deltaY)) * 0.002), point(event)); event.preventDefault(); }, { passive: false });
    on(host, 'dblclick', function (event) { if (!interactive(event.target)) { event.preventDefault(); fitContent(); } });
    on(host, 'keydown', function (event) {
      if (event.key === 'Escape' && gesture) { cancel(); event.preventDefault(); event.stopImmediatePropagation(); return; }
      if (event.key === ' ' && event.target === host) { space = true; event.preventDefault(); return; }
      if (event.target !== host || !camera()) return;
      var directions = { ArrowLeft: [-60, 0], ArrowRight: [60, 0], ArrowUp: [0, -60], ArrowDown: [0, 60] };
      if (directions[event.key]) { before(); var delta = directions[event.key], value = camera(); set({ x: value.x + delta[0] / value.zoom, y: value.y + delta[1] / value.zoom, zoom: value.zoom }); }
      else if (event.key === '+' || event.key === '=') zoom(1.2);
      else if (event.key === '-') zoom(1 / 1.2);
      else if (event.key === '0') zoom(1 / camera().zoom);
      else if (event.key.toLowerCase() === 'f') fitContent();
      else return;
      event.preventDefault(); event.stopImmediatePropagation();
    }, true);
    on(window, 'keyup', function (event) { if (event.key === ' ') space = false; });
    on(window, 'blur', function () { space = false; cancel(); pointers.clear(); });
    function resize() {
      var next = measure();
      if (size.width === next.width && size.height === next.height) return;
      cancel(); before();
      if (camera()) { var value = camera(); states.set(scope, { x: value.x + (size.width - next.width) / value.zoom / 2, y: value.y + (size.height - next.height) / value.zoom / 2, zoom: value.zoom }); }
      size = next; notify();
    }
    var observer = new ResizeObserver(resize); observer.observe(host);
    function mount(next, key) {
      if (scope !== key) { cancel(); pointers.clear(); }
      resize();
      root = next; scope = key; host.scrollLeft = 0; host.scrollTop = 0;
      root.classList.add('spatial-scene'); root.removeAttribute('width'); root.removeAttribute('height');
      var bounds = root.getBBox(); content = { x: bounds.x, y: bounds.y, width: bounds.width || 1, height: bounds.height || 1 };
      if (!states.has(scope)) states.set(scope, initial(content, size));
      notify();
    }
    function reveal(target) {
      if (!root || !target || !camera()) return;
      var b = target.getBBox(), c = camera(), height = size.height / c.zoom, width = size.width / c.zoom;
      var x = c.x, y = c.y, padding = 24 / c.zoom;
      if (b.y < y || b.y + b.height > y + height) y = b.y - padding;
      if (b.x + b.width < x || b.x > x + width) x = b.x + b.width / 2 - width / 2;
      set({ x: x, y: y, zoom: c.zoom });
    }
    return Object.freeze({ mount: mount, fit: fitContent, reveal: reveal, state: function () { return camera() ? Object.assign({}, camera()) : null; }, dispose: function () { cancel(); observer.disconnect(); listeners.forEach(function (remove) { remove(); }); panel.remove(); host.classList.remove('spatial-viewport'); } });
  }
  global.FlowAtlasViewport = Object.freeze({ create: create, zoomAt: zoomAt, fit: fit });
}(globalThis));
