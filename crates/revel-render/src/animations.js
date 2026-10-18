/* Revel — animations du site (étape 6.1), sans bibliothèque extérieure.
 *
 * Chaque élément animé porte un attribut par moment, réglé dans le panneau Interactions :
 *   data-revel-appear="monter dur=0.6 delay=0 ease=smooth dist=32 stagger=0 trigger=view replay"
 *   data-revel-text, data-revel-hover, data-revel-press, data-revel-loop, data-revel-scroll,
 *   data-revel-active (joue chaque fois que l'élément, ou l'onglet, le filtre ou l'interrupteur
 *   qui le contient, devient actif)
 * Premier mot : l'animation ; ensuite les réglages (clé=valeur, ou drapeau seul).
 *
 * Web Animations (transformations ajoutées à celles de l'élément : `composite: "add"`),
 * IntersectionObserver pour l'arrivée à l'écran et la pause des boucles hors de l'écran.
 * Contenu visible sans JavaScript ; rien n'est animé pour les visiteurs qui demandent moins de
 * mouvement (`prefers-reduced-motion`). `window.RevelAnimationsManual` : rien n'est lancé
 * (éditeur de Revel), seulement `RevelAnimations.preview(élément, moment, valeur)`.
 */
(function () {
  "use strict";
  if (window.RevelAnimations) return;
  var doc = document;
  var root = doc.documentElement;
  var FAMILIES = ["appear", "text", "hover", "press", "loop", "scroll", "active"];
  var EASE = {
    smooth: "cubic-bezier(.22,1,.36,1)",
    bounce: "cubic-bezier(.34,1.56,.64,1)",
    ease: "cubic-bezier(.65,0,.35,1)",
    linear: "linear",
  };
  var reduced = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  var manual = !!window.RevelAnimationsManual;

  // ------------------------------------------------------------------ réglages
  // Valeurs par défaut de chaque moment, puis de certaines animations.
  var DEFAULTS = {
    appear: { dur: 0.6, delay: 0, ease: "smooth", dist: 32, stagger: 0, trigger: "view" },
    text: { dur: 0.5, delay: 0, ease: "smooth", split: "words", trigger: "view" },
    hover: { dur: 0.25, ease: "smooth", amount: 100 },
    press: { dur: 0.12, ease: "smooth", amount: 100 },
    loop: { dur: 2, delay: 0, ease: "ease", amount: 100 },
    scroll: { amount: 100 },
    active: { dur: 0.25, ease: "bounce", amount: 100 },
  };
  var PRESET_DEFAULTS = {
    "appear:pop": { ease: "bounce" },
    "text:pop": { ease: "bounce" },
    "text:typewriter": { split: "chars", dur: 0.01, stagger: 0.05 },
    "loop:spin": { ease: "linear", dur: 4 },
    "loop:shake": { dur: 0.8 },
    "loop:blink": { dur: 1.2 },
    "loop:bounce": { dur: 1.2 },
    "loop:heartbeat": { dur: 1.6 },
    "loop:ticker": { ease: "linear", dir: "left", speed: 60, fade: 0 },
  };
  var STAGGER = { chars: 0.03, words: 0.06, lines: 0.12 };

  function parse(family, value) {
    var parts = String(value || "").trim().split(/\s+/);
    var o = { preset: parts[0] || "", given: {} };
    var base = DEFAULTS[family] || {};
    var k;
    for (k in base) o[k] = base[k];
    var extra = PRESET_DEFAULTS[family + ":" + o.preset] || {};
    for (k in extra) o[k] = extra[k];
    for (var i = 1; i < parts.length; i++) {
      var p = parts[i];
      var eq = p.indexOf("=");
      if (eq < 0) { o[p] = true; o.given[p] = true; continue; }
      o.given[p.slice(0, eq)] = true;
      var v = p.slice(eq + 1);
      var n = Number(v);
      o[p.slice(0, eq)] = v !== "" && !isNaN(n) ? n : v;
    }
    if (family === "text" && o.stagger === undefined) o.stagger = STAGGER[o.split] || 0.06;
    o.easing = EASE[o.ease] || o.ease || EASE.smooth;
    o.k = (typeof o.amount === "number" ? o.amount : 100) / 100;
    return o;
  }

  // ------------------------------------------------------------------ images clés
  function tr(s) { return { transform: s }; }
  function merge(a, b) { var r = {}, k; for (k in a) r[k] = a[k]; for (k in b) r[k] = b[k]; return r; }

  // Apparition (et chaque morceau d'un texte) : état de départ -> état final.
  function enter(preset, d) {
    switch (preset) {
      case "fade": return [{ opacity: 0 }, { opacity: 1 }];
      case "up": return [merge({ opacity: 0 }, tr("translateY(" + d + "px)")), merge({ opacity: 1 }, tr("translateY(0px)"))];
      case "down": return [merge({ opacity: 0 }, tr("translateY(" + -d + "px)")), merge({ opacity: 1 }, tr("translateY(0px)"))];
      case "left": return [merge({ opacity: 0 }, tr("translateX(" + -d + "px)")), merge({ opacity: 1 }, tr("translateX(0px)"))];
      case "right": return [merge({ opacity: 0 }, tr("translateX(" + d + "px)")), merge({ opacity: 1 }, tr("translateX(0px)"))];
      case "zoom-in": return [merge({ opacity: 0 }, tr("scale(0.9)")), merge({ opacity: 1 }, tr("scale(1)"))];
      case "zoom-out": return [merge({ opacity: 0 }, tr("scale(1.1)")), merge({ opacity: 1 }, tr("scale(1)"))];
      case "blur": return [{ opacity: 0, filter: "blur(12px)" }, { opacity: 1, filter: "blur(0px)" }];
      case "pop": return [merge({ opacity: 0 }, tr("scale(0.5)")), merge({ opacity: 1 }, tr("scale(1)"))];
      case "flip": return [merge({ opacity: 0 }, tr("perspective(800px) rotateX(70deg)")), merge({ opacity: 1 }, tr("perspective(800px) rotateX(0deg)"))];
      case "rotate": return [merge({ opacity: 0 }, tr("translateY(" + d / 2 + "px) rotate(-8deg)")), merge({ opacity: 1 }, tr("translateY(0px) rotate(0deg)"))];
      case "reveal": return [{ clipPath: "inset(100% 0% 0% 0%)" }, { clipPath: "inset(0% 0% 0% 0%)" }];
      case "typewriter": return [{ opacity: 0 }, { opacity: 1 }];
      default: return null;
    }
  }

  // Survol et appui : état normal -> état actif (intensité `k`).
  function active(family, preset, k, el) {
    var shadow = function (s) { return [{ boxShadow: getComputedStyle(el).boxShadow }, { boxShadow: s }]; };
    switch (family + ":" + preset) {
      case "hover:grow": return [tr("scale(1)"), tr("scale(" + (1 + 0.05 * k) + ")")];
      case "hover:shrink": return [tr("scale(1)"), tr("scale(" + (1 - 0.05 * k) + ")")];
      case "hover:lift": {
        var s = shadow("0 " + Math.round(12 * k) + "px " + Math.round(28 * k) + "px rgba(0,0,0,.18)");
        return [merge(s[0], tr("translateY(0px)")), merge(s[1], tr("translateY(" + -6 * k + "px)"))];
      }
      case "hover:shadow": return shadow("0 " + Math.round(16 * k) + "px " + Math.round(40 * k) + "px rgba(0,0,0,.22)");
      case "hover:dim": {
        var o = parseFloat(getComputedStyle(el).opacity) || 1;
        return [{ opacity: o }, { opacity: Math.max(0, o * (1 - 0.3 * k)) }];
      }
      case "hover:tilt": return [tr("rotate(0deg)"), tr("rotate(" + 3 * k + "deg)")];
      case "hover:brighten": return [{ filter: "brightness(1)" }, { filter: "brightness(" + (1 + 0.15 * k) + ")" }];
      case "hover:underline": return [{ backgroundSize: "0% 1px" }, { backgroundSize: "100% 1px" }];
      case "press:press": return [tr("scale(1)"), tr("scale(" + (1 - 0.05 * k) + ")")];
      case "press:squish": return [tr("scale(1, 1)"), tr("scale(" + (1 + 0.04 * k) + ", " + (1 - 0.08 * k) + ")")];
      case "press:push": return [tr("translateY(0px)"), tr("translateY(" + 2 * k + "px)")];
      default: return null;
    }
  }

  // Boucle : un cycle ; `alternate` : aller-retour.
  function cycle(preset, k) {
    switch (preset) {
      case "spin": return { frames: [tr("rotate(0deg)"), tr("rotate(360deg)")] };
      case "pulse": return { frames: [tr("scale(1)"), tr("scale(" + (1 + 0.06 * k) + ")")], alternate: true };
      case "float": return { frames: [tr("translateY(0px)"), tr("translateY(" + -10 * k + "px)")], alternate: true };
      case "swing": return { frames: [tr("rotate(" + -6 * k + "deg)"), tr("rotate(" + 6 * k + "deg)")], alternate: true };
      case "shake": {
        var x = 4 * k;
        return { frames: [0, -x, x, -x, x, 0].map(function (v) { return tr("translateX(" + v + "px)"); }) };
      }
      case "blink": return { frames: [{ opacity: 1 }, { opacity: Math.max(0, 1 - 0.7 * k) }], alternate: true };
      case "bounce": {
        var h = -16 * k;
        return { frames: [
          merge(tr("translateY(0px)"), { offset: 0, easing: "cubic-bezier(.33,0,.66,0)" }),
          merge(tr("translateY(" + h + "px)"), { offset: 0.4, easing: "cubic-bezier(.33,1,.66,1)" }),
          merge(tr("translateY(0px)"), { offset: 0.8 }),
          merge(tr("translateY(0px)"), { offset: 1 })] };
      }
      case "heartbeat": {
        var b = "scale(" + (1 + 0.12 * k) + ")";
        return { frames: [[0, "scale(1)"], [0.15, b], [0.3, "scale(1)"], [0.45, b], [0.6, "scale(1)"], [1, "scale(1)"]]
          .map(function (f) { return { offset: f[0], transform: f[1] }; }) };
      }
      default: return null;
    }
  }

  // Devenu actif : un aller-retour joué une fois.
  function once(preset, k) {
    switch (preset) {
      case "pop": return [tr("scale(1)"), tr("scale(" + (1 + 0.5 * k) + ")"), tr("scale(1)")];
      case "jump": return [tr("translateY(0px)"), tr("translateY(" + -12 * k + "px)"), tr("translateY(0px)")];
      case "shake": {
        var x = 6 * k;
        return [0, -x, x, -x, x, 0].map(function (v) { return tr("translateX(" + v + "px)"); });
      }
      case "flash": return [{ opacity: 1 }, { opacity: Math.max(0, 1 - 0.7 * k) }, { opacity: 1 }];
      case "spin": return [tr("rotate(0deg)"), tr("rotate(360deg)")];
      default: return null;
    }
  }
  function activeAnim(el, o) {
    var frames = once(o.preset, o.k);
    if (!frames) return [];
    return play(el, frames, { duration: o.dur * 1000, easing: o.easing, fill: "none" });
  }

  // Défilement : de l'arrivée de l'élément en bas de la fenêtre à sa sortie en haut.
  function travel(preset, k) {
    switch (preset) {
      case "parallax": return [tr("translateY(" + -80 * k + "px)"), tr("translateY(" + 80 * k + "px)")];
      case "fade-in": return [{ opacity: 0 }, { opacity: 1, offset: 0.4 }, { opacity: 1 }];
      case "zoom-in": return [tr("scale(" + (1 - 0.15 * k) + ")"), merge(tr("scale(1)"), { offset: 0.45 }), tr("scale(1)")];
      case "rotate": return [tr("rotate(" + -10 * k + "deg)"), tr("rotate(" + 10 * k + "deg)")];
      case "slide-x": return [tr("translateX(" + 120 * k + "px)"), tr("translateX(" + -120 * k + "px)")];
      case "fade-out": return [
        merge({ opacity: 1 }, tr("scale(1)")),
        merge({ opacity: 1, offset: 0.6 }, tr("scale(1)")),
        merge({ opacity: 0 }, tr("scale(" + (1 - 0.05 * k) + ")"))];
      default: return null;
    }
  }

  // Styles des morceaux de texte, du texte lu par les lecteurs d'écran, du bandeau.
  var css = doc.createElement("style");
  css.textContent = ".rv-w,.rv-c{display:inline-block}.rv-w{white-space:nowrap}" +
    ".rv-sr{position:absolute;width:1px;height:1px;margin:-1px;padding:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}";
  (doc.head || root).appendChild(css);

  // Les transformations ne s'appliquent pas à un élément en ligne : il devient un bloc en ligne.
  function ensureBox(el) {
    if (getComputedStyle(el).display === "inline") { el.style.display = "inline-block"; return true; }
    return false;
  }

  // ------------------------------------------------------------------ lecture
  // Transformations et filtres ajoutés à ceux de l'élément ; le reste remplacé le temps de
  // l'animation. Une animation Web par groupe de propriétés.
  var ADDED = { transform: 1, filter: 1 };
  function play(el, frames, timing) {
    var add = [], rep = [], hasAdd = false, hasRep = false;
    frames.forEach(function (f) {
      var a = {}, r = {};
      for (var key in f) {
        if (key === "offset" || key === "easing") { a[key] = f[key]; r[key] = f[key]; }
        else if (ADDED[key]) { a[key] = f[key]; hasAdd = true; }
        else { r[key] = f[key]; hasRep = true; }
      }
      add.push(a); rep.push(r);
    });
    var out = [];
    if (hasAdd) out.push(el.animate(add, merge(timing, { composite: "add" })));
    if (hasRep) out.push(el.animate(rep, timing));
    return out;
  }

  function stopAll(list) { (list || []).forEach(function (a) { a.cancel(); }); }

  // ------------------------------------------------------------------ texte découpé
  var SKIP = /^(BR|SVG|IMG|VIDEO|IFRAME|SCRIPT|STYLE|TEXTAREA|INPUT|SELECT)$/;
  function span(cls) { var s = doc.createElement("span"); s.className = cls; return s; }
  function split(el, mode) {
    if (el.__rvPieces) return el.__rvPieces;
    var full = el.textContent;
    var pieces = [];
    var walk = function (node) {
      Array.prototype.slice.call(node.childNodes).forEach(function (n) {
        if (n.nodeType === 3) {
          var frag = doc.createDocumentFragment();
          n.data.split(/(\s+)/).forEach(function (part) {
            if (!part) return;
            if (/^\s+$/.test(part)) { frag.appendChild(doc.createTextNode(part)); return; }
            var w = span("rv-w");
            w.setAttribute("aria-hidden", "true");
            if (mode === "chars") {
              Array.from(part).forEach(function (ch) { var c = span("rv-c"); c.textContent = ch; w.appendChild(c); pieces.push(c); });
            } else {
              w.textContent = part;
              pieces.push(w);
            }
            frag.appendChild(w);
          });
          n.parentNode.replaceChild(frag, n);
        } else if (n.nodeType === 1 && !SKIP.test(n.tagName)) {
          walk(n);
        }
      });
    };
    walk(el);
    var sr = span("rv-sr");
    sr.textContent = full;
    el.insertBefore(sr, el.firstChild);
    el.__rvPieces = pieces;
    return pieces;
  }

  // Rang de chaque morceau : son ordre, ou sa ligne (mots à la même hauteur).
  function ranks(pieces, mode) {
    if (mode !== "lines") return pieces.map(function (_, i) { return i; });
    var tops = [];
    var t = pieces.map(function (p) {
      var y = Math.round(p.getBoundingClientRect().top);
      if (tops.indexOf(y) < 0) tops.push(y);
      return y;
    });
    tops.sort(function (a, b) { return a - b; });
    return t.map(function (y) { return tops.indexOf(y); });
  }

  // ------------------------------------------------------------------ moments
  // Apparition : l'élément, ou ses enfants l'un après l'autre (`stagger`).
  function appear(el, o) {
    var frames = enter(o.preset, o.dist);
    if (!frames) return [];
    el.classList.add("rv-in");
    var targets = o.stagger > 0 && el.children.length ? Array.prototype.slice.call(el.children) : [el];
    var out = [];
    targets.forEach(function (t, i) {
      out = out.concat(play(t, frames, {
        duration: o.dur * 1000, delay: (o.delay + i * (o.stagger || 0)) * 1000, easing: o.easing, fill: "backwards",
      }));
    });
    return out;
  }

  function text(el, o) {
    var frames = enter(o.preset, 12);
    if (!frames) return [];
    var pieces = split(el, o.split === "chars" ? "chars" : "words");
    var r = ranks(pieces, o.split);
    el.classList.add("rv-in");
    var out = [];
    pieces.forEach(function (p, i) {
      out = out.concat(play(p, frames, {
        duration: Math.max(1, o.dur * 1000), delay: (o.delay + r[i] * o.stagger) * 1000, easing: o.easing, fill: "backwards",
      }));
    });
    return out;
  }

  // Survol, appui : état actif atteint puis quitté (lecture inversée).
  function toggler(el, family, o) {
    var anims = null, on = false;
    return function (want) {
      if (want === on) return;
      on = want;
      if (!anims) {
        if (family === "hover" && o.preset === "underline") {
          el.style.backgroundImage = "linear-gradient(currentColor, currentColor)";
          el.style.backgroundRepeat = "no-repeat";
          el.style.backgroundPosition = "0 100%";
          el.style.backgroundSize = "0% 1px";
        }
        var frames = active(family, o.preset, o.k, el);
        if (!frames) return;
        anims = play(el, frames, { duration: o.dur * 1000, easing: o.easing, fill: "both" });
        return;
      }
      anims.forEach(function (a) { a.playbackRate = want ? 1 : -1; a.play(); });
    };
  }

  function loop(el, o) {
    if (o.preset === "ticker") return ticker(el, o);
    var c = cycle(o.preset, o.k);
    if (!c) return [];
    var alt = o.mirror === undefined ? c.alternate : o.mirror === true || o.mirror === "on";
    return play(el, c.frames, {
      duration: o.dur * 1000, delay: o.delay * 1000, iterations: Infinity,
      direction: alt ? "alternate" : "normal", easing: c.frames.some(function (f) { return f.easing; }) ? "linear" : o.easing,
    });
  }

  // Bandeau défilant (élément « Défilement ») : enfants recopiés jusqu'à remplir deux longueurs,
  // défilement d'une période. Sens `left`, `right` (horizontal), `up`, `down` (vertical : hauteur
  // de l'élément fixée par son style) ; vitesse en px/s (`dur` écrit : durée d'une période) ;
  // bords estompés (`fade`, px) ; arrêt au survol (`pause`).
  function ticker(el, o) {
    if (el.__rvTrack) return el.__rvTrack.anims;
    var cs = getComputedStyle(el);
    var vertical = o.dir === "up" || o.dir === "down";
    var gap = parseFloat(vertical ? cs.rowGap : cs.columnGap) || 0;
    var track = doc.createElement("div");
    track.className = "rv-track";
    track.style.cssText = "display:flex;flex-wrap:nowrap;gap:" + gap + "px;align-items:" + cs.alignItems +
      (vertical ? ";flex-direction:column;height:max-content" : ";width:max-content");
    while (el.firstChild) track.appendChild(el.firstChild);
    el.appendChild(track);
    el.style.overflow = "hidden";
    var fade = Number(o.fade) || 0;
    if (fade > 0) {
      var m = "linear-gradient(" + (vertical ? "to bottom" : "to right") + ",transparent,#000 " + fade + "px,#000 calc(100% - " + fade + "px),transparent)";
      el.style.webkitMaskImage = m;
      el.style.maskImage = m;
    }
    var originals = Array.prototype.slice.call(track.children);
    var r = track.getBoundingClientRect();
    var period = (vertical ? r.height : r.width) + gap;
    var b = el.getBoundingClientRect();
    var box = vertical ? b.height : b.width;
    var copies = Math.max(1, Math.ceil(box / Math.max(1, period)));
    for (var n = 0; n < copies; n++) {
      originals.forEach(function (c) {
        var d = c.cloneNode(true);
        d.setAttribute("aria-hidden", "true");
        d.removeAttribute("data-rv");
        track.appendChild(d);
      });
    }
    var axis = vertical ? "translateY(" : "translateX(";
    var back = o.dir === "right" || o.dir === "down";
    var frames = [tr(axis + (back ? -period : 0) + "px)"), tr(axis + (back ? 0 : -period) + "px)")];
    var secs = o.given.dur || !(Number(o.speed) > 0) ? Number(o.dur) || 20 : period / Number(o.speed);
    var anims = play(track, frames, { duration: Math.max(0.1, secs) * 1000, iterations: Infinity, easing: "linear" });
    if (o.pause) {
      el.addEventListener("pointerenter", function () { anims.forEach(function (a) { a.pause(); }); });
      el.addEventListener("pointerleave", function () { if (!el.__rvTrack.stopped) anims.forEach(function (a) { a.play(); }); });
    }
    el.__rvTrack = { anims: anims, track: track, stopped: false };
    return anims;
  }

  function scrollAnims(el, o) {
    var frames = travel(o.preset, o.k);
    if (!frames) return [];
    var anims = play(el, frames, { duration: 1000, fill: "both", easing: "linear" });
    anims.forEach(function (a) { a.pause(); });
    return anims;
  }

  // ------------------------------------------------------------------ site publié
  var scrolls = [];
  function docTop(el) {
    if (el.offsetParent === undefined) return el.getBoundingClientRect().top + scrollY;
    var t = 0;
    for (var e = el; e; e = e.offsetParent) t += e.offsetTop;
    return t;
  }
  var measured = false;
  function measure() {
    scrolls.forEach(function (s) { s.top = docTop(s.el); s.h = s.el.offsetHeight || s.el.getBoundingClientRect().height; });
    measured = true;
  }
  var pending = false;
  function update() {
    pending = false;
    if (!measured) measure();
    var vh = innerHeight;
    scrolls.forEach(function (s) {
      var p = (scrollY + vh - s.top) / (vh + s.h);
      p = Math.min(1, Math.max(0, p));
      s.anims.forEach(function (a) { a.currentTime = p * 999; });
    });
  }
  function onScroll() { if (!pending) { pending = true; requestAnimationFrame(update); } }

  function start() {
    var inView = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        var el = e.target, st = el.__rv;
        if (!st) return;
        if (e.isIntersecting) {
          FAMILIES.forEach(function (f) {
            var o = st[f];
            if (!o || o.played || o.trigger === "load") return;
            if (f === "appear") { o.played = true; o.anims = appear(el, o); }
            else if (f === "text") { o.played = true; o.anims = text(el, o); }
          });
          if (st.loop && st.loop.anims) st.loop.anims.forEach(function (a) { a.play(); });
        } else {
          ["appear", "text"].forEach(function (f) {
            var o = st[f];
            if (o && o.replay && o.played) { stopAll(o.anims); o.played = false; el.classList.remove("rv-in"); }
          });
          if (st.loop && st.loop.anims) st.loop.anims.forEach(function (a) { a.pause(); });
        }
      });
    }, { rootMargin: "0px 0px -10% 0px", threshold: 0 });

    var sel = FAMILIES.map(function (f) { return "[data-revel-" + f + "]"; }).join(",");
    Array.prototype.forEach.call(doc.querySelectorAll(sel), function (el) {
      var st = (el.__rv = {});
      FAMILIES.forEach(function (f) {
        var v = el.getAttribute("data-revel-" + f);
        if (v !== null) st[f] = parse(f, v);
      });
      if (st.appear || st.hover || st.press || st.loop || st.scroll || st.active) ensureBox(el);
      if (st.appear && st.appear.trigger === "load") { st.appear.played = true; st.appear.anims = appear(el, st.appear); }
      if (st.text && st.text.trigger === "load") { st.text.played = true; st.text.anims = text(el, st.text); }
      if (st.hover) {
        var h = toggler(el, "hover", st.hover);
        el.addEventListener("pointerenter", function (e) { if (e.pointerType === "mouse") h(true); });
        el.addEventListener("pointerleave", function () { h(false); });
      }
      if (st.press) {
        var p = toggler(el, "press", st.press);
        el.addEventListener("pointerdown", function () { p(true); });
        ["pointerup", "pointerleave", "pointercancel"].forEach(function (n) { el.addEventListener(n, function () { p(false); }); });
      }
      if (st.loop) st.loop.anims = loop(el, st.loop);
      if (st.active) {
        // Élément suivi : l'onglet, le filtre ou l'interrupteur qui contient l'élément (ou
        // lui-même) ; joué quand il reçoit la classe `is-active`.
        var w = el.closest("[data-revel-tab],[data-revel-filter],[data-revel-switch]") || el;
        var was = w.classList.contains("is-active");
        new MutationObserver(function () {
          var now = w.classList.contains("is-active");
          if (now && !was) { stopAll(st.active.anims || []); st.active.anims = activeAnim(el, st.active); }
          was = now;
        }).observe(w, { attributes: true, attributeFilter: ["class"] });
      }
      if (st.scroll) {
        var anims = scrollAnims(el, st.scroll);
        if (anims.length) scrolls.push({ el: el, anims: anims });
      }
      inView.observe(el);
    });
    if (scrolls.length) {
      addEventListener("scroll", onScroll, { passive: true });
      addEventListener("resize", function () { measured = false; onScroll(); });
      addEventListener("load", function () { measured = false; onScroll(); });
      update();
    }
  }

  // ------------------------------------------------------------------ aperçu (éditeur)
  // Joue une fois l'animation `value` du moment `family` sur `el`, puis remet l'élément tel quel.
  function preview(el, family, value) {
    if (!el || !el.animate) return;
    // Aperçu précédent encore en cours : arrêté (sans toucher aux animations du site).
    if (el.__rvPreview) { el.__rvPreview(); el.__rvPreview = null; }
    var o = parse(family, value);
    var html = null;
    var boxed = family !== "text" && ensureBox(el);
    var mine = [];
    var timers = [];
    var alive = true;
    var own = function (a) { return !(window.CSSAnimation && a instanceof CSSAnimation) && !(window.CSSTransition && a instanceof CSSTransition); };
    var later = function (fn, ms) { timers.push(setTimeout(fn, ms)); };
    var restore = function () {
      if (html !== null) { el.innerHTML = html; el.__rvPieces = null; el.__rvTrack = null; el.style.overflow = ""; }
      if (boxed) el.style.display = "";
      el.classList.remove("rv-in");
    };
    var done = function (anims, extra) {
      var ms = anims.reduce(function (m, a) {
        var t = a.effect.getComputedTiming();
        return Math.max(m, isFinite(t.endTime) ? t.endTime : 0);
      }, 0);
      mine = mine.concat(anims);
      later(function () { stopAll(anims); restore(); }, ms + (extra || 0));
    };
    if (family === "appear") { o.trigger = "load"; done(appear(el, o), 200); }
    else if (family === "active") { done(activeAnim(el, o), 100); }
    else if (family === "text") { html = el.innerHTML; o.trigger = "load"; done(text(el, o), 400); }
    else if (family === "hover" || family === "press") {
      var t = toggler(el, family, o);
      t(true);
      later(function () { t(false); }, o.dur * 1000 + 500);
      later(function () {
        el.getAnimations().filter(own).forEach(function (a) { a.cancel(); });
        if (o.preset === "underline") { ["backgroundImage", "backgroundRepeat", "backgroundPosition", "backgroundSize"].forEach(function (p) { el.style[p] = ""; }); }
        restore();
      }, o.dur * 2000 + 700);
    } else if (family === "loop") {
      if (o.preset === "ticker") html = el.innerHTML;
      var anims = loop(el, o);
      mine = mine.concat(anims);
      later(function () { stopAll(anims); restore(); }, o.preset === "ticker" ? 4000 : Math.max(1500, o.dur * 2000 + o.delay * 1000));
    } else if (family === "scroll") {
      var sa = scrollAnims(el, o);
      mine = mine.concat(sa);
      var t0 = performance.now();
      var step = function (now) {
        var p = Math.min(1, (now - t0) / 1600);
        sa.forEach(function (a) { a.currentTime = p * 999; });
        if (!alive) return;
        if (p < 1) requestAnimationFrame(step); else later(function () { stopAll(sa); }, 300);
      };
      requestAnimationFrame(step);
      later(restore, 2000);
    }
    el.__rvPreview = function () {
      alive = false;
      timers.forEach(clearTimeout);
      stopAll(mine);
      el.getAnimations().filter(own).forEach(function (a) { a.cancel(); });
      restore();
    };
  }

  window.RevelAnimations = { parse: parse, preview: preview, families: FAMILIES };
  if (manual || reduced || !window.IntersectionObserver || !Element.prototype.animate) {
    root.classList.remove("rv-anim");
    return;
  }
  if (doc.readyState === "loading") doc.addEventListener("DOMContentLoaded", start);
  else start();
})();
