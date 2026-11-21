/* Pure immutable layout sessions for the finite v1 viewer. No DOM, clock or I/O.
 * This exchange format is deliberately distinct from the target TS platform.
 */
(function (global) {
  "use strict";
  var FORMAT = "flow-atlas/viewer-layout-1";
  var own = function (value, key) { return Object.prototype.hasOwnProperty.call(value, key); };
  var copy = function (value) { return JSON.parse(JSON.stringify(value)); };

  function canonical(value) {
    if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
    if (value && typeof value === "object") return "{" + Object.keys(value).sort().map(function (key) { return JSON.stringify(key) + ":" + canonical(value[key]); }).join(",") + "}";
    return JSON.stringify(value);
  }

  // Synchronous SHA-256 allows the same deterministic contract in classic scripts
  // and VM consumers, including offline/file contexts without WebCrypto.
  function identity(model) {
    var encoded = encodeURIComponent(canonical(model));
    var bytes = [];
    for (var i = 0; i < encoded.length; i += 1) {
      if (encoded[i] === "%") { bytes.push(parseInt(encoded.slice(i + 1, i + 3), 16)); i += 2; }
      else bytes.push(encoded.charCodeAt(i));
    }
    var length = bytes.length, bits = length * 8;
    bytes.push(128);
    while (bytes.length % 64 !== 56) bytes.push(0);
    var high = Math.floor(bits / 4294967296), low = bits >>> 0;
    for (i = 3; i >= 0; i -= 1) bytes.push((high >>> (i * 8)) & 255);
    for (i = 3; i >= 0; i -= 1) bytes.push((low >>> (i * 8)) & 255);
    var h = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    var k = [0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2];
    function rotate(value, count) { return (value >>> count) | (value << (32 - count)); }
    for (var offset = 0; offset < bytes.length; offset += 64) {
      var w = new Array(64);
      for (i = 0; i < 16; i += 1) w[i] = (bytes[offset + i * 4] << 24) | (bytes[offset + i * 4 + 1] << 16) | (bytes[offset + i * 4 + 2] << 8) | bytes[offset + i * 4 + 3];
      for (i = 16; i < 64; i += 1) {
        var s0 = rotate(w[i - 15], 7) ^ rotate(w[i - 15], 18) ^ (w[i - 15] >>> 3);
        var s1 = rotate(w[i - 2], 17) ^ rotate(w[i - 2], 19) ^ (w[i - 2] >>> 10);
        w[i] = (w[i - 16] + s0 + w[i - 7] + s1) | 0;
      }
      var a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], z = h[7];
      for (i = 0; i < 64; i += 1) {
        var t1 = (z + (rotate(e, 6) ^ rotate(e, 11) ^ rotate(e, 25)) + ((e & f) ^ (~e & g)) + k[i] + w[i]) | 0;
        var t2 = ((rotate(a, 2) ^ rotate(a, 13) ^ rotate(a, 22)) + ((a & b) ^ (a & c) ^ (b & c))) | 0;
        z = g; g = f; f = e; e = (d + t1) | 0; d = c; c = b; b = a; a = (t1 + t2) | 0;
      }
      [a, b, c, d, e, f, g, z].forEach(function (value, index) { h[index] = (h[index] + value) | 0; });
    }
    return "sha256:" + h.map(function (value) { return (value >>> 0).toString(16).padStart(8, "0"); }).join("");
  }

  function keys(value, expected, location) {
    if (!value || typeof value !== "object" || Array.isArray(value) || canonical(Object.keys(value).sort()) !== canonical(expected.slice().sort())) throw new Error(location + ": fields do not match the layout contract");
  }

  function point(value, location) {
    keys(value, ["x", "y"], location);
    if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || Math.abs(value.x) > 1000000 || Math.abs(value.y) > 1000000) throw new Error(location + ": coordinates must be finite and within ±1000000");
    return { x: value.x, y: value.y };
  }

  /** create({contentDigest, scope:{view,modeId,scenarioId}, positions}) -> session.
   * Every operation returns a new session; caller inputs are never modified.
   */
  function create(options) {
    if (typeof options.contentDigest !== "string" || !/^sha256:[a-f0-9]{64}$/.test(options.contentDigest)) throw new Error("contentDigest: expected SHA-256 identity");
    keys(options.scope, ["view", "modeId", "scenarioId"], "scope");
    if (options.scope.view !== "relations" || !options.scope.modeId || !options.scope.scenarioId || typeof options.scope.modeId !== "string" || typeof options.scope.scenarioId !== "string") throw new Error("scope: expected a relation view, mode and scenario");
    if (!options.positions || typeof options.positions !== "object" || Array.isArray(options.positions)) throw new Error("positions: expected a node map");
    var positions = Object.create(null);
    Object.keys(options.positions).forEach(function (id) {
      if (["__proto__", "constructor", "prototype"].indexOf(id) >= 0) throw new Error("positions: unsafe node identity");
      positions[id] = point(options.positions[id], "positions." + id);
    });
    var document = { format: FORMAT, contentDigest: options.contentDigest, scope: copy(options.scope), positions: positions, pinned: [] };
    return { document: document, defaults: copy(document), history: [], gesture: null };
  }

  function pinned(session, id) { return session.document.pinned.indexOf(id) >= 0; }
  function replace(session, document, history, gesture) { return { document: document, defaults: session.defaults, history: history, gesture: gesture }; }
  function cancel(session) { return session.gesture ? replace(session, session.gesture.before, session.history, null) : session; }

  /** Begin and update use absolute model-space node coordinates, not deltas. */
  function begin(session, id) {
    session = cancel(session);
    if (!own(session.document.positions, id) || pinned(session, id)) return session;
    return replace(session, session.document, session.history, { nodeId: id, before: session.document });
  }

  function update(session, position) {
    if (!session.gesture) return session;
    var document = copy(session.document);
    document.positions[session.gesture.nodeId] = point(position, "position");
    return replace(session, document, session.history, session.gesture);
  }

  function apply(session, document) {
    if (canonical(session.document) === canonical(document)) return session;
    return replace(session, document, session.history.concat([session.document]), null);
  }

  function commit(session) {
    if (!session.gesture) return session;
    var before = session.gesture.before;
    return replace(session, session.document, canonical(before) === canonical(session.document) ? session.history : session.history.concat([before]), null);
  }

  function move(session, id, position) { return commit(update(begin(session, id), position)); }
  function togglePin(session, id) {
    session = cancel(session);
    if (!own(session.document.positions, id)) return session;
    var document = copy(session.document);
    document.pinned = pinned(session, id) ? document.pinned.filter(function (item) { return item !== id; }) : document.pinned.concat([id]).sort();
    return apply(session, document);
  }
  function undo(session) {
    if (session.gesture) return cancel(session);
    if (!session.history.length) return session;
    return replace(session, session.history[session.history.length - 1], session.history.slice(0, -1), null);
  }
  function reset(session) { return apply(cancel(session), copy(session.defaults)); }
  function exportDocument(session) { return copy(cancel(session).document); }

  /** Strict import is atomic. Failure throws and leaves the caller's session intact. */
  function importDocument(session, input) {
    var document = typeof input === "string" ? JSON.parse(input) : input;
    keys(document, ["format", "contentDigest", "scope", "positions", "pinned"], "layout");
    if (document.format !== FORMAT) throw new Error("format: unsupported viewer layout version");
    if (document.contentDigest !== session.document.contentDigest) throw new Error("contentDigest: layout belongs to different or changed model content");
    keys(document.scope, ["view", "modeId", "scenarioId"], "scope");
    if (canonical(document.scope) !== canonical(session.document.scope)) throw new Error("scope: layout belongs to another view, mode or scenario");
    keys(document.positions, Object.keys(session.defaults.positions), "positions");
    Object.keys(document.positions).forEach(function (id) { point(document.positions[id], "positions." + id); });
    if (!Array.isArray(document.pinned) || document.pinned.some(function (id, index) { return typeof id !== "string" || !own(document.positions, id) || document.pinned.indexOf(id) !== index; })) throw new Error("pinned: expected unique known node identities");
    var normalized = copy(document);
    normalized.pinned.sort();
    return apply(cancel(session), normalized);
  }

  /** Preserve all default geometry until positions change; endpoint edits expire
   * only that edge's authored route hint. This is not an arrangement/router API.
   */
  function project(session, base, sizes, edges) {
    var document = session.document, routes = {};
    function moved(id) { return canonical(document.positions[id]) !== canonical(session.defaults.positions[id]); }
    Object.keys(base.routes || {}).forEach(function (id) {
      var edge = (edges || []).find(function (item) { return item.id === id; });
      if (!edge || (!moved(edge.from) && !moved(edge.to))) routes[id] = copy(base.routes[id]);
    });
    if (!Object.keys(document.positions).some(moved)) return { x: 0, y: 0, width: base.width, height: base.height, nodes: copy(document.positions), routes: routes };
    var minX = 0, minY = 0, maxX = base.width, maxY = base.height;
    Object.keys(document.positions).forEach(function (id) {
      var p = document.positions[id], size = sizes[id];
      minX = Math.min(minX, p.x - 16); minY = Math.min(minY, p.y - 28);
      maxX = Math.max(maxX, p.x + size.width + 16); maxY = Math.max(maxY, p.y + size.height + 10);
    });
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY, nodes: copy(document.positions), routes: routes };
  }

  function overlaps(boxes) {
    var ids = Object.keys(boxes), result = [];
    ids.forEach(function (id, index) {
      var a = boxes[id];
      ids.slice(index + 1).forEach(function (other) {
        var b = boxes[other];
        if (a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height) result.push({ code: "node-overlap", nodeIds: [id, other], message: "对象重叠：" + id + " / " + other });
      });
    });
    return result;
  }

  // Intersect each segment with the open card interior. Touching its boundary
  // is allowed; traveling through even the endpoint owner is diagnosed.
  function crosses(a, b, box) {
    var low = 0, high = 1, epsilon = 0.01;
    return ["x", "y"].every(function (axis) {
      var start = a[axis], delta = b[axis] - start;
      var minimum = box[axis] + epsilon, maximum = box[axis] + box[axis === "x" ? "width" : "height"] - epsilon;
      if (Math.abs(delta) < 1e-12) return start > minimum && start < maximum;
      var first = (minimum - start) / delta, last = (maximum - start) / delta;
      low = Math.max(low, Math.min(first, last)); high = Math.min(high, Math.max(first, last));
      return high > low;
    });
  }

  function routeDiagnostics(routeId, points, boxes) {
    if (!Array.isArray(points) || points.length < 2 || points.some(function (p) { return !p || !Number.isFinite(p.x) || !Number.isFinite(p.y); })) return [{ code: "route-unresolved", routeId: routeId, nodeIds: [], message: "布线未解决：" + routeId }];
    var blocked = Object.keys(boxes).filter(function (id) { return points.slice(1).some(function (p, index) { return crosses(points[index], p, boxes[id]); }); });
    return blocked.length ? [{ code: "route-obstructed", routeId: routeId, nodeIds: blocked, message: "路线穿过对象：" + routeId + " → " + blocked.join(" / ") }] : [];
  }

  global.FlowAtlasLayout = Object.freeze({ format: FORMAT, identity: identity, create: create, begin: begin, update: update, commit: commit, cancel: cancel, move: move, togglePin: togglePin, pinned: pinned, undo: undo, reset: reset, exportDocument: exportDocument, importDocument: importDocument, project: project, overlaps: overlaps, routeDiagnostics: routeDiagnostics });
}(globalThis));
