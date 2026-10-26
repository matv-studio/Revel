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
  // Révision du document rendu : l'interface demande une mise à jour sur place si elle en a une
  // plus récente (modification faite pendant le chargement).
  let rev = slotsEl ? Number(slotsEl.getAttribute("data-rev")) || 0 : 0;
  // Valeurs des affectations `{% set %}` au rendu (panneau Variables).
  const varsEl = document.getElementById("rv-vars");
  let vars = [];
  try { vars = JSON.parse(varsEl ? varsEl.textContent : "[]"); } catch (_) { vars = []; }

  // Calques masqués : cachés dès qu'ils le sont (attribut posé sur place, sans recharger) ; la
  // règle, posée ici, ne compte pas dans la signature de l'en-tête.
  if (!document.querySelector("style[data-rv-hidden]")) {
    const hid = document.createElement("style");
    hid.setAttribute("data-rv-hidden", "");
    hid.textContent = "[data-revel-hidden]{display:none!important}";
    document.head.appendChild(hid);
  }

  // La page doit rester identique au site généré : les éléments ajoutés par Revel sont retirés
  // (sinon `:last-child` changerait), et les commentaires repères sont remplacés par des plages
  // (Range) vivantes, puis les textes adjacents fusionnés comme dans le site (sinon la mise en
  // forme du texte pourrait varier d'une fraction de pixel).
  // En-tête de la page au chargement (sans titre ni métadonnées ; feuilles de style par leur
  // chemin) : comparé à celui d'un nouveau rendu pour savoir si la page peut être mise à jour sur
  // place (voir `refreshPage`).
  const headSig = (h) => [...h.children]
    .filter((e) => !/^(TITLE|META)$/.test(e.tagName) && !(e.tagName === "LINK" && /\b(canonical|alternate)\b/.test(e.getAttribute("rel") || "")))
    .filter((e) => !(e.tagName === "STYLE" && e.hasAttribute("data-rv-hidden")))
    .map((e) => (e.tagName === "LINK" && /stylesheet/.test(e.getAttribute("rel") || "")
      ? "css:" + new URL(e.getAttribute("href") || "", location.href).pathname
      : e.outerHTML))
    .join("\n");
  const headAtLoad = headSig(document.head);
  if (slotsEl) slotsEl.remove();
  if (varsEl) varsEl.remove();
  if (document.currentScript) document.currentScript.remove();
  const ranges = [];
  // Repères remplacés par des plages (au chargement, puis après chaque mise à jour sur place).
  const scanMarkers = () => {
    ranges.length = 0;
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
  };
  scanMarkers();
  /** Plage du repère `n` dans la page (texte, expression, composant). */
  const slotRange = (n) => ranges[n] || null;
  window.__revelSlot = (n) => ({ slot: slots[n], range: slotRange(n) });

  const post = (msg) => window.parent.postMessage(Object.assign({ revel: 1 }, msg), "*");
  // Canevas des composants : page d'un composant seul, variante de geste éventuelle.
  const STAGE = location.pathname === "/__revel/component/";
  // Canevas d'un composant : emplacement du contenu (D-118) montré en pointillé.
  if (STAGE) {
    const st = document.createElement("style");
    st.textContent = "[data-revel-slot]{outline:1px dashed rgba(124,92,255,.75);outline-offset:-1px;min-height:24px}";
    document.head.appendChild(st);
  }
  const stageState = STAGE ? new URLSearchParams(location.search).get("rvstate") : null;

  // ---------------------------------------------------------------- calque de sélection
  // Dans un shadow DOM fermé : les styles du site ne le touchent pas, et inversement.
  const host = document.createElement("revel-overlay");
  const root = host.attachShadow({ mode: "closed" });
  root.innerHTML =
    "<style>" +
    // `--k` : inverse du zoom de l'éditeur (traits, poignées et étiquettes gardent leur taille à
    // l'écran).
    ":host{all:initial;--k:1}" +
    ".box{position:fixed;box-sizing:border-box;pointer-events:none;display:none;z-index:2147483646}" +
    ".hover{outline:calc(1px*var(--k)) solid #6d8cff}" +
    ".sel{outline:calc(2px*var(--k)) solid #6d8cff}" +
    // Autres éléments de la sélection multiple.
    ".sel2{outline:calc(1.5px*var(--k)) solid #6d8cff;background:rgba(109,140,255,.08)}" +
    // Espacements de l'élément sélectionné sous le pointeur (comme Framer) : intérieurs et écarts
    // entre enfants en teinte unie, marges plus claires ; une poignée au milieu de chaque côté,
    // réglable en glissant.
    ".side{position:fixed;display:none;pointer-events:none;z-index:2147483645;box-sizing:border-box}" +
    ".side.p,.side.g{background:rgba(109,140,255,.16)}" +
    ".side.m{background:rgba(109,140,255,.07)}" +
    ".spacing .side.p,.spacing .side.g{background:rgba(109,140,255,.22)}" +
    ".side.hot{background:rgba(109,140,255,.3)}" +
    ".ph{position:fixed;display:none;pointer-events:none;z-index:2147483647;box-sizing:border-box;background:#fff;" +
    "border:calc(1.5px*var(--k)) solid #6d8cff;border-radius:calc(3px*var(--k));box-shadow:0 1px 2px rgba(0,0,0,.18)}" +
    ".ph.hot{background:#6d8cff}" +
    ".sval{position:fixed;display:none;pointer-events:none;z-index:2147483647;font:600 calc(11px*var(--k))/calc(18px*var(--k)) system-ui,-apple-system,'Segoe UI',sans-serif;" +
    "background:#6d8cff;color:#fff;padding:0 calc(5px*var(--k));border-radius:calc(3px*var(--k));font-variant-numeric:tabular-nums}" +
    ".tag,.ghost,.into,.size{position:fixed;pointer-events:none;display:none;z-index:2147483647;" +
    "font:600 calc(11px*var(--k))/calc(18px*var(--k)) system-ui,-apple-system,'Segoe UI',sans-serif;background:#6d8cff;color:#fff;" +
    "padding:0 calc(6px*var(--k));border-radius:calc(3px*var(--k));white-space:nowrap;max-width:60vw;overflow:hidden;text-overflow:ellipsis}" +
    ".size{background:#1f1f1f;font-variant-numeric:tabular-nums}" +
    // Poignées de taille de l'élément sélectionné (bord droit, bord bas, coin).
    ".h{position:fixed;display:none;z-index:2147483647;box-sizing:border-box;width:calc(9px*var(--k));height:calc(9px*var(--k));" +
    "background:#fff;border:calc(1.5px*var(--k)) solid #3d7eff;border-radius:calc(2px*var(--k));pointer-events:auto}" +
    ".h-e{cursor:ew-resize}.h-s{cursor:ns-resize}.h-se{cursor:nwse-resize}" +
    // Glisser-déposer : conteneur d'arrivée (pointillés), ligne d'insertion, étiquette qui suit le pointeur.
    ".drop{outline:1px dashed #6d8cff;outline-offset:-1px;background:rgba(109,140,255,.06)}" +
    ".line{background:#3d7eff;border-radius:2px;box-shadow:0 0 0 1px #fff}" +
    ".into{background:#1f1f1f;color:#fff}" +
    ".ghost{background:#3d7eff;box-shadow:0 4px 12px rgba(0,0,0,.25)}" +
    // Utilisation de composant (couleur des composants, réglable : D-125) ; modification isolée : le reste estompé.
    ".hover.comp{outline-color:var(--rv-comp,#a684ff)}.sel.comp{outline-color:var(--rv-comp,#a684ff)}.tag.comp{background:var(--rv-comp,#a684ff)}" +
    ".dim{position:fixed;inset:0;pointer-events:none;display:none;z-index:2147483645;background:rgba(20,20,24,.5)}" +
    // Barre de mise en forme du texte en cours de modification.
    ".fbar{position:fixed;display:none;align-items:center;gap:calc(2px*var(--k));z-index:2147483647;padding:calc(3px*var(--k));" +
    "background:#1f1f1f;border-radius:calc(6px*var(--k));box-shadow:0 4px 14px rgba(0,0,0,.3);font:600 calc(12px*var(--k))/1 system-ui,-apple-system,'Segoe UI',sans-serif}" +
    ".fbar button{all:unset;cursor:pointer;color:#fff;min-width:calc(24px*var(--k));height:calc(24px*var(--k));display:grid;place-items:center;border-radius:calc(4px*var(--k))}" +
    ".fbar button:hover{background:#3a3a3a}.fbar button.on{background:#6d8cff}.fbar svg{width:calc(16px*var(--k));height:calc(16px*var(--k))}" +
    ".fbar .fb-url{display:none;width:calc(200px*var(--k));height:calc(24px*var(--k));border:0;border-radius:calc(4px*var(--k));padding:0 calc(6px*var(--k));font:inherit;font-weight:400}" +
    ".fbar.is-link button{display:none}.fbar.is-link .fb-url{display:block}" +
    "</style><div class='dim'></div><div class='spc'></div><div class='sval'></div>" +
    "<div class='box hover'></div><div class='extra'></div><div class='box sel'></div><div class='box drop'></div>" +
    "<div class='box line'></div><div class='tag'></div><div class='into'></div><div class='ghost'></div><div class='size'></div>" +
    "<div class='h h-e' data-axis='e'></div><div class='h h-s' data-axis='s'></div><div class='h h-se' data-axis='se'></div>";
  const [dimBox, hoverBox, selBox, dropBox, lineBox, tagBox, intoBox, ghostBox] =
    root.querySelectorAll(".dim, .hover, .sel, .drop, .line, .tag, .into, .ghost");
  const spcBox = root.querySelector(".spc");
  const svalBox = root.querySelector(".sval");
  const extraBox = root.querySelector(".extra");
  const sizeBox = root.querySelector(".size");
  const handles = [...root.querySelectorAll(".h")];
  document.documentElement.appendChild(host);
  // Cadre actif (plusieurs écrans côte à côte : un seul reçoit les poignées de taille).
  let active = true;
  // Autres éléments sélectionnés (identifiants), espacements montrés pour la sélection.
  let extra = [];
  let spacingSel = false;

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
  const withinRaw = (i, el) => {
    const r = rangeOfInst(i);
    try { return r.isPointInRange(el, 0) && !el.contains(r.startContainer); } catch (_) { return false; }
  };
  // Contenu propre d'une utilisation, placé dans l'emplacement de son composant (D-118) : de la
  // page, sélectionnable à part.
  const inOwnSlot = (i, el) => {
    for (let s = el.parentElement && el.parentElement.closest("[data-rv-slot-filled]"); s;
      s = s.parentElement && s.parentElement.closest("[data-rv-slot-filled]")) {
      if (withinRaw(i, s) && !instances.some((j) => j !== i && j.depth > i.depth && withinRaw(j, s))) return true;
    }
    return false;
  };
  const within = (i, el) => withinRaw(i, el) && !inOwnSlot(i, el);
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
  // Utilisations relues (après une mise à jour sur place).
  const buildInstances = () => {
    instances.length = 0;
    for (let n = 0; n < slots.length; n++) {
      const sl = slots[n];
      if (sl && sl.slot === "block" && sl.component && ranges[n]) instances.push({ n, node: sl.node, component: sl.component });
    }
    for (const i of instances) i.depth = depth(rangeOfInst(i).startContainer);
    roots.clear();
    for (const i of instances) { i.root = rootOf(i); if (i.root && !roots.has(i.root)) roots.set(i.root, i); }
  };
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

  // Nom simple d'un élément (comme dans les calques) : classe principale rendue lisible
  // (`hero__media` -> « Hero media »), sinon sa nature dans la langue de l'interface (envoyée par
  // l'interface : message `labels`).
  let tagNames = null;
  let otherName = "";
  const label = (el, inst) => {
    if (inst) return "\u25C6 " + instName(inst);
    const cls = typeof el.className === "string" ? el.className.trim().split(/\s+/).filter(Boolean) : [];
    if (cls.length) {
      const main = cls.filter((c) => c.startsWith(cls[0])).pop() || cls[0];
      const h = main.split(/[-_]+/).filter(Boolean).join(" ");
      if (h) return h[0].toUpperCase() + h.slice(1);
    }
    const t = el.tagName.toLowerCase();
    return tagNames ? tagNames[t] || otherName || t : t;
  };

  // Cadre d'une utilisation : ses éléments de premier niveau (le contenu replié d'un `details`
  // fermé garde des boîtes que le rectangle de la plage compterait), sinon la plage.
  const instBox = (inst) => {
    const r = rangeOfInst(inst);
    if (r.startContainer === r.endContainer && r.startContainer.nodeType === 1) {
      const kids = [...r.startContainer.childNodes].slice(r.startOffset, r.endOffset).filter((k) => k.nodeType === 1);
      if (kids.length) {
        let L = Infinity, T = Infinity, R = -Infinity, B = -Infinity;
        for (const k of kids) {
          const b = k.getBoundingClientRect();
          if (!b.width && !b.height) continue;
          L = Math.min(L, b.left); T = Math.min(T, b.top); R = Math.max(R, b.right); B = Math.max(B, b.bottom);
        }
        if (L !== Infinity) return { left: L, top: T, right: R, bottom: B, width: R - L, height: B - T, x: L, y: T };
      }
    }
    return r.getBoundingClientRect();
  };
  const boxOf = (el, inst) => (inst ? instBox(inst) : el.getBoundingClientRect());
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
    // Canevas des composants : rien d'autre que le composant, rien à estomper.
    if (!isolated || STAGE) { dimBox.style.display = "none"; return; }
    const W = innerWidth;
    const H = innerHeight;
    let d = "M0 0H" + W + "V" + H + "H0Z";
    // Une découpe par utilisation (son rectangle englobant : les rectangles de chaque boîte
    // contenue, superposés, s'annuleraient deux à deux).
    for (const i of instances) {
      if (i.component !== isolated) continue;
      const b = instBox(i);
      if (b.width && b.height) d += "M" + b.left + " " + b.top + "H" + b.right + "V" + b.bottom + "H" + b.left + "Z";
    }
    dimBox.style.clipPath = "path(evenodd, '" + d + "')";
    dimBox.style.display = "block";
  };

  // Espacements d'un élément : intérieurs, marges, écarts entre enfants (`gap`).
  const px = (v) => Math.max(0, parseFloat(v) || 0);
  // Côtés : propriété CSS, sens du glissement qui augmente la valeur.
  const SIDES = {
    pt: { prop: "padding-top", axis: "y", sign: 1 }, pr: { prop: "padding-right", axis: "x", sign: -1 },
    pb: { prop: "padding-bottom", axis: "y", sign: -1 }, pl: { prop: "padding-left", axis: "x", sign: 1 },
    mt: { prop: "margin-top", axis: "y", sign: -1 }, mr: { prop: "margin-right", axis: "x", sign: 1 },
    mb: { prop: "margin-bottom", axis: "y", sign: 1 }, ml: { prop: "margin-left", axis: "x", sign: -1 },
  };
  const OPPOSITE = { pt: "pb", pb: "pt", pl: "pr", pr: "pl", mt: "mb", mb: "mt", ml: "mr", mr: "ml" };
  // Zones réglables de l'élément : clé, propriété, axe, sens, rectangle [x, y, l, h], valeur (px).
  // Marges : seulement non nulles (une marge vide n'a pas de zone à montrer).
  const spaceTargets = (el) => {
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const m = [cs.marginTop, cs.marginRight, cs.marginBottom, cs.marginLeft].map(px);
    const b = [cs.borderTopWidth, cs.borderRightWidth, cs.borderBottomWidth, cs.borderLeftWidth].map(px);
    const p = [cs.paddingTop, cs.paddingRight, cs.paddingBottom, cs.paddingLeft].map(px);
    const il = r.left + b[3], ir = r.right - b[1], it = r.top + b[0], ib = r.bottom - b[2];
    const rects = {
      pt: [il, it, ir - il, p[0], p[0]],
      pb: [il, ib - p[2], ir - il, p[2], p[2]],
      pl: [il, it + p[0], p[3], ib - it - p[0] - p[2], p[3]],
      pr: [ir - p[1], it + p[0], p[1], ib - it - p[0] - p[2], p[1]],
      mt: [r.left, r.top - m[0], r.width, m[0], m[0]],
      mb: [r.left, r.bottom, r.width, m[2], m[2]],
      ml: [r.left - m[3], r.top, m[3], r.height, m[3]],
      mr: [r.right, r.top, m[1], r.height, m[1]],
    };
    const out = [];
    for (const [key, [x, y, w, h, v]] of Object.entries(rects)) {
      if (key[0] === "m" && v <= 0) continue;
      out.push({ key, prop: SIDES[key].prop, axis: SIDES[key].axis, sign: SIDES[key].sign, rect: [x, y, w, h], v });
    }
    // Écarts entre enfants d'un conteneur flex ou grille : entre deux enfants voisins côte à côte
    // (écart horizontal) ou l'un sous l'autre (écart vertical).
    const disp = cs.display;
    if (/flex|grid/.test(disp)) {
      const kids = [...el.children].filter((c) => {
        const cc = getComputedStyle(c);
        return cc.display !== "none" && cc.position !== "absolute" && cc.position !== "fixed";
      }).map((c) => c.getBoundingClientRect()).filter((k) => k.width > 0 || k.height > 0);
      const cg = px(cs.columnGap), rg = px(cs.rowGap);
      const same = cs.columnGap === cs.rowGap;
      let n = 0;
      for (let i = 0; i + 1 < kids.length && n < 24; i++) {
        const a = kids[i], c = kids[i + 1];
        const top = Math.max(a.top, c.top), bottom = Math.min(a.bottom, c.bottom);
        const left = Math.max(a.left, c.left), right = Math.min(a.right, c.right);
        if (bottom - top > 0 && c.left >= a.right - 0.5) {
          out.push({ key: "g" + n++, prop: same ? "gap" : "column-gap", axis: "x", sign: 1, gap: true,
            rect: [a.right, Math.min(a.top, c.top), c.left - a.right, Math.max(a.bottom, c.bottom) - Math.min(a.top, c.top)], v: cg });
        } else if (right - left > 0 && c.top >= a.bottom - 0.5) {
          out.push({ key: "g" + n++, prop: same ? "gap" : "row-gap", axis: "y", sign: 1, gap: true,
            rect: [Math.min(a.left, c.left), a.bottom, Math.max(a.right, c.right) - Math.min(a.left, c.left), c.top - a.bottom], v: rg });
        }
      }
    }
    return out;
  };
  // Poignée d'une zone : au milieu, couchée pour un côté haut ou bas (pilule), debout sinon.
  // Le long du côté, au milieu de sa partie visible dans la page (élément plus haut que l'écran).
  const handleRect = (t, k) => {
    let [x, y, w, h] = t.rect;
    const seen = (a, len, max) => {
      const lo = Math.max(a, 0), hi = Math.min(a + len, max);
      return hi - lo > 24 * k ? [lo, hi - lo] : [a, len];
    };
    if (t.axis === "x") [y, h] = seen(y, h, window.innerHeight);
    else [x, w] = seen(x, w, window.innerWidth);
    const long = 16 * k, short = 5 * k;
    const hw = t.axis === "y" ? long : short, hh = t.axis === "y" ? short : long;
    return [x + w / 2 - hw / 2, y + h / 2 - hh / 2, hw, hh];
  };
  // Zone (ou poignée) sous le pointeur, état d'un glisser en cours.
  let hotSide = null;
  let spacing = null;
  // Pointeur au-dessus de l'élément sélectionné (marges comprises) : espacements montrés.
  let overSel = false;
  const spacingEl = () => {
    if (!active || !selected || selectedInst || !selected.isConnected || drag || editing || resizing) return null;
    const el = styled(selected);
    const d = getComputedStyle(el).display;
    return d === "inline" || d === "contents" || d === "none" ? null : el;
  };
  const kOf = () => parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
  const pool = (cls, n) => {
    const list = [...spcBox.querySelectorAll(":scope > ." + cls.split(" ").join("."))];
    while (list.length < n) {
      const d = document.createElement("div");
      d.className = cls;
      spcBox.appendChild(d);
      list.push(d);
    }
    return list;
  };
  const hideSpacing = () => { for (const b of spcBox.children) b.style.display = "none"; };
  const drawSpacing = () => {
    const el = spacingEl();
    if (!el || !(spacing || spacingSel || overSel)) { hideSpacing(); return; }
    const k = kOf();
    const ts = spaceTargets(el);
    const zones = pool("side", ts.length);
    const grips = pool("ph", ts.length);
    const hot = spacing ? spacing.key : hotSide;
    ts.forEach((t, i) => {
      const z = zones[i];
      const [x, y, w, h] = t.rect;
      z.className = "side " + (t.gap ? "g" : t.key[0]) + (t.key === hot ? " hot" : "");
      if (w > 0 && h > 0) Object.assign(z.style, { display: "block", left: x + "px", top: y + "px", width: w + "px", height: h + "px" });
      else z.style.display = "none";
      const g = grips[i];
      const [hx, hy, hw, hh] = handleRect(t, k);
      // Côté trop court pour une poignée : pas de poignée.
      const room = t.axis === "y" ? w : h;
      g.className = "ph" + (t.key === hot ? " hot" : "");
      if (room >= 24 * k) Object.assign(g.style, { display: "block", left: hx + "px", top: hy + "px", width: hw + "px", height: hh + "px" });
      else g.style.display = "none";
    });
    for (let i = ts.length; i < zones.length; i++) zones[i].style.display = "none";
    for (let i = ts.length; i < grips.length; i++) grips[i].style.display = "none";
    spcBox.classList.toggle("spacing", spacingSel);
  };
  // Zone sous le pointeur : `{ t, grip }` (grip : sur la poignée, qui seule se glisse).
  const spacingAt = (x, y) => {
    const el = spacingEl();
    if (!el) return null;
    const k = kOf();
    const ts = spaceTargets(el);
    const tol = 4 * k;
    for (const t of ts) {
      const room = t.axis === "y" ? t.rect[2] : t.rect[3];
      if (room < 24 * k) continue;
      const [hx, hy, hw, hh] = handleRect(t, k);
      if (x >= hx - tol && x <= hx + hw + tol && y >= hy - tol && y <= hy + hh + tol) return { t, grip: true };
    }
    for (const t of ts) {
      const [rx, ry, w, h] = t.rect;
      if (w > 0 && h > 0 && x >= rx && x <= rx + w && y >= ry && y <= ry + h) return { t, grip: false };
    }
    return null;
  };
  // Pointeur au-dessus de l'élément sélectionné, marges comprises.
  const overSelected = (x, y) => {
    const el = spacingEl();
    if (!el) return false;
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return x >= r.left - px(cs.marginLeft) && x <= r.right + px(cs.marginRight) &&
      y >= r.top - px(cs.marginTop) && y <= r.bottom + px(cs.marginBottom);
  };
  // Pointeur de la page : flèche de redimensionnement sur une poignée.
  const cursorStyle = document.createElement("style");
  let cursorAxis = null;
  const setCursor = (axis) => {
    if (axis === cursorAxis) return;
    cursorAxis = axis;
    cursorStyle.textContent = axis ? "*{cursor:" + (axis === "y" ? "ns-resize" : "ew-resize") + "!important}" : "";
    if (axis && !cursorStyle.isConnected) document.documentElement.appendChild(cursorStyle);
  };
  const showValue = (text, x, y) => {
    const k = parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
    svalBox.textContent = text;
    Object.assign(svalBox.style, { display: "block", left: (x + 12 * k) + "px", top: (y + 14 * k) + "px" });
  };

  // Poignées de taille : élément sélectionné (pas une utilisation de composant, pas un élément
  // en ligne, dont la largeur ne se règle pas), cadre actif, hors glisser et texte en cours.
  const resizable = () => {
    if (!active || !selected || selectedInst || !selected.isConnected || editing || drag) return false;
    const el = styled(selected);
    const d = getComputedStyle(el).display;
    return d !== "contents" && d !== "none" && (d !== "inline" || REPLACED.has(el.tagName));
  };
  const drawHandles = (r) => {
    if (!r || (!resizable() && !resizing)) { hide(...handles); return; }
    const k = parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
    const half = 4.5 * k;
    const at = { e: [r.right, r.top + r.height / 2], s: [r.left + r.width / 2, r.bottom], se: [r.right, r.bottom] };
    for (const h of handles) {
      const [x, y] = at[h.dataset.axis];
      h.style.display = "block";
      h.style.left = (x - half) + "px";
      h.style.top = (y - half) + "px";
    }
  };

  // Autres éléments de la sélection multiple : un contour chacun.
  const drawExtra = () => {
    const els = extra.map((id) => findId(id)).filter((f) => f.el && f.el !== selected);
    while (extraBox.children.length < els.length) {
      const b = document.createElement("div");
      b.className = "box sel2";
      extraBox.appendChild(b);
    }
    [...extraBox.children].forEach((b, i) => {
      if (i < els.length) place(b, els[i].el, els[i].inst);
      else b.style.display = "none";
    });
  };

  const draw = () => {
    drawDim();
    drawSpacing();
    drawExtra();
    const same = hovered === selected && hoveredInst === selectedInst;
    place(hoverBox, hovered && !same ? hovered : null, hoveredInst);
    const r = place(selBox, selected, selectedInst);
    if (r && selected) {
      const k = parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
      tagBox.textContent = label(selected, selectedInst);
      tagBox.classList.toggle("comp", !!selectedInst);
      tagBox.style.display = "block";
      tagBox.style.left = Math.max(0, r.left) + "px";
      tagBox.style.top = (r.top >= 20 * k ? r.top - 20 * k : r.bottom + 2 * k) + "px";
    } else {
      tagBox.style.display = "none";
    }
    drawHandles(r && !selectedInst ? styled(selected).getBoundingClientRect() : null);
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

  // Élément qui porte les styles : l'image d'une `<picture>` (ses classes sont sur `<img>`).
  const styled = (el) => (el && el.tagName === "PICTURE" ? el.querySelector("img") || el : el);
  // Éléments remplacés : largeur et hauteur s'appliquent même en ligne.
  const REPLACED = new Set(["IMG", "VIDEO", "IFRAME", "CANVAS", "svg", "SVG", "INPUT", "TEXTAREA", "SELECT", "OBJECT", "EMBED"]);

  // Valeurs calculées des propriétés suivies par le panneau de styles, pour l'élément sélectionné.
  let watched = [];
  let computedTimer = 0;
  // Éléments visés par les actions d'un verbe (`random`, `play`) des interactions de la page
  // (`data-revel-on="click play:… #entete"`), résolus comme le script du site.
  const actionTargets = (verb) => {
    const out = new Set();
    document.querySelectorAll("[data-revel-on]").forEach((el) => {
      for (const part of (el.getAttribute("data-revel-on") || "").split(";")) {
        const w = part.trim().split(/\s+/);
        if (w.length < 3 || !w[1].startsWith(verb)) continue;
        const t = w.slice(2).join(" ");
        let list = [];
        if (t === "self") list = [el];
        else if (t === "parent") list = el.parentElement ? [el.parentElement] : [];
        else if (t === "next") list = el.nextElementSibling ? [el.nextElementSibling] : [];
        else if (t === "body") list = [document.body];
        else if (t.startsWith("closest:")) { try { const c = el.closest(t.slice(8)); if (c) list = [c]; } catch (_) { /* sélecteur invalide */ } }
        else { try { list = [...document.querySelectorAll(t)]; } catch (_) { /* sélecteur invalide */ } }
        list.forEach((x) => out.add(x));
      }
    });
    return out;
  };
  // États utiles à un élément (menu « État » du panneau de styles, mêmes valeurs que `STATES`) :
  // appui pour ce qui réagit au clic, focus pour ce qui se prend au clavier, page affichée pour
  // un lien ; les autres seulement si la page les pose sur l'élément ou sur ce qui le contient
  // (onglets, interrupteurs, médias au survol, élément pris au hasard, accordéon, vidéo arrêtée).
  const statesOf = (el) => {
    const s = [];
    const up = (test) => { for (let p = el.parentElement; p; p = p.parentElement) if (test(p)) return true; return false; };
    if (el.matches("input,select,textarea") || el.closest("a,button,summary,label,[tabindex],[data-revel-on],[data-revel-tab],[data-revel-switch],[data-revel-media-for],[data-revel-prev],[data-revel-next]")) s.push(":active");
    if (el.matches("a[href],button,input,select,textarea,summary,[tabindex]")) s.push(":focus-visible");
    if (el.matches("a[href]")) s.push('[aria-current="page"]');
    const randoms = actionTargets("random");
    const active = (x) => x.matches("[data-revel-tab],[data-revel-switch],[data-revel-media-for]") || (!!x.parentElement && randoms.has(x.parentElement));
    if (active(el)) s.push(".is-active");
    if (up(active)) s.push("^.is-active");
    if (el.matches("details")) s.push("[open]");
    if (up((p) => p.matches("details"))) s.push("^[open]");
    if (el.matches("[data-revel-switch]")) s.push('[aria-pressed="false"]');
    if (up((p) => p.matches("[data-revel-switch]"))) s.push('^[aria-pressed="false"]');
    if (el.matches("[data-revel-hover-media]")) s.push(".is-ready");
    const paused = actionTargets("play");
    if (up((p) => paused.has(p))) s.push("^.is-paused");
    return s;
  };
  const sendComputed = () => {
    clearTimeout(computedTimer);
    computedTimer = setTimeout(() => {
      if (!selected || !selected.isConnected || !watched.length) return;
      const cs = getComputedStyle(styled(selected));
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
      values["--rv-states"] = statesOf(styled(selected)).join("\n");
      post({ type: "computed", id: selected.getAttribute("data-rv"), values });
    }, 30);
  };

  // Textes modifiables de l'élément sélectionné (champ « Texte » du panneau de styles) : textes
  // dont il est l'élément le plus proche ; dans une utilisation de composant, ses propriétés de
  // texte et les textes venus des données.
  const sendTexts = () => {
    const items = [];
    // Texte mis en forme modifié d'un bloc : un seul champ pour tout l'élément (D-72).
    const hostId = selected && !selectedInst ? selected.getAttribute("data-rv") : null;
    if (hostId && richIds.has(hostId)) {
      for (let n = 0; n < ranges.length; n++) {
        const r = ranges[n];
        if (!r || !slots[n] || slots[n].slot !== "text" || !selected.contains(r.startContainer)) continue;
        items.push({ slot: slots[n], text: richHtml(selected), rich: false, host: hostId });
        break;
      }
    }
    if (selected && !items.length) {
      for (let n = 0; n < ranges.length; n++) {
        const r = ranges[n];
        const sl = slots[n];
        if (!r || !editable(sl)) continue;
        let c = r.commonAncestorContainer;
        if (c.nodeType !== 1) c = c.parentElement;
        if (!c) continue;
        const ir = selectedInst ? rangeOfInst(selectedInst) : null;
        // Utilisation dans une liste de collection : les textes d'un contenu de la liste restent
        // dans le CMS (le panneau ne montre que ceux de l'utilisation).
        const ofItem = !!sl.binding && !!sl.binding.source && sl.binding.source.kind === "entry_item";
        const ok = selectedInst
          ? !!ir && ir.intersectsNode(c) && (propOf(sl, selectedInst) || (dataOf(sl) && !ofItem))
          : c.closest("[data-rv]") === selected;
        if (!ok) continue;
        const frag = r.cloneContents();
        const box = document.createElement("div");
        box.appendChild(frag);
        items.push({ slot: sl, text: sl.html ? box.innerHTML : box.textContent, rich: !!sl.html });
        if (items.length >= 8) break;
      }
    }
    post({ type: "texts", id: selected ? idOf(selected, selectedInst) : null, items });
  };
  // Texte rendu des éléments qui affichent une expression (calques : le texte, pas l'expression).
  // Clé : identifiant de l'élément, ou « utilisation/identifiant » dans un composant.
  const sendPreviews = () => {
    const map = {};
    for (let n = 0; n < ranges.length; n++) {
      const r = ranges[n];
      const sl = slots[n];
      if (!r || !sl || sl.slot !== "expr") continue;
      let c = r.commonAncestorContainer;
      if (c.nodeType !== 1) c = c.parentElement;
      const el = c && c.closest("[data-rv]");
      if (!el) continue;
      const ins = around(el);
      const inst = ins.length ? ins[ins.length - 1] : null;
      const key = (inst ? inst.node + "/" : "") + el.getAttribute("data-rv");
      if (key in map) continue;
      map[key] = (el.textContent || "").replace(/\s+/g, " ").trim().slice(0, 120);
    }
    post({ type: "previews", map });
  };
  const select = (el, notify, inst, add) => {
    selected = el;
    selectedInst = el ? inst || null : null;
    if (!add) extra = [];
    sendComputed();
    sendTexts();
    redraw();
    if (notify) {
      post(el
        ? { type: "select", id: idOf(el, selectedInst), tag: selectedInst ? "" : el.tagName.toLowerCase(), label: label(el, selectedInst),
            rect: selectedInst ? rectOf({ getBoundingClientRect: () => boxOf(el, selectedInst) }) : rectOf(el),
            component: selectedInst ? selectedInst.component : undefined, add: !!add }
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
  // Texte riche lié (`{{ … | safe }}` : HTML venu des données) : modifiable avec sa mise en forme.
  const editable = (s) => !!s && (s.slot === "text" || (s.slot === "expr" && !!s.binding && !s.transformed));
  // HTML gardé d'un texte riche modifié : paragraphes, titres, listes, gras, italique, liens.
  const KEEP = new Set(["P", "BR", "STRONG", "B", "EM", "I", "U", "A", "UL", "OL", "LI", "H2", "H3", "H4", "BLOCKQUOTE", "SPAN"]);
  const cleanHtml = (root) => {
    const out = document.createElement("div");
    const copy = (from, to) => {
      for (const n of from.childNodes) {
        if (n.nodeType === 3) { to.appendChild(document.createTextNode(n.data)); continue; }
        if (n.nodeType !== 1) continue;
        if (!KEEP.has(n.tagName)) { copy(n, to); continue; }
        const tag = n.tagName === "B" ? "strong" : n.tagName === "I" ? "em" : n.tagName.toLowerCase();
        if (tag === "span") { copy(n, to); continue; }
        const el = document.createElement(tag);
        if (tag === "a" && n.getAttribute("href")) el.setAttribute("href", n.getAttribute("href"));
        copy(n, el);
        to.appendChild(el);
      }
    };
    copy(root, out);
    return out.innerHTML;
  };
  const BLOCKS = /^(P|DIV|UL|OL|LI|H[1-6]|BLOCKQUOTE|TABLE|SECTION|FIGURE)$/;
  // Mise en forme d'un texte de la page (gras, italique, souligné, lien, span) : seules ces
  // balises restent ; `null` : texte simple, sans mise en forme.
  const INLINE = { STRONG: "strong", B: "strong", EM: "em", I: "em", U: "u", A: "a", SPAN: "span", BR: "br" };
  const cleanInline = (root) => {
    const out = document.createElement("div");
    let rich = false;
    const copy = (from, to) => {
      for (const n of from.childNodes) {
        if (n.nodeType === 3) { to.appendChild(document.createTextNode(n.data)); continue; }
        if (n.nodeType !== 1) continue;
        const tag = INLINE[n.tagName];
        if (!tag) { copy(n, to); continue; }
        rich = true;
        const el = document.createElement(tag);
        if (tag === "a") {
          if (n.getAttribute("href")) el.setAttribute("href", n.getAttribute("href"));
          if (n.getAttribute("target")) el.setAttribute("target", n.getAttribute("target"));
        }
        if (tag === "span" && n.getAttribute("class")) el.setAttribute("class", n.getAttribute("class"));
        copy(n, el);
        to.appendChild(el);
      }
    };
    copy(root, out);
    return rich ? out.innerHTML : null;
  };
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

  // Texte mis en forme modifié d'un bloc (D-72) : un élément qui ne contient que du texte et des
  // éléments de mise en forme (liste envoyée par l'interface) se modifie en entier, comme un seul
  // texte ; ses éléments gardent leur repère `data-rv` (classes et réglages repris).
  let richIds = new Set();
  const RICH = Object.assign({ SMALL: "small", SUP: "sup", SUB: "sub", MARK: "mark", S: "s", CODE: "code" }, INLINE);
  const richHost = (n) => {
    const sl = slots[n];
    const r = ranges[n];
    if (!sl || sl.slot !== "text" || !r || !richIds.size) return null;
    let c = r.commonAncestorContainer;
    if (c.nodeType !== 1) c = c.parentElement;
    let el = c && c.closest("[data-rv]");
    let best = null;
    // Du texte pointé vers l'élément qui le contient : un span ou un gras dans un titre modifie
    // tout le titre.
    while (el) {
      if (richIds.has(el.getAttribute("data-rv"))) best = el;
      else if (best) break;
      if (!RICH[el.tagName]) break;
      el = parentRv(el);
    }
    return best;
  };
  // HTML d'un élément modifié d'un bloc : texte et éléments de mise en forme, avec leur repère.
  const richHtml = (root) => {
    const out = document.createElement("div");
    const copy = (from, to) => {
      for (const n of from.childNodes) {
        if (n.nodeType === 3) { to.appendChild(document.createTextNode(n.data)); continue; }
        if (n.nodeType !== 1) continue;
        const tag = RICH[n.tagName];
        // Span posé par le navigateur (style d'une commande) : son texte seul.
        const stray = tag === "span" && !n.hasAttribute("data-rv") && !n.hasAttribute("class") && n.attributes.length > 0;
        if (!tag || stray) { copy(n, to); continue; }
        const el = document.createElement(tag);
        for (const a of ["data-rv", "class", "href", "target"]) if (n.hasAttribute(a)) el.setAttribute(a, n.getAttribute(a));
        copy(n, el);
        to.appendChild(el);
      }
    };
    copy(root, out);
    return out.innerHTML;
  };
  // Chemin d'un nœud depuis `root` (positions), pour retrouver les repères après une annulation.
  const pathOf = (root, node) => {
    const p = [];
    for (let x = node; x && x !== root; x = x.parentNode) p.unshift(Array.prototype.indexOf.call(x.parentNode.childNodes, x));
    return p;
  };
  const nodeAt = (root, p) => p.reduce((x, i) => (x ? x.childNodes[i] : null), root);
  // Mot sous le pointeur (double-clic) : sélectionné, comme dans un traitement de texte.
  const wordAt = (x, y, within) => {
    if (typeof x !== "number" || !document.caretRangeFromPoint) return null;
    const c = document.caretRangeFromPoint(x, y);
    if (!c || c.startContainer.nodeType !== 3 || !within.contains(c.startContainer)) return null;
    const t = c.startContainer;
    const s = t.data;
    const w = /[\p{L}\p{N}_'’-]/u;
    let a = c.startOffset;
    let b = a;
    while (a > 0 && w.test(s[a - 1])) a--;
    while (b < s.length && w.test(s[b])) b++;
    const r = document.createRange();
    r.setStart(t, a);
    r.setEnd(t, b);
    return r;
  };
  const selectIn = (el, x, y) => {
    const sel = window.getSelection();
    if (!sel) return;
    const w = wordAt(x, y, el);
    if (w && !w.collapsed) { sel.removeAllRanges(); sel.addRange(w); } else sel.selectAllChildren(el);
  };
  // Sortie du texte : validé, sauf vers le champ d'adresse de la barre de mise en forme.
  const watchBlur = (el) => {
    const onBlur = () => {
      setTimeout(() => {
        if (!editing || editing.span !== el) return;
        if (fbar.classList.contains("is-link") || document.activeElement === el) {
          el.addEventListener("blur", onBlur, { once: true });
          return;
        }
        finishEdit(true);
      }, 0);
    };
    el.addEventListener("blur", onBlur, { once: true });
  };
  const plainPaste = (e) => {
    e.preventDefault();
    const t = (e.clipboardData && e.clipboardData.getData("text/plain")) || "";
    document.execCommand("insertText", false, t.replace(/\s*\n\s*/g, " "));
  };
  const startRich = (n, host, x, y) => {
    const saved = host.innerHTML;
    // Repères dans l'élément : positions retrouvées si la saisie est abandonnée.
    const marks = [];
    for (let k = 0; k < ranges.length; k++) {
      const r = ranges[k];
      if (!r || !host.contains(r.startContainer) || !host.contains(r.endContainer)) continue;
      marks.push({ k, sp: pathOf(host, r.startContainer), so: r.startOffset, ep: pathOf(host, r.endContainer), eo: r.endOffset });
    }
    const style = host.getAttribute("style");
    host.setAttribute("data-rv-edit", "");
    host.setAttribute("contenteditable", "true");
    host.style.outline = "2px solid #6d8cff";
    host.style.outlineOffset = "2px";
    host.addEventListener("paste", plainPaste);
    editing = { n, span: host, rich: false, block: { saved, marks, style, before: richHtml(host) } };
    watchBlur(host);
    host.focus();
    selectIn(host, x, y);
    post({ type: "editing", slot: slots[n] });
    return true;
  };
  const finishRich = (commit) => {
    const { n, span: host, block } = editing;
    editing = null;
    hideFormatBar();
    host.removeEventListener("paste", plainPaste);
    host.removeAttribute("contenteditable");
    host.removeAttribute("data-rv-edit");
    if (block.style === null) host.removeAttribute("style");
    else host.setAttribute("style", block.style);
    const html = richHtml(host);
    if (commit && html !== block.before) {
      // Page rechargée après l'enregistrement.
      post({ type: "rich_text", slot: slots[n], id: host.getAttribute("data-rv"), html });
      return;
    }
    // Abandon : texte d'origine et repères rétablis.
    host.innerHTML = block.saved;
    for (const m of block.marks) {
      const s = nodeAt(host, m.sp);
      const e = nodeAt(host, m.ep);
      if (!s || !e) continue;
      const r = document.createRange();
      try { r.setStart(s, m.so); r.setEnd(e, m.eo); ranges[m.k] = r; } catch (_) { /* structure changée */ }
    }
    post({ type: "editing", slot: null });
  };
  // Gras, italique, souligné posés par Revel (pas par le navigateur : un titre déjà gras par son
  // style recevrait un « gras retiré » en style) : la sélection est enveloppée, ou sortie de
  // l'élément qui la contient déjà.
  const FORMAT = { bold: ["strong", "b"], italic: ["em", "i"], underline: ["u"] };
  const toggleFormat = (kind) => {
    const sel = window.getSelection();
    if (!editing || !sel || !sel.rangeCount) return;
    const r = sel.getRangeAt(0);
    if (r.collapsed || !editing.span.contains(r.commonAncestorContainer)) return;
    const tags = FORMAT[kind];
    let wrap = null;
    for (let x = r.commonAncestorContainer; x && x !== editing.span; x = x.parentNode) {
      if (x.nodeType === 1 && tags.includes(x.localName)) { wrap = x; break; }
    }
    if (wrap) {
      // Sortie : l'élément coupé en trois, la partie sélectionnée sans la mise en forme.
      const before = document.createRange();
      before.setStart(wrap, 0);
      before.setEnd(r.startContainer, r.startOffset);
      const after = document.createRange();
      after.setStart(r.endContainer, r.endOffset);
      after.setEnd(wrap, wrap.childNodes.length);
      const fa = after.extractContents();
      const fb = before.extractContents();
      const mid = Array.from(wrap.childNodes);
      if (fb.textContent) { const w1 = wrap.cloneNode(false); w1.appendChild(fb); wrap.before(w1); }
      if (fa.textContent) { const w2 = wrap.cloneNode(false); w2.appendChild(fa); wrap.after(w2); }
      wrap.replaceWith(...mid);
      if (mid.length) {
        const nr = document.createRange();
        nr.setStartBefore(mid[0]);
        nr.setEndAfter(mid[mid.length - 1]);
        sel.removeAllRanges();
        sel.addRange(nr);
      }
      return;
    }
    const frag = r.extractContents();
    // Même mise en forme déjà présente dans la sélection : retirée (une seule enveloppe).
    for (const inner of Array.from(frag.querySelectorAll(tags.join(",")))) inner.replaceWith(...inner.childNodes);
    const el = document.createElement(tags[0]);
    el.appendChild(frag);
    r.insertNode(el);
    sel.removeAllRanges();
    const nr = document.createRange();
    nr.selectNodeContents(el);
    sel.addRange(nr);
  };
  const startEdit = (n, x, y) => {
    const host = richHost(n);
    if (host) return startRich(n, host, x, y);
    const r = ranges[n];
    const rich = !!slots[n].html;
    // Texte riche qui contient des paragraphes ou des listes : enveloppe en bloc.
    const block = rich && Array.from(r.cloneContents().childNodes).some((c) => c.nodeType === 1 && BLOCKS.test(c.tagName));
    const span = document.createElement(block ? "div" : "span");
    span.setAttribute("data-rv-edit", "");
    // Texte de la page aussi en `true` : gras, italique, lien, span (barre de mise en forme) ;
    // un collage arrive en texte simple.
    span.setAttribute("contenteditable", "true");
    if (!rich) span.addEventListener("paste", plainPaste);
    span.style.outline = "2px solid #6d8cff";
    span.style.outlineOffset = "2px";
    try { r.surroundContents(span); } catch (_) { return false; }
    editing = { n, span, rich, before: rich ? span.innerHTML : span.textContent };
    watchBlur(span);
    span.focus();
    selectIn(span, x, y);
    post({ type: "editing", slot: slots[n] });
    return true;
  };
  const finishEdit = (commit) => {
    if (!editing) return;
    if (editing.block) { finishRich(commit); return; }
    hideFormatBar();
    const { n, span, before, rich } = editing;
    if (rich) {
      editing = null;
      const html = commit ? cleanHtml(span) : before;
      const parent = span.parentNode;
      if (!parent) return;
      const tmp = document.createElement("template");
      tmp.innerHTML = html;
      const nodes = Array.from(tmp.content.childNodes);
      const r = document.createRange();
      if (nodes.length) {
        parent.insertBefore(tmp.content, span);
        r.setStartBefore(nodes[0]);
        r.setEndAfter(nodes[nodes.length - 1]);
      } else {
        const empty = document.createTextNode("");
        parent.insertBefore(empty, span);
        r.selectNode(empty);
      }
      parent.removeChild(span);
      ranges[n] = r;
      if (commit && html !== cleanHtml(Object.assign(document.createElement("div"), { innerHTML: before }))) {
        post({ type: "text", slot: slots[n], text: html, before });
      } else post({ type: "editing", slot: null });
      return;
    }
    editing = null;
    hideFormatBar();
    const text = span.textContent;
    // Mise en forme ajoutée : le texte devient texte et éléments (la page est rechargée).
    const html = commit && slots[n].slot === "text" ? cleanInline(span) : null;
    const parent = span.parentNode;
    if (!parent) return;
    const node = document.createTextNode(commit ? text : before);
    parent.replaceChild(node, span);
    const r = document.createRange();
    r.selectNode(node);
    ranges[n] = r;
    parent.normalize();
    if (html !== null) post({ type: "text_html", slot: slots[n], html });
    else if (commit && text !== before) post({ type: "text", slot: slots[n], text, before });
    else post({ type: "editing", slot: null });
  };

  // Barre de mise en forme (comme Webflow) : au-dessus du texte sélectionné pendant la
  // modification ; gras, italique, lien, span (texte à styler à part).
  const fbar = document.createElement("div");
  fbar.className = "fbar";
  // Mêmes tracés que les icônes de l'interface (16 px, trait de 1,5 px).
  const fbIcon = (d) => "<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' " +
    "stroke-linecap='round' stroke-linejoin='round'><path d='" + d + "'/></svg>";
  fbar.innerHTML =
    "<button data-c='bold'>" + fbIcon("M5 3h4a2.5 2.5 0 0 1 0 5H5zM5 8h4.8a2.5 2.5 0 0 1 0 5H5z") + "</button>" +
    "<button data-c='italic'>" + fbIcon("M6.5 3h6M3.5 13h6M9.5 3l-3 10") + "</button>" +
    "<button data-c='link'>" + fbIcon("M6.5 9.5l3-3M7 4.5l1-1a2.8 2.8 0 0 1 4 4l-1 1M9 11.5l-1 1a2.8 2.8 0 0 1-4-4l1-1") + "</button>" +
    "<button data-c='span'>" + fbIcon("M5.5 3H3.5v10h2M10.5 3h2v10h-2") + "</button>" +
    "<input class='fb-url' placeholder='https://… /page/'>";
  root.appendChild(fbar);
  const fbUrl = fbar.querySelector(".fb-url");
  let fbRange = null;
  // Événement sur la barre (racine fermée : repérée par sa position).
  const onBar = (e) => {
    if (fbar.style.display !== "flex" || typeof e.clientX !== "number") return false;
    const r = fbar.getBoundingClientRect();
    return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
  };
  const hideFormatBar = () => { fbar.style.display = "none"; fbar.classList.remove("is-link"); fbRange = null; };
  hideFormatBar();
  const formatTitles = (t) => {
    if (!t) return;
    for (const b of fbar.querySelectorAll("button")) if (t[b.dataset.c]) b.title = t[b.dataset.c];
    if (t.url) fbUrl.placeholder = t.url;
  };
  const showFormatBar = () => {
    const sel = window.getSelection();
    if (!editing || !sel || sel.rangeCount === 0 || sel.isCollapsed || !editing.span.contains(sel.anchorNode)) {
      if (!fbar.classList.contains("is-link")) fbar.style.display = "none";
      return;
    }
    const b = sel.getRangeAt(0).getBoundingClientRect();
    const k = parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
    // État des boutons : gras, italique ou lien déjà appliqués à la sélection.
    const inTag = (tags) => {
      for (let n = sel.anchorNode; n && n !== editing.span; n = n.parentNode) if (n.nodeType === 1 && tags.includes(n.localName)) return true;
      return false;
    };
    // Span : actif seulement si la sélection en est tout le texte (sinon un clic en crée un
    // autre dedans).
    const exactSpan = () => {
      for (let n = sel.anchorNode; n && n !== editing.span; n = n.parentNode) {
        if (n.nodeType === 1 && n.localName === "span") return sel.toString() === n.textContent;
      }
      return false;
    };
    const on = { bold: inTag(["strong", "b"]), italic: inTag(["em", "i"]), link: inTag(["a"]), span: exactSpan() };
    for (const btn of fbar.querySelectorAll("button")) btn.classList.toggle("on", !!on[btn.dataset.c]);
    fbar.style.display = "flex";
    fbar.style.left = Math.max(4, b.left + b.width / 2 - 70 * k) + "px";
    fbar.style.top = Math.max(4, b.top - 34 * k) + "px";
  };
  document.addEventListener("selectionchange", () => { if (editing) showFormatBar(); });
  // Boutons : la sélection du texte reste (appui sans prise du focus).
  fbar.addEventListener("mousedown", (e) => { if (e.target !== fbUrl) e.preventDefault(); });
  fbar.addEventListener("pointerdown", (e) => e.stopPropagation(), true);
  fbar.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b || !editing) return;
    e.stopPropagation();
    const c = b.dataset.c;
    const sel = window.getSelection();
    // Élément de mise en forme qui contient déjà la sélection.
    const around = (tag) => {
      for (let n = sel && sel.anchorNode; n && n !== editing.span; n = n.parentNode) if (n.nodeType === 1 && n.localName === tag) return n;
      return null;
    };
    if (c === "bold" || c === "italic") toggleFormat(c);
    else if (c === "span") {
      // Deuxième appui sur un span sélectionné en entier : retiré ; sinon un span de plus.
      const cur = around("span");
      const old = cur && sel.toString() === cur.textContent ? cur : null;
      if (old) {
        // Deuxième appui : le span est retiré, son texte reste.
        const r = document.createRange();
        const first = old.firstChild, last = old.lastChild;
        old.replaceWith(...old.childNodes);
        if (first && last) { r.setStartBefore(first); r.setEndAfter(last); sel.removeAllRanges(); sel.addRange(r); }
      } else if (sel && sel.rangeCount && !sel.isCollapsed) {
        const r = sel.getRangeAt(0);
        const sp = document.createElement("span");
        sp.appendChild(r.extractContents());
        r.insertNode(sp);
        sel.selectAllChildren(sp);
      }
    } else if (c === "link") {
      fbRange = sel && sel.rangeCount ? sel.getRangeAt(0).cloneRange() : null;
      const a = around("a");
      fbar.classList.add("is-link");
      fbUrl.value = a ? a.getAttribute("href") || "" : "";
      fbUrl.focus();
      fbUrl.select();
      return;
    }
    showFormatBar();
  });
  // Champ d'adresse quitté ailleurs que vers le texte : modification validée.
  fbUrl.addEventListener("blur", () => setTimeout(() => {
    if (editing && fbar.classList.contains("is-link") && document.activeElement !== editing.span) {
      fbar.classList.remove("is-link");
      finishEdit(true);
    }
  }, 0));
  fbUrl.addEventListener("keydown", (e) => {
    e.stopPropagation();
    if (e.key === "Escape") { hideFormatBar(); if (editing) editing.span.focus(); return; }
    if (e.key !== "Enter") return;
    e.preventDefault();
    const url = fbUrl.value.trim();
    const r = fbRange;
    fbar.classList.remove("is-link");
    if (!editing) return;
    editing.span.focus();
    const sel = window.getSelection();
    if (r && sel) { sel.removeAllRanges(); sel.addRange(r); }
    if (url) document.execCommand("createLink", false, url);
    else document.execCommand("unlink");
    showFormatBar();
  });

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
    if (v) { window.scrollBy({ top: v, behavior: "instant" }); probe(drag.x, drag.y); }
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
    if (e.button !== 0 || editing || inEdit(e.target) || onBar(e)) return;
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

  // ---------------------------------------------------------------- poignées de taille
  // Glisser une poignée : largeur et/ou hauteur de l'élément (valeurs de la propriété CSS, en px,
  // `box-sizing` respecté) ; Maj sur le coin : proportions gardées. L'élément suit en direct (style
  // en ligne prioritaire, retiré quand la feuille du site est rechargée) ; au relâchement,
  // l'interface écrit les valeurs dans la classe. Double-clic : valeur retirée (taille automatique).
  // Poignée sous le pointeur (le calque est une racine fermée : les événements de la page n'en
  // voient que l'hôte, d'où un repérage par position).
  const handleAt = (x, y) => {
    for (const h of handles) {
      if (h.style.display !== "block") continue;
      const r = h.getBoundingClientRect();
      if (x >= r.left - 1 && x <= r.right + 1 && y >= r.top - 1 && y <= r.bottom + 1) return h.dataset.axis;
    }
    return null;
  };
  const onHandle = (e) => typeof e.clientX === "number" && handleAt(e.clientX, e.clientY) !== null;
  let resizing = null;
  let resized = false;
  let held = null;
  const keep = (el, prop) => ({ prop, value: el.style.getPropertyValue(prop), prio: el.style.getPropertyPriority(prop) });
  const putBack = (h) => {
    if (!h) return;
    for (const s of h.saved) {
      if (s.value) h.el.style.setProperty(s.prop, s.value, s.prio);
      else h.el.style.removeProperty(s.prop);
    }
    if (!h.el.getAttribute("style")) h.el.removeAttribute("style");
  };
  const release_held = () => { if (held) { clearTimeout(held.timer); putBack(held); held = null; redraw(); } };
  const startResize = (axis, e) => {
    release_held();
    const el = styled(selected);
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const content = cs.boxSizing !== "border-box";
    const padX = content ? px(cs.paddingLeft) + px(cs.paddingRight) + px(cs.borderLeftWidth) + px(cs.borderRightWidth) : 0;
    const padY = content ? px(cs.paddingTop) + px(cs.paddingBottom) + px(cs.borderTopWidth) + px(cs.borderBottomWidth) : 0;
    resizing = { el, axis, id: rvOf(selected), x0: e.clientX, y0: e.clientY, w0: r.width, h0: r.height, padX, padY,
      ratio: r.height > 0 ? r.width / r.height : 1, w: null, h: null, saved: [keep(el, "width"), keep(el, "height")] };
    try { document.documentElement.setPointerCapture(e.pointerId); } catch (_) { /* relâché */ }
    hide(hoverBox);
    hideSpacing();
  };
  const moveResize = (e) => {
    const rs = resizing;
    const dx = e.clientX - rs.x0;
    const dy = e.clientY - rs.y0;
    let w = rs.axis !== "s" ? Math.max(1, Math.round(rs.w0 + dx)) : null;
    let h = rs.axis !== "e" ? Math.max(1, Math.round(rs.h0 + dy)) : null;
    if (rs.axis === "se" && e.shiftKey) h = Math.max(1, Math.round(w / rs.ratio));
    rs.w = w === null ? null : Math.max(0, Math.round(w - rs.padX));
    rs.h = h === null ? null : Math.max(0, Math.round(h - rs.padY));
    if (rs.w !== null) rs.el.style.setProperty("width", rs.w + "px", "important");
    if (rs.h !== null) rs.el.style.setProperty("height", rs.h + "px", "important");
    sizeBox.textContent = rs.w !== null && rs.h !== null ? rs.w + " \u00d7 " + rs.h : (rs.w !== null ? rs.w : rs.h) + " px";
    const k = parseFloat(getComputedStyle(host).getPropertyValue("--k")) || 1;
    Object.assign(sizeBox.style, { display: "block", left: (e.clientX + 12 * k) + "px", top: (e.clientY + 14 * k) + "px" });
    redraw();
  };
  const endResize = (cancel) => {
    const rs = resizing;
    resizing = null;
    sizeBox.style.display = "none";
    resized = true;
    setTimeout(() => { resized = false; }, 0);
    const changed = !cancel && ((rs.w !== null && Math.round(rs.w + rs.padX) !== Math.round(rs.w0)) ||
      (rs.h !== null && Math.round(rs.h + rs.padY) !== Math.round(rs.h0)));
    if (!changed) { putBack(rs); redraw(); return; }
    // Taille gardée jusqu'au rechargement de la feuille (ou de la page) ; sinon rétablie.
    held = { el: rs.el, saved: rs.saved, timer: setTimeout(release_held, 6000) };
    post({ type: "resized", id: rs.id, width: rs.w === null ? undefined : rs.w + "px", height: rs.h === null ? undefined : rs.h + "px" });
    redraw();
  };
  window.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || !onHandle(e) || !resizable()) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    startResize(handleAt(e.clientX, e.clientY), e);
  }, true);
  window.addEventListener("pointermove", (e) => {
    if (!resizing) return;
    e.stopImmediatePropagation();
    moveResize(e);
  }, true);
  window.addEventListener("pointerup", (e) => {
    if (!resizing) return;
    e.stopImmediatePropagation();
    endResize(false);
  }, true);
  window.addEventListener("pointercancel", () => { if (resizing) endResize(true); }, true);

  // Glisser la poignée d'un côté de l'élément sélectionné : espacement intérieur, marge ou écart
  // entre enfants (px) ; Maj : les quatre côtés, Alt : le côté opposé aussi. L'élément suit en
  // direct ; au relâchement, l'interface écrit les valeurs dans la classe. Clic sur une poignée
  // sans glisser : champ correspondant du panneau de styles.
  const startSpacing = (t, e) => {
    release_held();
    const el = styled(selected);
    const kind = t.key[0];
    const keys = t.gap ? [] : e.shiftKey ? Object.keys(SIDES).filter((k) => k[0] === kind)
      : e.altKey ? [t.key, OPPOSITE[t.key]] : [t.key];
    const props = t.gap ? [t.prop] : keys.map((k) => SIDES[k].prop);
    spacing = { el, key: t.key, axis: t.axis, sign: t.sign, prop: t.prop, props, id: rvOf(selected),
      x0: e.clientX, y0: e.clientY, v0: t.v, v: null, saved: props.map((p) => keep(el, p)) };
    try { document.documentElement.setPointerCapture(e.pointerId); } catch (_) { /* relâché */ }
    hide(hoverBox);
  };
  const moveSpacing = (e) => {
    const sp = spacing;
    const d = sp.axis === "y" ? e.clientY - sp.y0 : e.clientX - sp.x0;
    if (sp.v === null && Math.abs(d) < 2) return;
    const v = Math.max(0, Math.round(sp.v0 + sp.sign * d));
    sp.v = v;
    for (const p of sp.props) sp.el.style.setProperty(p, v + "px", "important");
    showValue(v + " px", e.clientX, e.clientY);
    redraw();
  };
  const endSpacing = (cancel) => {
    const sp = spacing;
    spacing = null;
    svalBox.style.display = "none";
    resized = true;
    setTimeout(() => { resized = false; }, 0);
    if (!cancel && sp.v === null) {
      post({ type: "spacing_focus", prop: sp.prop });
      redraw();
      return;
    }
    if (cancel || sp.v === Math.round(sp.v0)) { putBack(sp); redraw(); return; }
    held = { el: sp.el, saved: sp.saved, timer: setTimeout(release_held, 6000) };
    const values = {};
    for (const p of sp.props) values[p] = sp.v + "px";
    post({ type: "spacing", id: sp.id, values });
    redraw();
  };
  window.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || onHandle(e)) return;
    const at = spacingAt(e.clientX, e.clientY);
    if (!at || !at.grip) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    startSpacing(at.t, e);
  }, true);
  window.addEventListener("pointermove", (e) => {
    if (spacing) { e.stopImmediatePropagation(); moveSpacing(e); return; }
    if (resizing) return;
    const over = overSelected(e.clientX, e.clientY);
    const at = over ? spacingAt(e.clientX, e.clientY) : null;
    const key = at ? at.t.key : null;
    if (over !== overSel || key !== hotSide) {
      overSel = over;
      hotSide = key;
      redraw();
    }
    setCursor(at && at.grip ? at.t.axis : null);
    if (at) showValue(Math.round(at.t.v) + " px", e.clientX, e.clientY);
    else svalBox.style.display = "none";
  }, true);
  window.addEventListener("pointerup", (e) => {
    if (!spacing) return;
    e.stopImmediatePropagation();
    endSpacing(false);
  }, true);
  window.addEventListener("pointercancel", () => { if (spacing) endSpacing(true); }, true);
  document.addEventListener("pointerleave", () => {
    if (spacing || (!overSel && !hotSide)) return;
    overSel = false;
    hotSide = null;
    setCursor(null);
    svalBox.style.display = "none";
    redraw();
  });

  // Les clics sélectionnent : liens, boutons et formulaires du site ne réagissent pas.
  const block = (e) => { e.preventDefault(); e.stopPropagation(); };
  document.addEventListener("click", (e) => {
    if (inEdit(e.target) || onBar(e)) return;
    if (editing) finishEdit(true);
    block(e);
    if (dragged || onHandle(e) || resized) return;
    const p = pick(e.target);
    // Maj+clic : élément ajouté à la sélection (ou retiré), géré par l'interface.
    if (e.shiftKey && p && selected) { post({ type: "toggle", id: idOf(p.el, p.inst), tag: p.inst ? "" : p.el.tagName.toLowerCase(), label: label(p.el, p.inst) }); return; }
    select(p ? p.el : null, true, p ? p.inst : null);
  }, true);
  // Clic droit : élément sélectionné, menu de l'éditeur (pas celui du navigateur).
  document.addEventListener("contextmenu", (e) => {
    if (inEdit(e.target) || onBar(e)) return;
    block(e);
    if (onHandle(e)) return;
    const p = pick(e.target);
    if (p && (p.el !== selected || p.inst !== selectedInst)) select(p.el, true, p.inst);
    post({ type: "context", x: e.clientX, y: e.clientY });
  }, true);
  const propOf = (sl, inst) => !!sl && sl.slot === "expr" && !!sl.binding && !!sl.binding.source &&
    sl.binding.source.kind === "prop" && sl.binding.source.node === inst.node;
  // Texte venu des données du site (nom du site, menu, contenu) affiché dans un composant :
  // modifié sur place comme ailleurs (la donnée change, pas le composant).
  const dataOf = (sl) => !!sl && sl.slot === "expr" && !!sl.binding && !!sl.binding.source &&
    sl.binding.source.kind !== "prop" && !sl.transformed;
  document.addEventListener("dblclick", (e) => {
    if (inEdit(e.target) || onBar(e)) return;
    block(e);
    if (onHandle(e)) {
      const axis = handleAt(e.clientX, e.clientY);
      if (selected && resizable()) post({ type: "resized", id: rvOf(selected), reset: axis });
      return;
    }
    const n = slotAt(e.clientX, e.clientY);
    const p = pick(e.target);
    if (p && p.inst) {
      // Texte d'une propriété de cette utilisation : modifié sur place (valeur de l'utilisation).
      if (n !== null && (propOf(slots[n], p.inst) || dataOf(slots[n]))) { startEdit(n, e.clientX, e.clientY); return; }
      const inner = targetOf(e.target);
      // Texte écrit dans le composant : modifié aussitôt, sur place (un seul double-clic).
      if (inner && n !== null && slots[n] && slots[n].slot === "text") {
        isolated = p.inst.component;
        select(inner, true, null);
        startEdit(n, e.clientX, e.clientY);
        return;
      }
      // Sinon : canevas du composant (ses variantes), élément sous le pointeur sélectionné.
      post({ type: "open_component", component: p.inst.component,
        node: inner ? inner.getAttribute("data-rv") : null, tag: inner ? inner.tagName.toLowerCase() : "" });
      return;
    }
    if (n !== null) startEdit(n, e.clientX, e.clientY);
  }, true);
  for (const type of ["auxclick", "submit", "mouseup", "pointerup"]) {
    document.addEventListener(type, (e) => { if (!inEdit(e.target) && !onBar(e)) block(e); }, true);
  }
  // Appui : pas de sélection de texte ni de focus d'un champ du site, mais le canevas reçoit le
  // clavier (Échap, raccourcis).
  for (const type of ["mousedown", "pointerdown"]) {
    document.addEventListener(type, (e) => {
      if (inEdit(e.target) || onBar(e)) return;
      block(e);
      window.focus();
    }, true);
  }

  document.addEventListener("keydown", (e) => {
    if (editing) {
      if (e.key === "Escape") { e.preventDefault(); finishEdit(false); }
      // Gras, italique, souligné (texte riche et texte de la page).
      else if ((e.ctrlKey || e.metaKey) && /^[biu]$/i.test(e.key)) {
        e.preventDefault();
        toggleFormat({ b: "bold", i: "italic", u: "underline" }[e.key.toLowerCase()]);
        showFormatBar();
      }
      // Texte riche : Entrée ajoute un paragraphe, Ctrl+Entrée valide.
      else if (editing.rich) {
        if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); finishEdit(true); }
      } else if (e.key === "Enter" && e.shiftKey) {
        // Maj+Entrée : retour à la ligne dans le texte (balise `br`).
        e.preventDefault();
        document.execCommand("insertLineBreak");
      } else if (e.key === "Enter") { e.preventDefault(); finishEdit(true); }
      return;
    }
    if (e.key === "Escape") {
      if (drag && drag.move) { release(e, true); return; }
      if (resizing) { endResize(true); return; }
      select(null, true);
      return;
    }
    // Espace maintenue : déplacement de la vue (comme Figma), géré par l'interface.
    if (e.key === " " || e.code === "Space") {
      e.preventDefault();
      // Ctrl+Espace : palette de commandes (comme Ctrl+K), transmise une fois par appui.
      if ((e.ctrlKey || e.metaKey) && !e.repeat) {
        post({ type: "key", key: "k", code: "KeyK", shift: false, alt: false, ctrl: true, meta: false });
        return;
      }
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
  // Page entièrement visible dans ce sens (cadre allongé à la hauteur de la page) : la vue de
  // l'éditeur se déplace (comme Framer) ; sinon la page défile.
  window.addEventListener("wheel", (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) { e.preventDefault(); post({ type: "zoom", dy: e.deltaY || e.deltaX, x: e.clientX, y: e.clientY }); return; }
    const se = document.scrollingElement || document.documentElement;
    const canY = se.scrollHeight > innerHeight + 1;
    const canX = se.scrollWidth > innerWidth + 1;
    let inner = false;
    for (let el = e.target; el && el !== document.documentElement && el.nodeType === 1; el = el.parentElement) {
      const cs = getComputedStyle(el);
      if (/(auto|scroll)/.test(cs.overflowY + cs.overflowX) && (el.scrollHeight > el.clientHeight || el.scrollWidth > el.clientWidth)) { inner = true; break; }
    }
    if (!inner && !(canY && e.deltaY) && !(canX && e.deltaX) && (e.deltaY || e.deltaX)) {
      e.preventDefault();
      post({ type: "pan", dx: -e.deltaX, dy: -e.deltaY });
    }
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
  // Composant seul, variante de geste : feuille des règles recopiées (voir plus bas).
  const forceCss = (css) => (stageState ? css.split(":" + stageState).join(".rv-force-" + stageState) : css);
  let forcedSheet = null;
  const buildForced = () => {
    if (!stageState || !("adoptedStyleSheets" in document) || typeof CSSStyleSheet !== "function") return;
    const pseudo = ":" + stageState;
    const out = [];
    const walk = (rules, wrap) => {
      for (const r of rules) {
        if (r.selectorText !== undefined && r.style) {
          if (r.selectorText.includes(pseudo)) out.push(wrap(forceCss(r.selectorText) + "{" + r.style.cssText + "}"));
        } else if (r.media && r.cssRules) {
          walk(r.cssRules, (t) => wrap("@media " + r.media.mediaText + "{" + t + "}"));
        } else if (r.cssRules) {
          walk(r.cssRules, wrap);
        }
      }
    };
    for (const sh of document.styleSheets) {
      let rules = null;
      try { rules = sh.cssRules; } catch (_) { rules = null; }
      if (rules) walk(rules, (t) => t);
    }
    if (!forcedSheet) {
      forcedSheet = new CSSStyleSheet();
      document.adoptedStyleSheets = [...document.adoptedStyleSheets, forcedSheet];
    }
    try { forcedSheet.replaceSync(out.join("\n")); } catch (_) { /* règle illisible */ }
  };
  const setPreview = (css) => {
    css = forceCss(css);
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

  // ---------------------------------------------------------------- cadre allongé
  // Hauteur normale du cadre (`--rv-vh`, 1 % de cette hauteur) : les unités `vh` de la page,
  // rapportées à cette variable par Revel, ne changent pas quand le cadre est allongé.
  let vhSheet = null;
  let vhEl = null;
  const setViewport = (h) => {
    const css = h > 0 ? ":root{--rv-vh:" + (h / 100) + "px}" : "";
    if ("adoptedStyleSheets" in document && typeof CSSStyleSheet === "function" && "replaceSync" in CSSStyleSheet.prototype) {
      if (!vhSheet) {
        vhSheet = new CSSStyleSheet();
        document.adoptedStyleSheets = [...document.adoptedStyleSheets, vhSheet];
      }
      vhSheet.replaceSync(css);
    } else if (css) {
      if (!vhEl) vhEl = document.createElement("style");
      vhEl.textContent = css;
      if (!vhEl.isConnected) document.head.appendChild(vhEl);
    } else if (vhEl) {
      vhEl.remove();
    }
    redraw();
  };

  // Élément montré dans la page : défilement de la page seule (jamais des cadres de l'éditeur
  // autour, que `scrollIntoView` ferait aussi défiler).
  // `band` : partie de la page visible dans l'éditeur ({ top, bottom, left, right }, coordonnées
  // de la fenêtre de la page), sinon toute la fenêtre.
  const reveal = (el, band) => {
    const r = el.getBoundingClientRect();
    const b = band || { top: 0, bottom: window.innerHeight, left: 0, right: window.innerWidth };
    const m = 16;
    const along = (a, z, lo, hi) => (a < lo ? a - lo - m : z > hi ? Math.min(a - lo - m, z - hi + m) : 0);
    const dx = along(r.left, r.right, b.left, b.right), dy = along(r.top, r.bottom, b.top, b.bottom);
    // Sans animation (`scroll-behavior: smooth` du site) : la position est mesurée aussitôt.
    if (dx || dy) window.scrollBy({ left: dx, top: dy, behavior: "instant" });
  };

  // ---------------------------------------------------------------- éléments vides
  // Élément sans contenu ni taille (bloc, section, grille… tout juste ajouté) : dans le canevas
  // seulement, haut d'environ 50 px avec l'icône de son type, pour le retrouver et le sélectionner.
  const EMPTY_ICONS = {
    box: '<rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/>',
    section: '<path d="M1.5 3h13M1.5 13h13"/><rect x="3.5" y="5.5" width="9" height="5" rx="1"/>',
    grid: '<rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="9" y="2.5" width="4.5" height="4.5" rx="1"/>' +
      '<rect x="2.5" y="9" width="4.5" height="4.5" rx="1"/><rect x="9" y="9" width="4.5" height="4.5" rx="1"/>',
    flex: '<rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="M5.5 2.5v11M10.5 2.5v11"/>',
    list: '<path d="M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.01M2.5 8h.01M2.5 12h.01"/>',
    lottie: '<path d="M8 2.5v3M8 10.5v3M2.5 8h3M10.5 8h3M4.2 4.2l1.6 1.6M10.2 10.2l1.6 1.6M4.2 11.8l1.6-1.6M10.2 5.8l1.6-1.6"/>',
  };
  const emptyCss = "[data-rv-empty]{min-height:50px!important;min-width:50px!important;" +
    "background:rgba(109,140,255,.07) center/20px no-repeat!important;outline:1px dashed rgba(109,140,255,.6)!important;outline-offset:-1px}" +
    Object.entries(EMPTY_ICONS).map(([k, d]) => '[data-rv-empty="' + k + '"]{background-image:url("data:image/svg+xml,' +
      encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="#6d8cff" ' +
        'stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">' + d + "</svg>") + '")!important}').join("");
  if ("adoptedStyleSheets" in document && typeof CSSStyleSheet === "function") {
    try {
      const sh = new CSSStyleSheet();
      sh.replaceSync(emptyCss);
      document.adoptedStyleSheets = [...document.adoptedStyleSheets, sh];
    } catch (_) { /* moteur sans feuilles construites */ }
  }
  const CONTAINERS = new Set(["DIV", "SECTION", "HEADER", "FOOTER", "MAIN", "ARTICLE", "ASIDE", "NAV", "FORM", "FIGURE",
    "UL", "OL", "LI", "DETAILS", "BLOCKQUOTE"]);
  const markEmpty = () => {
    for (const el of document.querySelectorAll("[data-rv-empty]")) el.removeAttribute("data-rv-empty");
    const found = [];
    for (const el of document.querySelectorAll("[data-rv]")) {
      if (!CONTAINERS.has(el.tagName)) continue;
      if ([...el.childNodes].some((c) => c.nodeType === 1 || (c.nodeType === 3 && c.nodeValue.trim()))) continue;
      // Masqué (lui ou un parent) : laissé tel quel.
      if (!el.getClientRects().length) continue;
      // Décor vide à dessein (barre de progression, élément caché aux lecteurs d'écran).
      if (el.closest('[data-revel-progress],[aria-hidden="true"]')) continue;
      const r = el.getBoundingClientRect();
      if (r.height >= 2 && r.width >= 2) continue;
      found.push(el);
    }
    for (const el of found) {
      const d = getComputedStyle(el).display;
      el.setAttribute("data-rv-empty",
        el.hasAttribute("data-lottie") ? "lottie"
          : /grid/.test(d) ? "grid"
            : /flex/.test(d) ? "flex"
              : /^(SECTION|HEADER|FOOTER|MAIN|ARTICLE|ASIDE|NAV)$/.test(el.tagName) ? "section"
                : /^(UL|OL|LI)$/.test(el.tagName) ? "list" : "box");
    }
    if (found.length) redraw();
  };
  markEmpty();

  // ---------------------------------------------------------------- messages de l'interface
  const byId = (id) => (id ? document.querySelector('[data-rv="' + CSS.escape(id) + '"]') : null);
  // Élément (ou utilisation de composant) désigné par l'interface.
  const findId = (id) => {
    if (!id) return { el: null, inst: null };
    const inst = instances.find((i) => i.node === id && i.root);
    if (inst) return { el: inst.root, inst };
    return { el: byId(id), inst: null };
  };

  // Feuille de style rechargée (même chemin) : la nouvelle remplace l'ancienne une fois chargée,
  // sans éclair ni animation rejouée.
  // ---------------------------------------------------------------- mode sombre (D-115)
  // Thème montré : les requêtes `prefers-color-scheme` des feuilles du site deviennent vraies ou
  // fausses selon le choix de l'éditeur, quel que soit le thème de l'appareil (le cadre suit
  // sinon celui du système, différemment selon le moteur).
  let scheme = null;
  let stateOpened = null;
  const SCHEME_RE = /\(\s*prefers-color-scheme\s*:\s*(dark|light)\s*\)/g;
  const applyScheme = () => {
    const walk = (rules) => {
      for (const r of rules) {
        if (r.media && r.cssRules) {
          if (r.__rvMedia === undefined) r.__rvMedia = r.media.mediaText;
          const orig = r.__rvMedia;
          if (/prefers-color-scheme/.test(orig)) {
            const next = scheme
              ? orig.replace(SCHEME_RE, (_, v) => (v === scheme ? "(min-width: 0px)" : "(max-width: 0.001px)"))
              : orig;
            if (r.media.mediaText !== next) { try { r.media.mediaText = next; } catch (_) { /* moteur ancien */ } }
          }
          walk(r.cssRules);
        } else if (r.cssRules) {
          walk(r.cssRules);
        }
      }
    };
    for (const sh of document.styleSheets) {
      let rules = null;
      try { rules = sh.cssRules; } catch (_) { rules = null; }
      if (rules) walk(rules);
    }
    document.documentElement.toggleAttribute("data-rv-scheme-dark", scheme === "dark");
  };
  window.addEventListener("load", () => { if (scheme) applyScheme(); });

  const reloadSheet = (path) => {
    for (const l of document.querySelectorAll('link[rel~="stylesheet"]')) {
      const u = new URL(l.href, location.href);
      if (u.origin !== location.origin || u.pathname !== path) continue;
      u.searchParams.set("rv", String(Date.now()));
      const next = l.cloneNode();
      next.href = u.pathname + u.search;
      next.addEventListener("load", () => { l.remove(); setPreview(""); release_held(); if (scheme) applyScheme(); buildForced(); markEmpty(); redraw(); sendComputed(); }, { once: true });
      l.after(next);
    }
  };

  // ---------------------------------------------------------------- mise à jour sur place
  // Après une modification, la page rendue de nouveau est comparée à celle affichée : seuls les
  // nœuds changés sont remplacés. Pas de rechargement : position, images, vidéos et animations
  // Lottie gardées, animations de la page non rejouées. En-tête changé (police, code ajouté) ou
  // rendu illisible : rechargement complet.
  const keyOf = (n) => (n.nodeType === 1 && n.hasAttribute("data-rv") ? n.tagName + "#" + n.getAttribute("data-rv") : null);
  const fits = (a, b) => a.nodeType === b.nodeType && (a.nodeType !== 1 || (a.tagName === b.tagName && keyOf(a) === keyOf(b)));
  const syncAttrs = (cur, next, keep) => {
    for (const a of [...cur.attributes]) {
      if (!next.hasAttribute(a.name) && !(keep && keep.includes(a.name))) cur.removeAttribute(a.name);
    }
    for (const a of [...next.attributes]) {
      if (cur.getAttribute(a.name) !== a.value && !(keep && keep.includes(a.name))) cur.setAttribute(a.name, a.value);
    }
  };
  const lottieOf = (el) => [...el.attributes].filter((a) => a.name.startsWith("data-lottie"))
    .map((a) => a.name + "=" + a.value).sort().join(" ");
  const morphNode = (cur, next) => {
    if (cur.nodeType !== 1) {
      if (cur.nodeValue !== next.nodeValue) cur.nodeValue = next.nodeValue;
      return;
    }
    if (cur.__rvLottie) {
      // Animation Lottie : lecteur gardé si ses réglages n'ont pas changé (son style aussi).
      if (lottieOf(cur) === lottieOf(next)) { syncAttrs(cur, next, ["style"]); return; }
      try { cur.__rvLottie.destroy(); } catch (_) { /* déjà retirée */ }
      cur.__rvLottie = null;
    }
    syncAttrs(cur, next);
    if (cur.tagName === "TEMPLATE") { cur.innerHTML = next.innerHTML; return; }
    morphChildren(cur, next);
  };
  const morphChildren = (parent, src) => {
    const list = [...src.childNodes];
    const wanted = new Set(list.map(keyOf).filter(Boolean));
    const spare = (x) => !keyOf(x) || !wanted.has(keyOf(x));
    let cur = parent.firstChild;
    for (const n of list) {
      if (cur && fits(cur, n)) { morphNode(cur, n); cur = cur.nextSibling; continue; }
      // Élément repéré plus loin : les nœuds qui le précèdent sont retirés s'ils ne servent plus
      // (l'élément reste en place), sinon il est déplacé.
      const k = keyOf(n);
      let found = null;
      if (k) for (let x = cur; x; x = x.nextSibling) if (keyOf(x) === k) { found = x; break; }
      if (found) {
        let before = true;
        for (let x = cur; x !== found; x = x.nextSibling) if (!spare(x)) { before = false; break; }
        if (before) {
          while (cur !== found) { const nx = cur.nextSibling; parent.removeChild(cur); cur = nx; }
          morphNode(found, n);
          cur = found.nextSibling;
        } else {
          parent.insertBefore(found, cur);
          morphNode(found, n);
        }
        continue;
      }
      // Nouveau nœud (ses scripts ne sont pas exécutés).
      parent.insertBefore(document.importNode(n, true), cur);
    }
    while (cur) { const nx = cur.nextSibling; parent.removeChild(cur); cur = nx; }
  };
  let refreshing = false;
  let refreshAgain = false;
  const refreshPage = async () => {
    if (refreshing) { refreshAgain = true; return; }
    // Texte en cours de modification, glisser ou réglage en cours : mise à jour juste après.
    if (editing || drag || resizing || spacing) { setTimeout(refreshPage, 150); return; }
    refreshing = true;
    try {
      const u = new URL(location.href);
      u.searchParams.set("rvt", String(Date.now()));
      const res = await fetch(u.pathname + u.search, { cache: "no-store" });
      // Page qui ne s'affiche pas (expression illisible…) : la page actuelle reste, la raison est
      // donnée à l'interface ; une annulation la remet à jour sur place.
      if (res.status >= 500) {
        post({ type: "render_error", text: (await res.text()).slice(0, 400) });
        return;
      }
      // Page déplacée ou retirée : rechargée (l'interface affiche la page demandée).
      if (!res.ok) throw new Error(String(res.status));
      const doc = new DOMParser().parseFromString(await res.text(), "text/html");
      if (headSig(doc.head) !== headAtLoad) { location.reload(); return; }
      const sEl = doc.getElementById("rv-slots");
      const vEl = doc.getElementById("rv-vars");
      const nextSlots = JSON.parse(sEl ? sEl.textContent : "[]");
      const nextVars = JSON.parse(vEl ? vEl.textContent : "[]");
      if (sEl) rev = Number(sEl.getAttribute("data-rev")) || rev;
      for (const x of [sEl, vEl, ...doc.querySelectorAll('script[src="/__revel/canvas.js"]')]) if (x) x.remove();
      // Liens de la page actuelle (à recharger) : feuilles du même site.
      const sheets = [...doc.head.querySelectorAll('link[rel~="stylesheet"]')]
        .map((l) => new URL(l.getAttribute("href") || "", location.href))
        .filter((x) => x.origin === location.origin)
        .map((x) => x.pathname);
      slots = nextSlots;
      vars = nextVars;
      if (document.title !== doc.title) document.title = doc.title;
      for (const a of ["lang", "dir"]) {
        const v = doc.documentElement.getAttribute(a);
        if (v === null) document.documentElement.removeAttribute(a);
        else if (document.documentElement.getAttribute(a) !== v) document.documentElement.setAttribute(a, v);
      }
      const selId = selected ? idOf(selected, selectedInst) : null;
      syncAttrs(document.body, doc.body);
      morphChildren(document.body, doc.body);
      scanMarkers();
      buildInstances();
      if (STAGE && stageState && instances[0] && instances[0].root) instances[0].root.classList.add("rv-force-" + stageState);
      if (window.RevelLottie) window.RevelLottie.refresh();
      markEmpty();
      for (const path of new Set(sheets)) reloadSheet(path);
      // Sélection retrouvée par son identifiant (l'interface la confirme sur `refreshed`).
      hovered = null;
      hoveredInst = null;
      const again = findId(selId);
      selected = again.el;
      selectedInst = again.inst;
      redraw();
      sendPreviews();
      post({ type: "refreshed", vars, rev });
    } catch (_) {
      location.reload();
      return;
    } finally {
      refreshing = false;
    }
    if (refreshAgain) { refreshAgain = false; refreshPage(); }
  };

  window.addEventListener("message", (e) => {
    const m = e.data;
    if (!m || m.revel !== 1 || e.source !== window.parent) return;
    switch (m.type) {
      case "select": {
        const { el, inst } = findId(m.id);
        select(el, false, inst);
        if (el && m.reveal) reveal(el);
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
      case "animate": {
        // Aperçu d'une animation du panneau Interactions (6.1) : script d'animation chargé à la
        // demande, sans rien lancer d'autre (`RevelAnimationsManual`).
        const { el } = findId(m.id);
        if (!el || editing) break;
        const go = () => window.RevelAnimations && window.RevelAnimations.preview(el, m.family, m.value);
        if (window.RevelAnimations) go();
        else {
          window.RevelAnimationsManual = true;
          const sc = document.createElement("script");
          sc.src = "/assets/revel-animations.js";
          sc.onload = go;
          document.head.appendChild(sc);
        }
        break;
      }
      case "attrs":
        // Attributs modifiés (classes, animations, liens…) : posés sur chaque rendu de l'élément,
        // sans recharger la page.
        for (const c of m.changes || []) {
          // Éléments repérés, et racine de chaque utilisation du composant (attributs posés sur
          // elle pour cette utilisation).
          const els = [...document.querySelectorAll('[data-rv="' + CSS.escape(c.id) + '"]'),
            ...instances.filter((i) => i.node === c.id && i.root).map((i) => i.root)];
          for (const el of els) {
            for (const [n, v] of Object.entries(c.set || {})) {
              try { el.setAttribute(n, v); } catch (_) { /* nom invalide */ }
            }
            for (const n of c.remove || []) el.removeAttribute(n);
            // Animation Lottie : relue avec ses nouveaux réglages (fichier, lecture, boucle).
            const lottie = [...Object.keys(c.set || {}), ...(c.remove || [])].some((n) => n.startsWith("data-lottie"));
            if (lottie && el.__rvLottie) {
              try { el.__rvLottie.destroy(); } catch (_) { /* déjà retirée */ }
              el.__rvLottie = null;
              el.style.aspectRatio = "";
            }
          }
        }
        if (window.RevelLottie) window.RevelLottie.refresh();
        markEmpty();
        // Animation Lottie chargée : l'élément n'est plus vide.
        setTimeout(markEmpty, 400);
        redraw();
        break;
      case "rich":
        // Éléments de texte mis en forme modifiés d'un bloc (D-72).
        richIds = new Set(Array.isArray(m.ids) ? m.ids : []);
        if (!editing) sendTexts();
        break;
      case "labels":
        tagNames = m.tags || null;
        otherName = m.other || "";
        formatTitles(m.format);
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
        // Aperçu retiré (geste refusé) : taille tenue par les poignées rétablie.
        if (!m.css) release_held();
        setPreview(m.css || "");
        break;
      case "state-preview": {
        // État choisi dans le panneau de styles : accordéon de la sélection ouvert le temps du réglage.
        if (stateOpened) { stateOpened.open = !!stateOpened.__rvWasOpen; stateOpened = null; }
        const st = String(m.state || "");
        const d = (st === "[open]" || st === "^[open]") && selected ? selected.closest("details") : null;
        if (d) { d.__rvWasOpen = d.open; d.open = true; stateOpened = d; }
        redraw();
        sendComputed();
        break;
      }
      case "scheme":
        scheme = m.value === "dark" || m.value === "light" ? m.value : null;
        applyScheme();
        sendComputed();
        break;
      case "stylesheet":
        // Feuille de style régénérée : rechargée sans recharger la page.
        reloadSheet(m.url);
        break;
      case "refresh":
        // Page modifiée : mise à jour sur place.
        refreshPage();
        break;
      case "scale":
        // Zoom de l'éditeur : traits, poignées et étiquettes à taille constante à l'écran.
        host.style.setProperty("--k", String(1 / Math.max(0.05, Number(m.s) || 1)));
        redraw();
        break;
      case "active":
        active = !!m.on;
        redraw();
        break;
      case "viewport":
        setViewport(Number(m.h) || 0);
        break;
      case "multi":
        // Autres éléments sélectionnés (sélection multiple).
        extra = Array.isArray(m.ids) ? m.ids : [];
        redraw();
        break;
      case "spacing":
        // Rubrique « Espacement » du panneau de styles survolée : espacements de la sélection.
        spacingSel = !!m.on;
        redraw();
        break;
      case "wheel":
        // Vue verrouillée de l'éditeur : molette hors de la page, la page défile.
        window.scrollBy({ left: Number(m.dx) || 0, top: Number(m.dy) || 0, behavior: "instant" });
        break;
      case "scroll":
        window.scrollTo({ left: m.x || 0, top: m.y || 0, behavior: "instant" });
        break;
      case "drag":
        // Glisser depuis l'interface (panneau « Ajouter ») : début, position, fin.
        if (m.phase === "start") startDrag({ el: null, tag: m.tag || "div", block: !!m.block, move: false });
        else if (m.phase === "move") probe(m.x, m.y);
        else if (m.phase === "out") { if (drag) { drag.out = true; showDrop(null); lastDrop = null; } }
        else endDrag();
        break;
      case "ui-colors": {
        // Couleur des composants choisie dans les réglages de Revel (D-125).
        if (m.comp) document.documentElement.style.setProperty("--rv-comp", m.comp);
        else document.documentElement.style.removeProperty("--rv-comp");
        break;
      }
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
          reveal(next);
        }
        break;
      }
      case "edit": {
        // Double-clic sur un texte dans les calques : premier texte modifiable de l'élément.
        const { el } = findId(m.id);
        if (!el || editing) break;
        for (let n = 0; n < ranges.length; n++) {
          const r = ranges[n];
          if (!r || !editable(slots[n])) continue;
          let c = r.commonAncestorContainer;
          if (c.nodeType !== 1) c = c.parentElement;
          if (!c || c.closest("[data-rv]") !== el) continue;
          if (selected !== el) select(el, true, null);
          reveal(el);
          window.focus();
          startEdit(n);
          break;
        }
        break;
      }
      case "measure": {
        // Position de la sélection (zoom sur la sélection), après l'avoir montrée.
        const el = findId(m.id).el || selected;
        if (el) reveal(el, m.band || null);
        post({ type: "measure", rect: el ? rectOf(el) : null, reveal: !!m.reveal });
        break;
      }
      default:
        break;
    }
  });

  // ---------------------------------------------------------------- composant seul
  // Canevas des composants (comme Framer) : le composant au milieu de la page, une variante par
  // cadre. Variante de geste (`rvstate=hover|active`) : règles `:hover` / `:active` recopiées sur
  // la classe `rv-force-…` posée sur la racine du composant, pour la montrer sans le pointeur.
  if (STAGE) {
    const pad = document.createElement("style");
    pad.textContent = "body{margin:0!important;padding:32px!important;min-height:0!important}";
    document.head.appendChild(pad);
    if (stageState && instances[0] && instances[0].root) instances[0].root.classList.add("rv-force-" + stageState);
    buildForced();
    // Hauteur du composant (cadres ajustés à leur contenu).
    const sendStage = () => {
      const root = instances[0] && instances[0].root;
      const r = root ? root.getBoundingClientRect() : null;
      post({ type: "stage", h: Math.ceil((r ? r.bottom + window.scrollY : document.body.scrollHeight) + 32) });
    };
    sendStage();
    if (typeof ResizeObserver === "function") new ResizeObserver(sendStage).observe(document.body);
  }

  post({
    type: "ready",
    path: location.pathname,
    title: document.title,
    slots: slots.length,
    vars,
    rev,
    scroll: { x: window.scrollX, y: window.scrollY },
  });
  sendPreviews();
})();
