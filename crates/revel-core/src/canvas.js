// Revel — script du canevas de l'éditeur.
//
// Injecté par Revel dans chaque page affichée dans l'éditeur (jamais dans le site publié).
// Les éléments portent `data-rv="<identifiant du nœud>"` ; les textes, expressions et composants
// sont entourés de commentaires `<!--rv:N-->` … `<!--/rv:N-->` décrits par la liste `#rv-slots` ;
// `#rv-vars` donne les valeurs des affectations `{% set %}`.
// Échanges avec l'interface de Revel (fenêtre parente) par `postMessage`, champ `revel: 1`.
(() => {
  "use strict";
  if (window.__revelCanvas) return;
  window.__revelCanvas = true;

  const slotsEl = document.getElementById("rv-slots");
  let slots = [];
  try { slots = JSON.parse(slotsEl ? slotsEl.textContent : "[]"); } catch (_) { slots = []; }
  // Valeurs des affectations `{% set %}` au rendu (panneau Variables).
  const varsEl = document.getElementById("rv-vars");
  let vars = [];
  try { vars = JSON.parse(varsEl ? varsEl.textContent : "[]"); } catch (_) { vars = []; }

  // La page doit rester identique au site généré : les éléments ajoutés par Revel sont retirés
  // (sinon `:last-child` changerait), et les commentaires repères sont remplacés par des plages
  // (Range) vivantes, puis les textes adjacents fusionnés comme dans le site (sinon la mise en
  // forme du texte pourrait varier d'une fraction de pixel).
  if (slotsEl) slotsEl.remove();
  if (varsEl) varsEl.remove();
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
    ".tag,.ghost,.into{position:fixed;pointer-events:none;display:none;z-index:2147483647;" +
    "font:600 11px/18px system-ui,-apple-system,'Segoe UI',sans-serif;background:#6d8cff;color:#fff;" +
    "padding:0 6px;border-radius:3px;white-space:nowrap;max-width:60vw;overflow:hidden;text-overflow:ellipsis}" +
    // Glisser-déposer : conteneur d'arrivée (pointillés), ligne d'insertion, étiquette qui suit le pointeur.
    ".drop{outline:1px dashed #6d8cff;outline-offset:-1px;background:rgba(109,140,255,.06)}" +
    ".line{background:#3d7eff;border-radius:2px;box-shadow:0 0 0 1px #fff}" +
    ".into{background:#1f1f1f;color:#fff}" +
    ".ghost{background:#3d7eff;box-shadow:0 4px 12px rgba(0,0,0,.25)}" +
    // Utilisation de composant (vert, comme Webflow) ; modification isolée : le reste estompé.
    ".hover.comp{outline-color:#2fb67c}.sel.comp{outline-color:#2fb67c}.tag.comp{background:#2fb67c}" +
    ".dim{position:fixed;inset:0;pointer-events:none;display:none;z-index:2147483645;background:rgba(20,20,24,.5)}" +
    "</style><div class='dim'></div><div class='box hover'></div><div class='box sel'></div><div class='box drop'></div>" +
    "<div class='box line'></div><div class='tag'></div><div class='into'></div><div class='ghost'></div>";
  const [dimBox, hoverBox, selBox, dropBox, lineBox, tagBox, intoBox, ghostBox] =
    root.querySelectorAll(".dim, .hover, .sel, .drop, .line, .tag, .into, .ghost");
  document.documentElement.appendChild(host);

  let hovered = null;
  let hoveredInst = null;
  let selected = null;
  let selectedInst = null;

  // ---------------------------------------------------------------- composants
  // Utilisations de composants (repères de bloc avec `component`) : un clic dans une utilisation
  // la sélectionne en entière ; un double-clic ouvre le composant (modification isolée, le reste
  // de la page estompé), ou modifie sur place le texte d'une de ses propriétés.
  const instances = [];
  for (let n = 0; n < slots.length; n++) {
    const sl = slots[n];
    if (sl && sl.slot === "block" && sl.component && ranges[n]) instances.push({ n, node: sl.node, component: sl.component });
  }
  let isolated = null;
  let names = {};
  const rangeOfInst = (i) => ranges[i.n];
  const depth = (node) => { let d = 0; for (let x = node; x; x = x.parentNode) d++; return d; };
  for (const i of instances) i.depth = depth(rangeOfInst(i).startContainer);
  // `el` est dans l'utilisation `i` (pas un de ses ancêtres).
  const within = (i, el) => {
    const r = rangeOfInst(i);
    try { return r.isPointInRange(el, 0) && !el.contains(r.startContainer); } catch (_) { return false; }
  };
  // Premier élément repéré d'une utilisation (sa racine, qui la représente).
  const rootOf = (i) => {
    const r = rangeOfInst(i);
    const walker = document.createTreeWalker(r.commonAncestorContainer, NodeFilter.SHOW_ELEMENT);
    for (let el = walker.nextNode(); el; el = walker.nextNode()) {
      if (el.hasAttribute("data-rv") && within(i, el)) return el;
    }
    return null;
  };
  const roots = new Map();
  for (const i of instances) { i.root = rootOf(i); if (i.root && !roots.has(i.root)) roots.set(i.root, i); }
  const instName = (i) => names[i.component] || i.component.split("/").pop();
  const isOpen = (i) => !!isolated && i.component === isolated;
  // Utilisations qui contiennent `el`, de la plus externe à la plus interne.
  const around = (el) => instances.filter((i) => within(i, el)).sort((a, b) => a.depth - b.depth);
  // Élément sélectionnable sous le pointeur, ou utilisation de composant (fermée) qui le contient.
  // Composant ouvert : ses utilisations sont ouvertes ; celles qu'il contient restent fermées.
  const pick = (node) => {
    const t = targetOf(node);
    if (!t) return null;
    let list = around(t);
    if (isolated) {
      const k = list.map((i) => i.component).lastIndexOf(isolated);
      list = k < 0 ? list : list.slice(k + 1);
    }
    const inst = list[0];
    return inst && inst.root ? { el: inst.root, inst } : { el: t, inst: null };
  };
  // Dans le composant ouvert (une de ses utilisations).
  const inIsolated = (el) => !isolated || around(el).some((i) => i.component === isolated);
  const idOf = (el, inst) => (inst ? inst.node : el.getAttribute("data-rv"));

  const label = (el, inst) => {
    if (inst) return "\u25C6 " + instName(inst);
    const cls = typeof el.className === "string" && el.className.trim()
      ? "." + el.className.trim().split(/\s+/).slice(0, 2).join(".")
      : "";
    return el.tagName.toLowerCase() + cls;
  };

  const boxOf = (el, inst) => (inst ? rangeOfInst(inst).getBoundingClientRect() : el.getBoundingClientRect());
  const place = (box, el, inst) => {
    if (!el || !el.isConnected) { box.style.display = "none"; return null; }
    const r = boxOf(el, inst);
    box.style.display = "block";
    box.style.left = r.left + "px";
    box.style.top = r.top + "px";
    box.style.width = r.width + "px";
    box.style.height = r.height + "px";
    box.classList.toggle("comp", !!inst);
    return r;
  };

  // Modification isolée : voile sur la page, percé aux utilisations du composant ouvert.
  const drawDim = () => {
    if (!isolated) { dimBox.style.display = "none"; return; }
    const W = innerWidth;
    const H = innerHeight;
    let d = "M0 0H" + W + "V" + H + "H0Z";
    for (const i of instances) {
      if (i.component !== isolated) continue;
      for (const b of rangeOfInst(i).getClientRects()) {
        if (b.width && b.height) d += "M" + b.left + " " + b.top + "H" + b.right + "V" + b.bottom + "H" + b.left + "Z";
      }
    }
    dimBox.style.clipPath = "path(evenodd, '" + d + "')";
    dimBox.style.display = "block";
  };

  const draw = () => {
    drawDim();
    const same = hovered === selected && hoveredInst === selectedInst;
    place(hoverBox, hovered && !same ? hovered : null, hoveredInst);
    const r = place(selBox, selected, selectedInst);
    if (r && selected) {
      tagBox.textContent = label(selected, selectedInst);
      tagBox.classList.toggle("comp", !!selectedInst);
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
      // Affichage du parent (réglages d'un élément dans une pile) et fond effectif sous l'élément
      // (contraste du texte) : premier fond non transparent en remontant, blanc à défaut.
      const parent = selected.parentElement;
      values["--rv-parent-display"] = parent ? getComputedStyle(parent).display : "";
      let bg = "";
      for (let el = selected; el && !bg; el = el.parentElement) {
        const c = getComputedStyle(el).backgroundColor;
        if (c && c !== "transparent" && !/rgba\([^)]*,\s*0\)$/.test(c)) bg = c;
      }
      values["--rv-background"] = bg || "rgb(255, 255, 255)";
      post({ type: "computed", id: selected.getAttribute("data-rv"), values });
    }, 30);
  };

  const select = (el, notify, inst) => {
    selected = el;
    selectedInst = el ? inst || null : null;
    sendComputed();
    redraw();
    if (notify) {
      post(el
        ? { type: "select", id: idOf(el, selectedInst), tag: selectedInst ? "" : el.tagName.toLowerCase(), label: label(el, selectedInst),
            rect: selectedInst ? rectOf({ getBoundingClientRect: () => boxOf(el, selectedInst) }) : rectOf(el),
            component: selectedInst ? selectedInst.component : undefined }
        : { type: "select", id: null });
    }
  };

  // ---------------------------------------------------------------- événements de la page
  document.addEventListener("mousemove", (e) => {
    const p = pick(e.target);
    const t = p ? p.el : null;
    const inst = p ? p.inst : null;
    if (t !== hovered || inst !== hoveredInst) {
      hovered = t;
      hoveredInst = inst;
      redraw();
      post({ type: "hover", id: t ? idOf(t, inst) : null });
    }
  }, true);

  document.addEventListener("mouseleave", () => {
    hovered = null;
    hoveredInst = null;
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

  // ---------------------------------------------------------------- glisser-déposer
  // Emplacement de dépôt sous le pointeur, selon le modèle de Webflow : sur le bord d'un élément
  // (dans le sens où ses frères se suivent), avant ou après lui ; au milieu d'un élément qui peut
  // contenir des enfants, dedans, à côté de l'enfant le plus proche. Les règles du HTML sont
  // respectées (pas de bloc dans un paragraphe, pas de lien dans un lien, `li` dans une liste).
  const NO_KIDS = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
    "track", "wbr", "svg", "picture", "video", "audio", "iframe", "canvas", "select", "textarea", "object"]);
  const PHRASING = new Set(["p", "h1", "h2", "h3", "h4", "h5", "h6", "span", "a", "button", "label", "strong",
    "em", "b", "i", "u", "small", "code", "figcaption", "summary", "legend", "dt", "q", "cite", "abbr", "time",
    "mark", "sub", "sup"]);
  const BLOCK = new Set(["section", "div", "header", "footer", "main", "nav", "article", "aside", "h1", "h2", "h3",
    "h4", "h5", "h6", "p", "ul", "ol", "li", "figure", "form", "blockquote", "table", "hr", "details", "dl"]);
  const rvOf = (el) => (el && el.getAttribute ? el.getAttribute("data-rv") : null);
  // Repère d'un élément pour un dépôt : l'utilisation pour la racine d'une utilisation fermée.
  const refOf = (el) => { const i = roots.get(el); return i && !isOpen(i) ? i.node : rvOf(el); };
  const parentRv = (el) => (el && el.parentElement ? el.parentElement.closest("[data-rv]") : null);
  const inDrag = (el, d) => !!d.el && (el === d.el || d.el.contains(el));
  const accepts = (c, d) => {
    if (!c || !rvOf(c) || inDrag(c, d)) return false;
    const t = c.tagName.toLowerCase();
    if (NO_KIDS.has(t)) return false;
    if (t === "ul" || t === "ol") return d.tag === "li";
    if (d.tag === "li") return false;
    if ((d.tag === "a" || d.tag === "button") && c.closest("a, button")) return false;
    if (PHRASING.has(t)) return !d.block;
    return true;
  };
  const visible = (el) => el.getClientRects().length > 0;
  // Éléments repérés dont `c` est le plus proche ancêtre repéré (sans l'élément déplacé).
  const kidsOf = (c, d) => {
    const out = [];
    const walk = (n) => {
      for (const k of n.children) {
        if (rvOf(k)) { if (!inDrag(k, d) && visible(k)) out.push(k); } else walk(k);
      }
    };
    walk(c);
    return out;
  };
  const dist = (r, x, y) => Math.hypot(Math.max(r.left - x, 0, x - r.right), Math.max(r.top - y, 0, y - r.bottom));
  // Sens dans lequel se suivent les frères : horizontal si un autre frère (élément déplacé compris)
  // est sur la même ligne ; seul sur sa ligne, d'après la mise en page du parent (pile ou grille
  // en ligne) ou de l'élément (en ligne).
  const axisAmong = (kids, k) => {
    const r = k.getBoundingClientRect();
    const sameLine = kids.some((o) => {
      if (o === k) return false;
      const q = o.getBoundingClientRect();
      return q.top < r.bottom - 2 && q.bottom > r.top + 2;
    });
    if (sameLine) return "x";
    if (kids.length > 1) return "y";
    const p = k.parentElement ? getComputedStyle(k.parentElement) : null;
    if (p && /flex/.test(p.display) && p.flexDirection.startsWith("row")) return "x";
    if (p && /grid/.test(p.display) && p.gridTemplateColumns.trim().split(/\s+/).length > 1) return "x";
    return /^inline/.test(getComputedStyle(k).display) ? "x" : "y";
  };
  const NONE = {};
  const findDrop = (x, y, d) => {
    let hit = null;
    for (const e of document.elementsFromPoint(x, y)) {
      if (e === host || inDrag(e, d)) continue;
      hit = e;
      break;
    }
    // Dans une utilisation fermée : elle compte comme un seul élément (sa racine).
    const hp = hit ? pick(hit) : null;
    let t = hp ? hp.el : null;
    while (t && inDrag(t, d)) t = parentRv(t);
    for (; t; t = parentRv(t)) {
      // Composant ouvert : dépôt seulement à l'intérieur.
      if (!inIsolated(t)) return null;
      const r = t.getBoundingClientRect();
      const parent = parentRv(t);
      const ci = roots.get(t);
      const closed = !!ci && !isOpen(ci) && inIsolated(parent || t);
      const openRoot = !!ci && isOpen(ci);
      const canIn = !closed && accepts(t, d);
      const canSib = !!parent && !openRoot && accepts(parent, d);
      let axis = "y";
      if (canSib) axis = axisAmong(kidsOf(parent, NONE), t);
      const pos = axis === "x" ? x - r.left : y - r.top;
      const size = axis === "x" ? r.width : r.height;
      const band = canIn ? Math.min(12, size * 0.25) : size / 2;
      if (canSib && (!canIn || pos < band || pos > size - band)) {
        return { ref: refOf(t), where: pos < size / 2 ? "before" : "after", axis, container: parent, el: t };
      }
      if (canIn) {
        const kids = kidsOf(t, d);
        if (!kids.length) return { ref: rvOf(t), where: "inside", axis: "y", container: t, el: t };
        let k = kids[0];
        let best = Infinity;
        for (const o of kids) {
          const dd = dist(o.getBoundingClientRect(), x, y);
          if (dd < best) { best = dd; k = o; }
        }
        const ax = axisAmong(kidsOf(t, NONE), k);
        const kr = k.getBoundingClientRect();
        const before = ax === "x" ? x < kr.left + kr.width / 2 : y < kr.top + kr.height / 2;
        return { ref: refOf(k), where: before ? "before" : "after", axis: ax, container: t, el: k };
      }
    }
    return null;
  };
  const hide = (...boxes) => { for (const b of boxes) b.style.display = "none"; };
  const showDrop = (t) => {
    if (!t) { hide(dropBox, lineBox, intoBox); return; }
    const c = t.container.getBoundingClientRect();
    place(dropBox, t.container);
    const r = t.el.getBoundingClientRect();
    const L = lineBox.style;
    lineBox.style.display = "block";
    if (t.where === "inside") {
      L.left = (c.left + 4) + "px"; L.width = Math.max(0, c.width - 8) + "px";
      L.top = (c.bottom - 6) + "px"; L.height = "2px";
    } else if (t.axis === "x") {
      L.left = ((t.where === "before" ? r.left : r.right) - 1) + "px"; L.width = "2px";
      L.top = r.top + "px"; L.height = r.height + "px";
    } else {
      L.left = r.left + "px"; L.width = r.width + "px";
      L.top = ((t.where === "before" ? r.top : r.bottom) - 1) + "px"; L.height = "2px";
    }
    intoBox.textContent = label(t.container);
    intoBox.style.display = "block";
    intoBox.style.left = Math.max(0, c.left) + "px";
    intoBox.style.top = (c.top >= 20 ? c.top - 20 : c.top + 2) + "px";
  };

  // Séance de glisser-déposer : élément du panneau « Ajouter » (piloté par l'interface) ou
  // élément de la page déplacé à la souris (Alt : copie).
  let drag = null;
  let lastDrop = null;
  let scrollTick = 0;
  const dropMsg = (t) => (t ? { ref: t.ref, where: t.where } : null);
  const probe = (x, y) => {
    if (!drag) return;
    drag.x = x; drag.y = y; drag.out = false;
    const t = findDrop(x, y, drag);
    showDrop(t);
    const m = dropMsg(t);
    if (JSON.stringify(m) !== JSON.stringify(lastDrop)) {
      lastDrop = m;
      post({ type: "drop", target: m });
    }
  };
  // Défilement près des bords haut et bas pendant le glisser.
  const edgeScroll = () => {
    if (!drag) { scrollTick = 0; return; }
    if (drag.out || drag.y === undefined) { scrollTick = requestAnimationFrame(edgeScroll); return; }
    const m = 28;
    const v = drag.y < m ? -(m - drag.y) / 2 : drag.y > innerHeight - m ? (drag.y - (innerHeight - m)) / 2 : 0;
    if (v) { window.scrollBy(0, v); probe(drag.x, drag.y); }
    scrollTick = requestAnimationFrame(edgeScroll);
  };
  const startDrag = (d) => {
    drag = d;
    lastDrop = undefined;
    hoverBox.style.display = "none";
    if (!scrollTick) scrollTick = requestAnimationFrame(edgeScroll);
  };
  const endDrag = () => {
    if (drag && drag.el) {
      if (drag.opacity) drag.el.style.setProperty("opacity", drag.opacity, drag.prio);
      else drag.el.style.removeProperty("opacity");
      if (!drag.el.getAttribute("style")) drag.el.removeAttribute("style");
    }
    drag = null;
    lastDrop = null;
    hide(dropBox, lineBox, intoBox, ghostBox);
    redraw();
  };
  const blockTag = (t) => BLOCK.has(t);

  // Déplacement d'un élément de la page : appui puis déplacement de plus de 4 px.
  let press = null;
  let dragged = false;
  document.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || editing || inEdit(e.target)) return;
    const p = pick(e.target);
    if (p) press = { el: p.el, inst: p.inst, x: e.clientX, y: e.clientY, id: e.pointerId };
  }, true);
  document.addEventListener("pointermove", (e) => {
    if (press && !drag && Math.hypot(e.clientX - press.x, e.clientY - press.y) > 5) {
      const el = press.el;
      const inst = press.inst;
      try { document.documentElement.setPointerCapture(press.id); } catch (_) { /* pointeur relâché */ }
      select(el, true, inst);
      const tag = el.tagName.toLowerCase();
      startDrag({ el, inst, id: idOf(el, inst), tag, block: inst ? true : blockTag(tag), move: true,
        opacity: el.style.getPropertyValue("opacity"), prio: el.style.getPropertyPriority("opacity") });
      // Élément déplacé estompé (prioritaire sur les animations du site).
      el.style.setProperty("opacity", "0.35", "important");
      ghostBox.textContent = label(el, inst);
    }
    if (drag && drag.move) {
      ghostBox.textContent = (e.altKey ? "+ " : "") + label(drag.el, drag.inst);
      ghostBox.style.display = "block";
      ghostBox.style.left = (e.clientX + 12) + "px";
      ghostBox.style.top = (e.clientY + 14) + "px";
      probe(e.clientX, e.clientY);
    }
  }, true);
  const release = (e, cancel) => {
    if (drag && drag.move) {
      const t = cancel ? null : findDrop(e.clientX, e.clientY, drag);
      const id = drag.id || rvOf(drag.el);
      endDrag();
      dragged = true;
      setTimeout(() => { dragged = false; }, 0);
      if (t) post({ type: "moved", id, target: dropMsg(t), copy: !!e.altKey });
      else post({ type: "drop", target: null });
    }
    press = null;
  };
  document.addEventListener("pointerup", (e) => release(e, false), true);
  document.addEventListener("pointercancel", (e) => release(e, true), true);

  // Les clics sélectionnent : liens, boutons et formulaires du site ne réagissent pas.
  const block = (e) => { e.preventDefault(); e.stopPropagation(); };
  document.addEventListener("click", (e) => {
    if (inEdit(e.target)) return;
    if (editing) finishEdit(true);
    block(e);
    if (dragged) return;
    const p = pick(e.target);
    select(p ? p.el : null, true, p ? p.inst : null);
  }, true);
  // Clic droit : élément sélectionné, menu de l'éditeur (pas celui du navigateur).
  document.addEventListener("contextmenu", (e) => {
    if (inEdit(e.target)) return;
    block(e);
    const p = pick(e.target);
    if (p && (p.el !== selected || p.inst !== selectedInst)) select(p.el, true, p.inst);
    post({ type: "context", x: e.clientX, y: e.clientY });
  }, true);
  const propOf = (sl, inst) => !!sl && sl.slot === "expr" && !!sl.binding && !!sl.binding.source &&
    sl.binding.source.kind === "prop" && sl.binding.source.node === inst.node;
  document.addEventListener("dblclick", (e) => {
    if (inEdit(e.target)) return;
    block(e);
    const n = slotAt(e.clientX, e.clientY);
    const p = pick(e.target);
    if (p && p.inst) {
      // Texte d'une propriété de cette utilisation : modifié sur place (valeur de l'utilisation).
      if (n !== null && propOf(slots[n], p.inst)) { startEdit(n); return; }
      // Sinon : le composant s'ouvre, l'élément sous le pointeur est sélectionné.
      const inner = targetOf(e.target);
      if (inner) {
        isolated = p.inst.component;
        select(inner, true, null);
      }
      return;
    }
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
    if (e.key === "Escape") {
      if (drag && drag.move) { release(e, true); return; }
      select(null, true);
      return;
    }
    // Espace maintenue : déplacement de la vue (comme Figma), géré par l'interface.
    if (e.key === " " || e.code === "Space") {
      e.preventDefault();
      if (!e.repeat) post({ type: "key", key: " ", code: "Space", shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey, meta: e.metaKey });
      return;
    }
    // Raccourcis de l'éditeur transmis à l'interface : avec Ctrl/Cmd, suppression, navigation dans
    // la sélection (Entrée, Maj+Entrée, Tab), zoom (Maj+0/1/2), ordre (Alt+flèches).
    const nav = e.key === "Enter" || e.key === "Tab";
    const zoom = e.shiftKey && /^Digit[012]$/.test(e.code);
    const order = e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown");
    if (e.ctrlKey || e.metaKey || e.key === "Delete" || e.key === "Backspace" || nav || zoom || order) {
      post({ type: "key", key: e.key, code: e.code, shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey, meta: e.metaKey });
      // Raccourcis du navigateur neutralisés (zoom de la page, enregistrer…), sauf copier/coller.
      if (!((e.ctrlKey || e.metaKey) && ["c", "x", "v", "a"].includes(e.key.toLowerCase()) && !e.shiftKey)) e.preventDefault();
    }
  }, true);
  document.addEventListener("keyup", (e) => {
    if (e.key === " " || e.code === "Space") post({ type: "keyup", key: " ", code: "Space" });
  }, true);

  // Molette avec Alt (ou Ctrl, pincement du pavé tactile) : zoom de l'éditeur autour du pointeur.
  window.addEventListener("wheel", (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) { e.preventDefault(); post({ type: "zoom", dy: e.deltaY || e.deltaX, x: e.clientX, y: e.clientY }); }
  }, { passive: false });
  // Bouton du milieu : déplacement de la vue (écarts en pixels de l'écran).
  let panFrom = null;
  window.addEventListener("pointerdown", (e) => {
    if (e.button !== 1) return;
    e.preventDefault();
    panFrom = { x: e.screenX, y: e.screenY };
    try { document.documentElement.setPointerCapture(e.pointerId); } catch (_) { /* relâché */ }
  }, true);
  window.addEventListener("pointermove", (e) => {
    if (!panFrom) return;
    post({ type: "pan", dx: e.screenX - panFrom.x, dy: e.screenY - panFrom.y });
    panFrom = { x: e.screenX, y: e.screenY };
  }, true);
  window.addEventListener("pointerup", (e) => { if (e.button === 1) panFrom = null; }, true);

  window.addEventListener("scroll", redraw, true);
  // Défilement suivi une fois par image (règles de l'interface).
  let scrollFrame = 0;
  window.addEventListener("scroll", () => {
    if (scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => { scrollFrame = 0; post({ type: "scrolled", x: window.scrollX, y: window.scrollY }); });
  }, { passive: true });
  // Position de défilement transmise à l'interface (rétablie après un rechargement).
  let scrollTimer = 0;
  window.addEventListener("scroll", () => {
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => post({ type: "scroll", x: window.scrollX, y: window.scrollY }), 120);
  }, { passive: true });
  // Taille du cadre changée (point de rupture, animée) : valeurs calculées envoyées une fois la
  // taille stable.
  let resizeTimer = 0;
  window.addEventListener("resize", () => { redraw(); clearTimeout(resizeTimer); resizeTimer = setTimeout(sendComputed, 120); });
  new ResizeObserver(redraw).observe(document.documentElement);

  // ---------------------------------------------------------------- aperçu des styles
  let previewSheet = null;
  let previewEl = null;
  const setPreview = (css) => {
    if ("adoptedStyleSheets" in document && typeof CSSStyleSheet === "function" && "replaceSync" in CSSStyleSheet.prototype) {
      if (!previewSheet) {
        previewSheet = new CSSStyleSheet();
        document.adoptedStyleSheets = [...document.adoptedStyleSheets, previewSheet];
      }
      try { previewSheet.replaceSync(css); } catch (_) { /* valeur en cours de saisie */ }
    } else if (css) {
      // Moteurs sans feuilles construites : élément `<style>` présent seulement pendant le geste.
      if (!previewEl) previewEl = document.createElement("style");
      previewEl.textContent = css;
      if (!previewEl.isConnected) document.head.appendChild(previewEl);
    } else if (previewEl) {
      previewEl.remove();
    }
    redraw();
    sendComputed();
  };

  // ---------------------------------------------------------------- messages de l'interface
  const byId = (id) => (id ? document.querySelector('[data-rv="' + CSS.escape(id) + '"]') : null);
  // Élément (ou utilisation de composant) désigné par l'interface.
  const findId = (id) => {
    if (!id) return { el: null, inst: null };
    const inst = instances.find((i) => i.node === id && i.root);
    if (inst) return { el: inst.root, inst };
    return { el: byId(id), inst: null };
  };

  window.addEventListener("message", (e) => {
    const m = e.data;
    if (!m || m.revel !== 1 || e.source !== window.parent) return;
    switch (m.type) {
      case "select": {
        const { el, inst } = findId(m.id);
        select(el, false, inst);
        if (el && m.reveal) el.scrollIntoView({ block: "nearest", inline: "nearest" });
        break;
      }
      case "hover": {
        const { el, inst } = findId(m.id);
        hovered = el;
        hoveredInst = inst;
        redraw();
        break;
      }
      case "isolate":
        // Composant ouvert (modification isolée) ou fermé ; noms des composants.
        isolated = m.component || null;
        if (m.names) names = m.names;
        redraw();
        break;
      case "watch":
        watched = Array.isArray(m.props) ? m.props : [];
        sendComputed();
        break;
      case "count": {
        // Nombre d'éléments de la page qui portent un sélecteur (sans les états).
        let n = 0;
        try { n = document.querySelectorAll(m.selector).length; } catch (_) { n = 0; }
        post({ type: "count", selector: m.selector, n });
        break;
      }
      case "preview":
        // Aperçu pendant un geste (glisser, couleur) : feuille ajoutée, retirée quand la feuille du
        // site est rechargée. Feuille construite (adoptedStyleSheets) : la page ne change pas.
        setPreview(m.css || "");
        break;
      case "stylesheet": {
        // Feuille de style régénérée : rechargée sans recharger la page.
        for (const l of document.querySelectorAll('link[rel~="stylesheet"]')) {
          const u = new URL(l.href, location.href);
          if (u.pathname === m.url) {
            u.searchParams.set("rv", String(Date.now()));
            const next = l.cloneNode();
            next.href = u.pathname + u.search;
            next.addEventListener("load", () => { l.remove(); setPreview(""); redraw(); sendComputed(); }, { once: true });
            l.after(next);
          }
        }
        break;
      }
      case "scroll":
        window.scrollTo(m.x || 0, m.y || 0);
        break;
      case "drag":
        // Glisser depuis l'interface (panneau « Ajouter ») : début, position, fin.
        if (m.phase === "start") startDrag({ el: null, tag: m.tag || "div", block: !!m.block, move: false });
        else if (m.phase === "move") probe(m.x, m.y);
        else if (m.phase === "out") { if (drag) { drag.out = true; showDrop(null); lastDrop = null; } }
        else endDrag();
        break;
      case "nav": {
        // Parcours de la sélection : parent, premier enfant, frère suivant ou précédent.
        if (!selected) break;
        let next = null;
        if (m.dir === "parent") next = parentRv(selected);
        else if (m.dir === "child") next = kidsOf(selected, {})[0] || null;
        else {
          const p = parentRv(selected);
          const sibs = p ? kidsOf(p, {}) : [];
          const i = sibs.indexOf(selected);
          next = i < 0 ? null : sibs[i + (m.dir === "next" ? 1 : -1)] || null;
        }
        if (next) {
          select(next, true);
          next.scrollIntoView({ block: "nearest", inline: "nearest" });
        }
        break;
      }
      case "measure": {
        // Position de la sélection (zoom sur la sélection), après l'avoir montrée.
        const el = findId(m.id).el || selected;
        if (el) el.scrollIntoView({ block: "nearest", inline: "nearest" });
        post({ type: "measure", rect: el ? rectOf(el) : null });
        break;
      }
      default:
        break;
    }
  });

  post({
    type: "ready",
    path: location.pathname,
    title: document.title,
    slots: slots.length,
    vars,
    scroll: { x: window.scrollX, y: window.scrollY },
  });
})();
