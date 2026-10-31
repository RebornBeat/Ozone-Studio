/*
 * uidocument-observer.js — the in-page, READ-ONLY live-DOM observer for the
 * UiDocument modality (pipeline 127). docs/UIDOCUMENT_RULES_GUIDE.md ("dom-live")
 * and docs/SPATIAL_GEOMETRY_LAYER.md define what it must produce.
 *
 * WRITTEN, SYNTAX-CHECKED ONLY (edits-only sweep): never run in a browser yet.
 *
 * What it does
 *   start(sink)  walk the live DOM once -> a `full` batch, then stream deltas
 *                (MutationObserver) as batches. sink(batch) is provided by the
 *                host/bridge (e.g. Playwright expose_binding).
 *   stop()       disconnect everything; ids stay stable (WeakMap) for restarts.
 *   snapshot()   a full batch on demand, WITHOUT touching tracking state.
 *
 * Batch shape (the contract with the engine, `uidocument::analyze` dom-live input):
 *   { seq, full, nodes:[Node], removed:[id], changed:[Change], meta, ... }
 *   Node   = { id, parent, tag, attrs, text, geom, style?, idx, ... }
 *   Change = { id, attrs?, text?, geom?, style?, parent?, idx?, value_len?,
 *              checked?, in_view?, ratio? }
 *   geom   = { frame:"page"|"frame:<iframeId>", x, y, w, h } (scroll-adjusted,
 *            CSS px) or null when the element has no box (honest absence).
 *   Extras: full:true + resync:true after a mutation burst overflow;
 *           burst:{count,limit,overflow}; truncated:true when the node budget
 *           was hit; sink_errors:n when earlier sink calls threw.
 *   Nodes in one batch may reference a parent that appears later in the batch.
 *
 * Hard rules (enforced by construction)
 *   - READ-ONLY: never clicks, types, focuses, scrolls, navigates, or evals;
 *     never injects nodes; no eval / Function / setAttribute / dispatchEvent.
 *   - Password inputs: NOTHING about their value is read (not even its length);
 *     they are marked `value_omitted`. `value`/`input[type=hidden]` ATTRIBUTE
 *     values are never emitted. For other text controls only `value.length`
 *     is read (the one permitted access), never the value itself.
 *   - Page text is untrusted data: capped, whitespace-collapsed, never acted on.
 *   - data-* attribute VALUES and on* handler BODIES are never emitted (names only).
 */
(function (root) {
  "use strict";

  // ───────────────────────── configuration ─────────────────────────
  var CFG = {
    MAX_TEXT: 200,          // own-text cap per element
    MAX_SHEET_TEXT: 65536,  // <style> text cap (CSS rules need it)
    MAX_ATTR: 256,          // attribute value cap
    MAX_STYLE_ATTR: 512,    // inline style attribute cap
    MAX_NODES: 5000,        // matches the host graph node cap
    FLUSH_MS: 100,          // debounce window
    BURST_N: 500,           // mutation records per flush before resync
    REMEASURE_PER_FLUSH: 400,
    SUBTREE_CAP: 300,       // descendants queued for remeasure per reflow
    SIBLING_CAP: 100,
    OBSERVE_CAP: 2000,      // ResizeObserver targets
    INTERSECT_CAP: 1000,    // IntersectionObserver targets
    EPS: 0.5                // geometry change threshold (CSS px)
  };

  var LAYOUT_ATTRS = {
    "class": 1, "style": 1, "hidden": 1, "width": 1, "height": 1, "src": 1,
    "srcset": 1, "open": 1, "rows": 1, "cols": 1, "size": 1, "colspan": 1,
    "rowspan": 1, "align": 1, "dir": 1
  };

  // ───────────────────────── pure helpers (testable without a DOM) ─────────────────────────
  function round1(n) {
    return isFinite(n) ? Math.round(n * 10) / 10 : 0;
  }

  /** Collapse whitespace and cap. len is the collapsed length. */
  function collapseText(s, max) {
    if (s === null || s === undefined) return { text: "", len: 0, truncated: false };
    var t = String(s).replace(/\s+/g, " ").replace(/^ | $/g, "");
    var len = t.length;
    if (len > max) return { text: t.slice(0, max), len: len, truncated: true };
    return { text: t, len: len, truncated: false };
  }

  /**
   * list = [[name, value], ...]. Returns { attrs, handlers, data_keys, value_omitted }.
   * Drops: input/textarea `value` attrs (any type), on* bodies (names kept),
   * data-* values (names kept), javascript: URL bodies, oversize values (capped).
   */
  function sanitizeAttrs(tag, list) {
    var attrs = {}, handlers = [], dataKeys = [], valueOmitted = false;
    var lowerTag = String(tag || "").toLowerCase();
    var type = "";
    for (var i = 0; i < list.length; i++) {
      if (String(list[i][0]).toLowerCase() === "type") type = String(list[i][1]).toLowerCase();
    }
    for (var j = 0; j < list.length; j++) {
      var n = String(list[j][0]).toLowerCase();
      var v = list[j][1] === null || list[j][1] === undefined ? "" : String(list[j][1]);
      if (/^on[a-z]/.test(n)) { handlers.push(n); continue; }
      if (n.indexOf("data-") === 0) { dataKeys.push(n); continue; }
      if ((lowerTag === "input" || lowerTag === "textarea") && n === "value") { valueOmitted = true; continue; }
      if ((n === "href" || n === "src" || n === "action" || n === "formaction") && /^\s*javascript:/i.test(v)) v = "javascript:";
      var cap = n === "style" ? CFG.MAX_STYLE_ATTR : CFG.MAX_ATTR;
      if (v.length > cap) v = v.slice(0, cap);
      attrs[n] = v;
    }
    var out = { attrs: attrs, handlers: handlers, data_keys: dataKeys, value_omitted: valueOmitted };
    if (lowerTag === "input" && type === "password") out.password = true;
    return out;
  }

  function boxFrom(rect, scrollX, scrollY, frame) {
    return {
      frame: frame,
      x: round1(rect.left + (scrollX || 0)),
      y: round1(rect.top + (scrollY || 0)),
      w: round1(rect.width),
      h: round1(rect.height)
    };
  }

  function boxChanged(a, b, eps) {
    var e = eps === undefined ? CFG.EPS : eps;
    if (!a && !b) return false;
    if (!a || !b) return true;
    if (a.frame !== b.frame) return true;
    return Math.abs(a.x - b.x) > e || Math.abs(a.y - b.y) > e ||
           Math.abs(a.w - b.w) > e || Math.abs(a.h - b.h) > e;
  }

  function styleChanged(a, b) {
    if (!a && !b) return false;
    if (!a || !b) return true;
    return JSON.stringify(a) !== JSON.stringify(b);
  }

  /** Stable ids: a WeakMap + counter. ids start at 1; 0/null mean "no node". */
  function makeIdAllocator() {
    var map = new WeakMap();
    var next = 1;
    return {
      get: function (obj) {
        var id = map.get(obj);
        if (id === undefined) { id = next++; map.set(obj, id); }
        return id;
      },
      peek: function (obj) { var id = map.get(obj); return id === undefined ? null : id; },
      count: function () { return next - 1; }
    };
  }

  function summarizeBurst(count, limit) {
    return { count: count, limit: limit, overflow: count > limit };
  }

  function isLayoutAttr(name) {
    return LAYOUT_ATTRS[String(name).toLowerCase()] === 1;
  }

  function makeBatch(seq, full, nodes, removed, changed, extra) {
    var b = { seq: seq, full: !!full, nodes: nodes || [], removed: removed || [], changed: changed || [] };
    if (extra) { for (var k in extra) { if (Object.prototype.hasOwnProperty.call(extra, k)) b[k] = extra[k]; } }
    return b;
  }

  var pure = {
    CFG: CFG, round1: round1, collapseText: collapseText, sanitizeAttrs: sanitizeAttrs,
    boxFrom: boxFrom, boxChanged: boxChanged, styleChanged: styleChanged,
    makeIdAllocator: makeIdAllocator, summarizeBurst: summarizeBurst,
    isLayoutAttr: isLayoutAttr, makeBatch: makeBatch
  };

  // ───────────────────────── the DOM-bound observer ─────────────────────────
  function createObserver(win) {
    var doc = win.document;
    var ids = makeIdAllocator();
    var tracked = new Set();               // elements currently in the graph
    var lastGeom = new WeakMap();
    var lastStyle = new WeakMap();
    var lastParent = new WeakMap();
    var frameOwner = new Map();            // Document -> owning <iframe> element
    var observedRoots = new WeakSet();     // docs/shadow roots under the MutationObserver
    var fixedSet = new Set();              // fixed/sticky tracked elements (scroll remeasure)
    var queue = new Set();                 // elements awaiting remeasure
    var seq = 0, sink = null, running = false, timer = null, sinkErrors = 0;
    var mo = null, ro = null, io = null, lastActive = undefined, pending = newPending();
    var roCount = 0, ioCount = 0;
    var MO_OPTS = { childList: true, subtree: true, attributes: true, characterData: true };

    function newPending() {
      return { count: 0, overflow: false, added: new Set(), removed: new Set(), attrs: new Set(),
               text: new Set(), reflow: new Set(), state: new Set(), inview: [] };
    }

    // ── structure helpers ──
    function isFormControl(tag) { return tag === "input" || tag === "textarea" || tag === "select"; }

    function logicalParent(el) {
      var p = el.parentNode;
      if (!p) return null;
      if (p.nodeType === 11 && p.host) return p.host;                 // ShadowRoot -> host
      if (p.nodeType === 9) return frameOwner.get(p) || null;         // iframe document -> <iframe>
      return p.nodeType === 1 ? p : null;
    }

    function inShadow(el) {
      var n = el;
      while (n) {
        var p = n.parentNode;
        if (!p) return false;
        if (p.nodeType === 11 && p.host) return true;
        n = p.nodeType === 1 ? p : null;
      }
      return false;
    }

    function idxOf(el) {
      var i = 0, s = el.previousElementSibling;
      while (s && i < 10000) { i++; s = s.previousElementSibling; }
      return i;
    }

    function frameOf(el) {
      var d = el.ownerDocument;
      if (d === doc) return "page";
      var owner = frameOwner.get(d);
      return owner ? "frame:" + ids.get(owner) : "page";
    }

    // ── measurement (geometry is captured WITH the node, in a named frame) ──
    function measure(el) {
      var w = el.ownerDocument && el.ownerDocument.defaultView;
      if (!w || !el.getClientRects || el.getClientRects().length === 0) return null;
      return boxFrom(el.getBoundingClientRect(), w.pageXOffset || 0, w.pageYOffset || 0, frameOf(el));
    }

    function styleOf(el) {
      var w = el.ownerDocument && el.ownerDocument.defaultView;
      if (!w || !w.getComputedStyle) return null;
      var cs = w.getComputedStyle(el);
      var s = {
        display: cs.display, visibility: cs.visibility, opacity: cs.opacity,
        position: cs.position, zIndex: cs.zIndex, pointerEvents: cs.pointerEvents,
        cursor: cs.cursor, overflow: cs.overflow
      };
      if (cs.transform && cs.transform !== "none") s.transform = cs.transform;
      return s;
    }

    function ownText(el, tag) {
      if (tag === "textarea") return { text: "", len: 0, truncated: false };   // default value is user data
      var out = "";
      var kids = el.childNodes;
      for (var i = 0; i < kids.length; i++) {
        if (kids[i].nodeType === 3) out += kids[i].nodeValue;
        if (out.length > CFG.MAX_SHEET_TEXT * 2) break;
      }
      return collapseText(out, tag === "style" ? CFG.MAX_SHEET_TEXT : CFG.MAX_TEXT);
    }

    function attrList(el) {
      var list = [], a = el.attributes;
      for (var i = 0; i < a.length; i++) list.push([a[i].name, a[i].value]);
      return list;
    }

    /** value_len / checked / selection: the ONLY form-state reads. Never password/hidden/file. */
    function formState(el, tag, san) {
      var out = {};
      if (!isFormControl(tag)) return out;
      if (san.password) { out.value_omitted = true; return out; }
      var type = (el.type || "").toLowerCase();
      if (tag === "input" && (type === "hidden" || type === "file")) { out.value_omitted = true; return out; }
      if (tag === "select") {
        out.options = el.options ? el.options.length : 0;
        out.selected_index = el.selectedIndex;
        return out;
      }
      if (type === "checkbox" || type === "radio") { out.checked = !!el.checked; return out; }
      if (typeof el.value === "string") out.value_len = el.value.length;
      return out;
    }

    function describe(el, idx, withStyle) {
      var tag = el.localName || String(el.tagName || "").toLowerCase();
      var san = sanitizeAttrs(tag, attrList(el));
      var t = ownText(el, tag);
      var parentEl = logicalParent(el);
      var node = {
        id: ids.get(el),
        parent: parentEl ? ids.get(parentEl) : null,
        tag: tag,
        attrs: san.attrs,
        text: t.text,
        geom: measure(el),
        idx: idx
      };
      if (t.truncated) { node.text_len = t.len; node.text_truncated = true; }
      if (el.namespaceURI && el.namespaceURI.indexOf("svg") >= 0) node.ns = "svg";
      else if (el.namespaceURI && el.namespaceURI.indexOf("MathML") >= 0) node.ns = "math";
      if (san.handlers.length) node.handlers = san.handlers;
      if (san.data_keys.length) node.data_keys = san.data_keys;
      if (san.value_omitted) node.value_omitted = true;
      var fs = formState(el, tag, san);
      for (var k in fs) { if (Object.prototype.hasOwnProperty.call(fs, k)) node[k] = fs[k]; }
      if (inShadow(el)) node.shadow = true;
      if (el.shadowRoot) node.has_shadow = true;                    // open root only
      if (withStyle !== false) {
        var st = styleOf(el);
        if (st) node.style = st;
      }
      if (el.ownerDocument !== doc) node.in_frame = frameOf(el);
      return node;
    }

    // ── iframes ──
    function frameDocOf(iframe) {
      try { return iframe.contentDocument || null; } catch (e) { return null; }
    }

    function attachFrame(iframe) {
      var d = frameDocOf(iframe);
      if (!d || !d.documentElement) return null;
      frameOwner.set(d, iframe);
      observeRoot(d.documentElement);
      return d;
    }

    function observeRoot(rootNode) {
      if (!mo || observedRoots.has(rootNode)) return;
      observedRoots.add(rootNode);
      try { mo.observe(rootNode, MO_OPTS); } catch (e) { /* detached or cross-realm: skip */ }
    }

    // ── tracking ──
    function track(el) {
      tracked.add(el);
      lastParent.set(el, logicalParent(el));
      if (el.ownerDocument === doc) {
        if (ro && roCount < CFG.OBSERVE_CAP) { try { ro.observe(el); roCount++; } catch (e) {} }
        if (io && ioCount < CFG.INTERSECT_CAP) { try { io.observe(el); ioCount++; } catch (e) {} }
      }
    }

    function untrack(el) {
      tracked.delete(el);
      queue.delete(el);
      fixedSet.delete(el);
      if (ro) { try { ro.unobserve(el); } catch (e) {} }
      if (io) { try { io.unobserve(el); } catch (e) {} }
    }

    function remember(el, node) {
      lastGeom.set(el, node.geom || null);
      lastStyle.set(el, node.style || null);
      if (node.style && (node.style.position === "fixed" || node.style.position === "sticky")) fixedSet.add(el);
    }

    /** Iterative DFS (no recursion limits). opts.track registers elements. */
    function walk(startEl, startIdx, opts, out) {
      var stack = [{ el: startEl, idx: startIdx }];
      while (stack.length) {
        var cur = stack.pop();
        var el = cur.el;
        if (opts.track && tracked.has(el) && !opts.force) continue;     // already in the graph
        if (out.nodes.length >= CFG.MAX_NODES) { out.truncated = true; continue; }
        var node = describe(el, cur.idx, true);
        var tag = node.tag;
        if (tag === "iframe" || tag === "frame") {
          var fd = opts.track ? attachFrame(el) : frameDocOf(el);
          if (fd && !opts.track && fd.documentElement) frameOwner.set(fd, el);   // frame naming only; no observing
          if (!fd) { node.opaque = true; node.cross_origin = true; }     // honest stub
        }
        out.nodes.push(node);
        if (opts.track) { track(el); remember(el, node); }
        // children, in document order (pushed reversed). Shadow children after light children.
        var kids = [];
        var ch = el.children;
        for (var i = 0; i < ch.length; i++) kids.push({ el: ch[i], idx: i });
        if (el.shadowRoot) {
          if (opts.track) observeRoot(el.shadowRoot);
          var sc = el.shadowRoot.children;
          for (var s = 0; s < sc.length; s++) kids.push({ el: sc[s], idx: s });
        }
        if ((tag === "iframe" || tag === "frame") && !node.opaque) {
          var d2 = frameDocOf(el);
          if (d2 && d2.documentElement) kids.push({ el: d2.documentElement, idx: 0 });
        }
        for (var k = kids.length - 1; k >= 0; k--) stack.push(kids[k]);
      }
    }

    function viewportMeta() {
      var de = doc.documentElement;
      return {
        url: win.location ? String(win.location.href).slice(0, 2048) : "",
        vw: win.innerWidth || 0, vh: win.innerHeight || 0,
        sx: win.pageXOffset || 0, sy: win.pageYOffset || 0,
        dpr: win.devicePixelRatio || 1,
        doc_w: de ? de.scrollWidth : 0, doc_h: de ? de.scrollHeight : 0
      };
    }

    function activeId() {
      var a = doc.activeElement;
      return a && tracked.has(a) ? ids.get(a) : null;
    }

    function fullBatch(trackIt, extra) {
      var out = { nodes: [], truncated: false };
      if (!doc.documentElement) return makeBatch(seq, true, [], [], [], extra);
      walk(doc.documentElement, 0, { track: trackIt, force: false }, out);
      var meta = viewportMeta();
      meta.title = String(doc.title || "").slice(0, 200);
      meta.lang = doc.documentElement.getAttribute("lang") || "";
      meta.active = trackIt ? activeId() : (doc.activeElement && ids.peek(doc.activeElement));
      var b = makeBatch(seq, true, out.nodes, [], [], extra);
      b.meta = meta;
      if (out.truncated) { b.truncated = true; b.node_budget = CFG.MAX_NODES; }
      return b;
    }

    // ── emitting ──
    function emit(batch) {
      if (!sink) return;
      seq += 1;
      batch.seq = seq;
      if (sinkErrors) batch.sink_errors = sinkErrors;
      try {
        var r = sink(batch);
        if (r && typeof r.catch === "function") r.catch(function () { sinkErrors++; });
      } catch (e) { sinkErrors++; }
    }

    function schedule() {
      if (timer !== null || !running) return;
      timer = win.setTimeout(flush, CFG.FLUSH_MS);
    }

    // ── removal ──
    function collectRemoved(rootEl, outIds) {
      var stack = [rootEl];
      while (stack.length) {
        var el = stack.pop();
        if (tracked.has(el)) { outIds.push(ids.get(el)); untrack(el); }
        var ch = el.children;
        for (var i = 0; i < ch.length; i++) stack.push(ch[i]);
        if (el.shadowRoot) {
          var sc = el.shadowRoot.children;
          for (var s = 0; s < sc.length; s++) stack.push(sc[s]);
        }
        var tag = el.localName;
        if (tag === "iframe" || tag === "frame") {
          var fd = frameDocOf(el);
          if (fd && fd.documentElement) { stack.push(fd.documentElement); frameOwner.delete(fd); }
        }
      }
    }

    // ── reflow queueing (bounded) ──
    function queueSubtree(el) {
      if (!tracked.has(el)) return;
      queue.add(el);
      var n = 0, stack = [el];
      while (stack.length && n < CFG.SUBTREE_CAP) {
        var cur = stack.pop();
        var ch = cur.children;
        for (var i = 0; i < ch.length && n < CFG.SUBTREE_CAP; i++) {
          if (tracked.has(ch[i])) { queue.add(ch[i]); n++; stack.push(ch[i]); }
        }
      }
    }

    function queueReflow(el) {
      if (!tracked.has(el)) return;
      queueSubtree(el);
      var s = el.nextElementSibling, c = 0;
      while (s && c < CFG.SIBLING_CAP) { if (tracked.has(s)) queue.add(s); s = s.nextElementSibling; c++; }
      var p = el.parentElement;
      if (p && tracked.has(p)) queue.add(p);
    }

    function queueAll() {
      tracked.forEach(function (el) { queue.add(el); });
    }

    // ── the flush: reconcile pending records against the live tree ──
    function flush() {
      timer = null;
      if (!running) return;
      var p = pending;
      pending = newPending();
      if (p.overflow) { resync(summarizeBurst(p.count, CFG.BURST_N)); return; }

      var removedIds = [], newNodes = [], changedMap = new Map();
      function chg(el) {
        var id = ids.get(el);
        var c = changedMap.get(id);
        if (!c) { c = { id: id }; changedMap.set(id, c); }
        return c;
      }

      // 1) removals (state-based: only if really disconnected now)
      p.removed.forEach(function (n) { if (!n.isConnected) collectRemoved(n, removedIds); });

      // 2) additions / moves
      var addOut = { nodes: [], truncated: false };
      p.added.forEach(function (n) {
        if (!n.isConnected) return;
        var parentEl = logicalParent(n);
        if (parentEl && !tracked.has(parentEl)) return;                 // an ancestor's walk will cover it
        if (tracked.has(n)) {                                           // a move
          if (lastParent.get(n) !== parentEl) {
            var c = chg(n);
            c.parent = parentEl ? ids.get(parentEl) : null;
            c.idx = idxOf(n);
            lastParent.set(n, parentEl);
            queueReflow(n);
          }
          return;
        }
        walk(n, idxOf(n), { track: true, force: false }, addOut);
        queueReflow(n);
      });
      newNodes = addOut.nodes;

      // 3) attribute / text / form-state changes on already-tracked elements
      var newIds = {};
      for (var i = 0; i < newNodes.length; i++) newIds[newNodes[i].id] = 1;
      p.attrs.forEach(function (el) {
        if (!tracked.has(el) || newIds[ids.get(el)]) return;
        var tag = el.localName;
        var san = sanitizeAttrs(tag, attrList(el));
        var c = chg(el);
        c.attrs = san.attrs;
        if (san.handlers.length) c.handlers = san.handlers;
        if (san.data_keys.length) c.data_keys = san.data_keys;
        var st = styleOf(el);
        if (st) { c.style = st; lastStyle.set(el, st); }
        var fs = formState(el, tag, san);
        for (var k in fs) { if (Object.prototype.hasOwnProperty.call(fs, k)) c[k] = fs[k]; }
      });
      p.text.forEach(function (el) {
        if (!tracked.has(el) || newIds[ids.get(el)]) return;
        var t = ownText(el, el.localName);
        var c = chg(el);
        c.text = t.text;
        if (t.truncated) { c.text_len = t.len; c.text_truncated = true; }
      });
      p.state.forEach(function (el) {
        if (!tracked.has(el) || newIds[ids.get(el)]) return;
        var tag = el.localName;
        var san = sanitizeAttrs(tag, attrList(el));
        var fs = formState(el, tag, san);
        var c = chg(el);
        for (var k in fs) { if (Object.prototype.hasOwnProperty.call(fs, k)) c[k] = fs[k]; }
      });
      p.reflow.forEach(function (el) { queueReflow(el); });
      for (var v = 0; v < p.inview.length; v++) {
        var iv = p.inview[v];
        if (tracked.has(iv.el)) { var cv = chg(iv.el); cv.in_view = iv.in_view; cv.ratio = iv.ratio; }
      }

      // 4) rolling remeasure (geometry + computed style), bounded per flush
      var done = 0, more = false;
      var it = queue.values();
      var batchEls = [];
      for (var nx = it.next(); !nx.done; nx = it.next()) {
        if (done >= CFG.REMEASURE_PER_FLUSH) { more = true; break; }
        batchEls.push(nx.value);
        done++;
      }
      for (var b = 0; b < batchEls.length; b++) {
        var e = batchEls[b];
        queue.delete(e);
        if (!tracked.has(e) || !e.isConnected || newIds[ids.get(e)]) continue;
        var g = measure(e), s2 = styleOf(e);
        var gc = boxChanged(lastGeom.get(e) || null, g);
        var sc2 = styleChanged(lastStyle.get(e) || null, s2);
        if (gc || sc2) {
          var cc = chg(e);
          if (gc) { cc.geom = g; lastGeom.set(e, g); }
          if (sc2 && s2) {
            cc.style = s2; lastStyle.set(e, s2);
            if (s2.position === "fixed" || s2.position === "sticky") fixedSet.add(e); else fixedSet.delete(e);
          }
        }
      }
      if (more || queue.size) schedule();

      // 5) emit
      var changed = [];
      changedMap.forEach(function (c) { changed.push(c); });
      var meta = viewportMeta();
      var act = activeId();
      var activeChanged = act !== lastActive;
      if (activeChanged) { meta.active = act; lastActive = act; }
      if (!newNodes.length && !removedIds.length && !changed.length && !activeChanged) return;   // nothing to say
      var extra = { meta: meta };
      if (p.count > 20) extra.burst = summarizeBurst(p.count, CFG.BURST_N);
      if (addOut.truncated) { extra.truncated = true; extra.node_budget = CFG.MAX_NODES; }
      emit(makeBatch(seq, false, newNodes, removedIds, changed, extra));
    }

    /** A burst overflowed the record budget: rebuild from the live tree (full, resync). */
    function resync(burst) {
      tracked.forEach(function (el) { untrack(el); });
      tracked.clear(); queue.clear(); fixedSet.clear();
      roCount = 0; ioCount = 0;
      var b = fullBatch(true, { resync: true, burst: burst });
      lastActive = b.meta ? b.meta.active : null;
      emit(b);
    }

    // ── observers and listeners ──
    function onMutations(records) {
      for (var i = 0; i < records.length; i++) {
        pending.count++;
        if (pending.count > CFG.BURST_N) { pending.overflow = true; continue; }
        var r = records[i];
        if (r.type === "childList") {
          var t = r.target;
          for (var a = 0; a < r.addedNodes.length; a++) {
            var an = r.addedNodes[a];
            if (an.nodeType === 1) pending.added.add(an); else if (an.nodeType === 3) pending.text.add(t);
          }
          for (var d = 0; d < r.removedNodes.length; d++) {
            var rn = r.removedNodes[d];
            if (rn.nodeType === 1) pending.removed.add(rn); else if (rn.nodeType === 3) pending.text.add(t);
          }
          if (t.nodeType === 1) pending.reflow.add(t);
        } else if (r.type === "attributes") {
          pending.attrs.add(r.target);
          if (isLayoutAttr(r.attributeName)) pending.reflow.add(r.target);
        } else if (r.type === "characterData") {
          var pe = r.target.parentElement;
          if (pe) { pending.text.add(pe); pending.reflow.add(pe); }
        }
      }
      schedule();
    }

    function onResize(entries) {
      for (var i = 0; i < entries.length; i++) queueSubtree(entries[i].target);
      schedule();
    }

    function onIntersect(entries) {
      for (var i = 0; i < entries.length; i++) {
        pending.inview.push({ el: entries[i].target, in_view: !!entries[i].isIntersecting,
                              ratio: round1(entries[i].intersectionRatio * 100) / 100 });
        queue.add(entries[i].target);
      }
      schedule();
    }

    function onScroll(ev) {
      var t = ev.target;
      fixedSet.forEach(function (el) { queue.add(el); });
      if (t && t.nodeType === 1 && tracked.has(t)) queueSubtree(t);
      schedule();
    }

    function onWindowResize() { queueAll(); schedule(); }

    function onFormEvent(ev) {
      var t = ev.target;
      if (t && t.nodeType === 1 && tracked.has(t)) { pending.state.add(t); schedule(); }
    }

    function onFocus() { schedule(); }

    function onFrameLoad(ev) {
      var f = ev.target;
      if (!f || f.nodeType !== 1 || !tracked.has(f)) return;
      // iframe navigated: drop its old subtree, re-walk it.
      var gone = [];
      var d = frameDocOf(f);
      var ch = f.children; for (var i = 0; i < ch.length; i++) collectRemoved(ch[i], gone);
      if (d && d.documentElement) {
        // remove anything previously tracked inside any prior document of this frame
        tracked.forEach(function (el) { if (el.ownerDocument !== doc && frameOwner.get(el.ownerDocument) === f && !el.isConnected) { gone.push(ids.get(el)); untrack(el); } });
        attachFrame(f);
        var out = { nodes: [], truncated: false };
        walk(d.documentElement, 0, { track: true, force: false }, out);
        var meta = viewportMeta();
        emit(makeBatch(seq, false, out.nodes, gone, [], { meta: meta }));
      } else if (gone.length) {
        emit(makeBatch(seq, false, [], gone, [], { meta: viewportMeta() }));
      }
    }

    function addListeners() {
      win.addEventListener("scroll", onScroll, { capture: true, passive: true });
      win.addEventListener("resize", onWindowResize, { passive: true });
      doc.addEventListener("input", onFormEvent, { capture: true, passive: true });
      doc.addEventListener("change", onFormEvent, { capture: true, passive: true });
      doc.addEventListener("focusin", onFocus, { capture: true, passive: true });
      doc.addEventListener("focusout", onFocus, { capture: true, passive: true });
      doc.addEventListener("load", onFrameLoad, { capture: true });    // iframe load events capture here
    }

    function removeListeners() {
      win.removeEventListener("scroll", onScroll, { capture: true });
      win.removeEventListener("resize", onWindowResize);
      doc.removeEventListener("input", onFormEvent, { capture: true });
      doc.removeEventListener("change", onFormEvent, { capture: true });
      doc.removeEventListener("focusin", onFocus, { capture: true });
      doc.removeEventListener("focusout", onFocus, { capture: true });
      doc.removeEventListener("load", onFrameLoad, { capture: true });
    }

    function begin() {
      if (!running) return;
      mo = new win.MutationObserver(onMutations);
      if (win.ResizeObserver) ro = new win.ResizeObserver(onResize);
      if (win.IntersectionObserver) io = new win.IntersectionObserver(onIntersect, { threshold: [0, 1] });
      observeRoot(doc.documentElement);
      addListeners();
      var b = fullBatch(true, null);
      lastActive = b.meta ? b.meta.active : null;
      emit(b);
    }

    // ── public API ──
    function start(newSink) {
      if (typeof newSink !== "function") throw new TypeError("start(sink): sink must be a function");
      if (running) { sink = newSink; return; }
      sink = newSink;
      running = true;
      if (doc.documentElement) { begin(); return; }
      var onReady = function () {
        doc.removeEventListener("readystatechange", onReady);
        if (running && !mo) begin();
      };
      doc.addEventListener("readystatechange", onReady);
    }

    function stop() {
      running = false;
      if (timer !== null) { win.clearTimeout(timer); timer = null; }
      if (mo) { mo.disconnect(); mo = null; }
      if (ro) { ro.disconnect(); ro = null; }
      if (io) { io.disconnect(); io = null; }
      removeListeners();
      tracked.clear(); queue.clear(); fixedSet.clear(); frameOwner.clear();
      observedRoots = new WeakSet();
      roCount = 0; ioCount = 0;
      pending = newPending();
      sink = null;
    }

    function snapshot() {
      return fullBatch(false, { snapshot: true });
    }

    return { start: start, stop: stop, snapshot: snapshot, version: "0.1.0", pure: pure };
  }

  // ───────────────────────── wiring ─────────────────────────
  if (typeof root.document !== "undefined" && root.window === root) {
    var api = createObserver(root);
    root.__uidocument = api;
    // Auto-start when the bridge exposed a binding named __uidocument_emit
    // (Playwright expose_binding("__uidocument_emit", fn) before this script runs).
    if (typeof root.__uidocument_emit === "function") {
      try { api.start(root.__uidocument_emit); } catch (e) { /* host can start() manually */ }
    }
  } else {
    // No DOM (e.g. node selftest): expose only the pure helpers.
    root.__uidocument_pure = pure;
  }
})(typeof globalThis !== "undefined" ? globalThis : this);
