// Revel — script du canevas de l'éditeur.
//
// Injecté par Revel dans chaque page affichée dans l'éditeur (jamais dans le site publié).
// Les éléments portent `data-rv="<identifiant du nœud>"` ; les textes, expressions et composants
// sont entourés de commentaires `<!--rv:N-->` … `<!--/rv:N-->` décrits par la liste `#rv-slots`.
// Échanges avec l'interface de Revel (fenêtre parente) par `postMessage`, champ `revel: 1`.
(() => {
  "use strict";
  if (window.__revelCanvas) return;
  window.__revelCanvas = true;

  const slotsEl = document.getElementById("rv-slots");
  let slots = [];
  try { slots = JSON.parse(slotsEl ? slotsEl.textContent : "[]"); } catch (_) { slots = []; }

  // La page doit rester identique au site généré : les éléments ajoutés par Revel sont retirés
  // (sinon `:last-child` changerait), et les commentaires repères sont remplacés par des plages
  // (Range) vivantes, puis les textes adjacents fusionnés comme dans le site (sinon la mise en
  // forme du texte pourrait varier d'une fraction de pixel).
  if (slotsEl) slotsEl.remove();
  if (document.currentScript) document.currentScript.remove();
  const ranges = [];
  {
    const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_COMMENT);
    const open = new Map();
    const marks = [];
    while (walker.nextNode()) {
      const c = walker.currentNode;
      const m = /^(\/?)rv:(\d+)$/.exec(c.data);
      if (!m) continue;
      marks.push(c);
      const n = Number(m[2]);
      if (!m[1]) {
        open.set(n, c);
      } else if (open.has(n)) {
        const r = document.createRange();
        r.setStartAfter(open.get(n));
        r.setEndBefore(c);
        ranges[n] = r;
      }
    }
    const parents = new Set();
    for (const c of marks) {
      if (c.parentNode) parents.add(c.parentNode);
      c.remove();
    }
    for (const p of parents) p.normalize();
  }
  /** Plage du repère `n` dans la page (texte, expression, composant). */
  const slotRange = (n) => ranges[n] || null;
  window.__revelSlot = (n) => ({ slot: slots[n], range: slotRange(n) });

  const post = (msg) => window.parent.postMessage(Object.assign({ revel: 1 }, msg), "*");

  // ---------------------------------------------------------------- calque de sélection
  // Dans un shadow DOM fermé : les styles du site ne le touchent pas, et inversement.
  const host = document.createElement("revel-overlay");
  const root = host.attachShadow({ mode: "closed" });
  root.innerHTML =
    "<style>" +
    ":host{all:initial}" +
    ".box{position:fixed;box-sizing:border-box;pointer-events:none;display:none;z-index:2147483646}" +
    ".hover{outline:1px solid #6d8cff}" +
    ".sel{outline:2px solid #6d8cff}" +
    ".tag{position:fixed;pointer-events:none;display:none;z-index:2147483647;" +
    "font:600 11px/18px system-ui,-apple-system,'Segoe UI',sans-serif;background:#6d8cff;color:#fff;" +
    "padding:0 6px;border-radius:3px;white-space:nowrap;max-width:60vw;overflow:hidden;text-overflow:ellipsis}" +
    "</style><div class='box hover'></div><div class='box sel'></div><div class='tag'></div>";
  const [hoverBox, selBox, tagBox] = root.querySelectorAll(".hover, .sel, .tag");
  document.documentElement.appendChild(host);

  let hovered = null;
  let selected = null;

  const label = (el) => {
    const cls = typeof el.className === "string" && el.className.trim()
      ? "." + el.className.trim().split(/\s+/).slice(0, 2).join(".")
      : "";
    return el.tagName.toLowerCase() + cls;
  };

  const place = (box, el) => {
    if (!el || !el.isConnected) { box.style.display = "none"; return null; }
    const r = el.getBoundingClientRect();
    box.style.display = "block";
    box.style.left = r.left + "px";
    box.style.top = r.top + "px";
    box.style.width = r.width + "px";
    box.style.height = r.height + "px";
    return r;
  };

  const draw = () => {
    place(hoverBox, hovered && hovered !== selected ? hovered : null);
    const r = place(selBox, selected);
    if (r && selected) {
      tagBox.textContent = label(selected);
      tagBox.style.display = "block";
      tagBox.style.left = Math.max(0, r.left) + "px";
      tagBox.style.top = (r.top >= 20 ? r.top - 20 : r.bottom + 2) + "px";
    } else {
      tagBox.style.display = "none";
    }
  };

  let frame = 0;
  const redraw = () => {
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; draw(); });
  };

  const rectOf = (el) => {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top, width: r.width, height: r.height };
  };

  // Élément sélectionnable sous le pointeur : le plus proche repéré ; un dessin SVG est
  // sélectionné en entier (pas ses tracés).
  const targetOf = (node) => {
    if (!node || !node.closest) return null;
    let svg = node.closest("svg");
    while (svg && svg.parentElement && svg.parentElement.closest("svg")) svg = svg.parentElement.closest("svg");
    return (svg || node).closest("[data-rv]");
  };

  // Valeurs calculées des propriétés suivies par le panneau de styles, pour l'élément sélectionné.
  let watched = [];
  let computedTimer = 0;
  const sendComputed = () => {
    clearTimeout(computedTimer);
    computedTimer = setTimeout(() => {
      if (!selected || !selected.isConnected || !watched.length) return;
      const cs = getComputedStyle(selected);
      const values = {};
      for (const p of watched) values[p] = cs.getPropertyValue(p).trim();
      post({ type: "computed", id: selected.getAttribute("data-rv"), values });
    }, 30);
  };

  const select = (el, notify) => {
    selected = el;
    sendComputed();
    redraw();
    if (notify) {
      post(el
        ? { type: "select", id: el.getAttribute("data-rv"), tag: el.tagName.toLowerCase(), label: label(el), rect: rectOf(el) }
        : { type: "select", id: null });
    }
  };

  // ---------------------------------------------------------------- événements de la page
  document.addEventListener("mousemove", (e) => {
    const t = targetOf(e.target);
    if (t !== hovered) {
      hovered = t;
      redraw();
      post({ type: "hover", id: t ? t.getAttribute("data-rv") : null });
    }
  }, true);

  document.addEventListener("mouseleave", () => {
    hovered = null;
    redraw();
    post({ type: "hover", id: null });
  });

  // ---------------------------------------------------------------- textes modifiés sur place
  // Double-clic sur un texte de la page ou un texte lié à sa source (D-19) : le texte devient
  // modifiable ; Entrée ou un clic ailleurs valide, Échap annule. L'interface reçoit le texte et
  // envoie la commande (SetText ou SetBound), puis la page est rechargée.
  let editing = null;
  const editable = (s) => !!s && (s.slot === "text" || (s.slot === "expr" && !!s.binding && !s.transformed && !s.html));
  const slotAt = (x, y) => {
    let best = null;
    let bestArea = Infinity;
    for (let n = 0; n < ranges.length; n++) {
      const r = ranges[n];
      if (!r || !editable(slots[n])) continue;
      for (const b of r.getClientRects()) {
        if (x >= b.left - 1 && x <= b.right + 1 && y >= b.top - 1 && y <= b.bottom + 1) {
          const area = b.width * b.height;
          if (area < bestArea) { best = n; bestArea = area; }
        }
      }
    }
    return best;
  };
  const inEdit = (t) => !!editing && !!t && editing.span.contains(t);
  const startEdit = (n) => {
    const r = ranges[n];
    const span = document.createElement("span");
    span.setAttribute("data-rv-edit", "");
    span.setAttribute("contenteditable", "plaintext-only");
    span.style.outline = "2px solid #6d8cff";
    span.style.outlineOffset = "2px";
    try { r.surroundContents(span); } catch (_) { return false; }
    editing = { n, span, before: span.textContent };
    span.addEventListener("blur", () => finishEdit(true), { once: true });
    span.focus();
    const sel = window.getSelection();
    if (sel) sel.selectAllChildren(span);
    post({ type: "editing", slot: slots[n] });
    return true;
  };
  const finishEdit = (commit) => {
    if (!editing) return;
    const { n, span, before } = editing;
    editing = null;
    const text = span.textContent;
    const parent = span.parentNode;
    if (!parent) return;
    const node = document.createTextNode(commit ? text : before);
    parent.replaceChild(node, span);
    const r = document.createRange();
    r.selectNode(node);
    ranges[n] = r;
    parent.normalize();
    if (commit && text !== before) post({ type: "text", slot: slots[n], text, before });
    else post({ type: "editing", slot: null });
  };

  // Les clics sélectionnent : liens, boutons et formulaires du site ne réagissent pas.
  const block = (e) => { e.preventDefault(); e.stopPropagation(); };
  document.addEventListener("click", (e) => {
    if (inEdit(e.target)) return;
    if (editing) finishEdit(true);
    block(e);
    select(targetOf(e.target), true);
  }, true);
  document.addEventListener("dblclick", (e) => {
    if (inEdit(e.target)) return;
    block(e);
    const n = slotAt(e.clientX, e.clientY);
    if (n !== null) startEdit(n);
  }, true);
  for (const type of ["auxclick", "submit", "mouseup", "pointerup"]) {
    document.addEventListener(type, (e) => { if (!inEdit(e.target)) block(e); }, true);
  }
  // Appui : pas de sélection de texte ni de focus d'un champ du site, mais le canevas reçoit le
  // clavier (Échap, raccourcis).
  for (const type of ["mousedown", "pointerdown"]) {
    document.addEventListener(type, (e) => {
      if (inEdit(e.target)) return;
      block(e);
      window.focus();
    }, true);
  }

  document.addEventListener("keydown", (e) => {
    if (editing) {
      if (e.key === "Escape") { e.preventDefault(); finishEdit(false); }
      else if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); finishEdit(true); }
      return;
    }
    if (e.key === "Escape") { select(null, true); return; }
    // Raccourcis de l'éditeur (annuler, rétablir, dupliquer, supprimer…) transmis à l'interface.
    if (e.ctrlKey || e.metaKey || e.key === "Delete" || e.key === "Backspace") {
      post({ type: "key", key: e.key, shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey, meta: e.metaKey });
      if (["z", "y", "s", "d", "delete", "backspace"].includes(e.key.toLowerCase())) e.preventDefault();
    }
  }, true);

  window.addEventListener("scroll", redraw, true);
  // Position de défilement transmise à l'interface (rétablie après un rechargement).
  let scrollTimer = 0;
  window.addEventListener("scroll", () => {
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => post({ type: "scroll", x: window.scrollX, y: window.scrollY }), 120);
  }, { passive: true });
  window.addEventListener("resize", () => { redraw(); sendComputed(); });
  new ResizeObserver(redraw).observe(document.documentElement);

  // ---------------------------------------------------------------- messages de l'interface
  const byId = (id) => (id ? document.querySelector('[data-rv="' + CSS.escape(id) + '"]') : null);

  window.addEventListener("message", (e) => {
    const m = e.data;
    if (!m || m.revel !== 1 || e.source !== window.parent) return;
    switch (m.type) {
      case "select": {
        const el = byId(m.id);
        select(el, false);
        if (el && m.reveal) el.scrollIntoView({ block: "nearest", inline: "nearest" });
        break;
      }
      case "hover":
        hovered = byId(m.id);
        redraw();
        break;
      case "watch":
        watched = Array.isArray(m.props) ? m.props : [];
        sendComputed();
        break;
      case "stylesheet": {
        // Feuille de style régénérée : rechargée sans recharger la page.
        for (const l of document.querySelectorAll('link[rel~="stylesheet"]')) {
          const u = new URL(l.href, location.href);
          if (u.pathname === m.url) {
            u.searchParams.set("rv", String(Date.now()));
            const next = l.cloneNode();
            next.href = u.pathname + u.search;
            next.addEventListener("load", () => { l.remove(); redraw(); sendComputed(); }, { once: true });
            l.after(next);
          }
        }
        break;
      }
      case "scroll":
        window.scrollTo(m.x || 0, m.y || 0);
        break;
      default:
        break;
    }
  });

  post({
    type: "ready",
    path: location.pathname,
    title: document.title,
    slots: slots.length,
    scroll: { x: window.scrollX, y: window.scrollY },
  });
})();
