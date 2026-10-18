/* Interactions entre éléments (6.3) : attribut `data-revel-on` de l'élément déclencheur,
   « déclencheur action élément » séparés par des points-virgules (voir revel-edit, actions.rs) ;
   onglets (`data-revel-tabs`) : un bouton `data-revel-tab` par panneau `data-revel-panel` ;
   carrousels et diaporamas (`data-revel-slider`).
   Sans JavaScript, la page reste lisible : rien n'est masqué d'avance par le script. */
(function () {
  var css = document.createElement("style");
  css.textContent = ".rv-hide{display:none!important}";
  document.head.appendChild(css);
  var store = {
    get: function (k) { try { return localStorage.getItem(k); } catch (_) { return null; } },
    set: function (k, v) { try { localStorage.setItem(k, v); } catch (_) { /* navigation privée */ } }
  };
  function targets(el, t) {
    if (t === "self") return [el];
    if (t === "parent") return el.parentElement ? [el.parentElement] : [];
    if (t === "next") return el.nextElementSibling ? [el.nextElementSibling] : [];
    if (t === "body") return [document.body];
    try { return Array.prototype.slice.call(document.querySelectorAll(t)); } catch (_) { return []; }
  }
  function shown(el) { return getComputedStyle(el).display !== "none"; }
  function show(el, on) {
    if (on) {
      el.classList.remove("rv-hide");
      if (!shown(el)) el.style.display = el.getAttribute("data-revel-display") || "block";
    } else {
      el.style.display = "";
      if (shown(el)) el.classList.add("rv-hide");
    }
  }
  // Action appliquée (`on`) ou défaite.
  function run(a, el, on, key) {
    // Bascule : inversée à chaque appel (survol : à l'entrée puis à la sortie).
    if (a === "toggle") show(el, !shown(el));
    else if (a === "show") show(el, on);
    else if (a === "hide") show(el, !on);
    else if (a === "play") {
      // Bandeau défilant (« Défilement ») : arrêté ou relancé.
      var t = el.__rvTrack;
      if (t) {
        t.stopped = on ? !t.stopped : true;
        t.anims.forEach(function (x) { if (t.stopped) x.pause(); else x.play(); });
        el.classList.toggle("is-paused", t.stopped);
        return;
      }
      var v = el.tagName === "VIDEO" ? el : el.querySelector("video");
      if (v) { if (on && v.paused) v.play(); else if (!on || !v.paused) v.pause(); }
    } else if (a === "dismiss") {
      if (on) { el.classList.add("rv-hide"); store.set("rv-dismiss:" + key, "1"); }
    } else if (a.indexOf("class:") === 0) {
      el.classList.toggle(a.slice(6), on);
    }
  }
  function bind(src, rule) {
    var p = rule.split(/\s+/);
    if (p.length !== 3) return;
    var trig = p[0], act = p[1], key = p[2];
    var each = function (on) { targets(src, key).forEach(function (t) { run(act, t, on, key); }); };
    if (act === "dismiss" && store.get("rv-dismiss:" + key)) {
      targets(src, key).forEach(function (t) { t.classList.add("rv-hide"); });
    }
    if (trig === "click") {
      var state = false;
      src.addEventListener("click", function (e) {
        if (src.tagName === "A" && (src.getAttribute("href") || "#") === "#") e.preventDefault();
        // Classe : ajoutée puis retirée à l'appui suivant ; autres actions : à chaque appui.
        state = !state;
        each(act.indexOf("class:") === 0 ? state : true);
        if (act === "toggle" || act.indexOf("class:") === 0) src.setAttribute("aria-expanded", String(state));
      });
    } else if (trig === "hover") {
      src.addEventListener("mouseenter", function () { each(true); });
      src.addEventListener("mouseleave", function () { each(false); });
    } else if (trig.indexOf("scroll:") === 0) {
      var px = parseInt(trig.slice(7), 10) || 0, cur = null;
      var check = function () {
        var on = (window.scrollY || window.pageYOffset) > px;
        if (on !== cur) { cur = on; each(on); }
      };
      window.addEventListener("scroll", check, { passive: true });
      check();
    } else if (trig === "view") {
      if ("IntersectionObserver" in window) {
        new IntersectionObserver(function (es) { es.forEach(function (e) { each(e.isIntersecting); }); }, { threshold: 0.2 }).observe(src);
      } else each(true);
    } else if (trig === "load") {
      each(true);
    }
  }
  function tabs(box) {
    var btns = box.querySelectorAll("[data-revel-tab]");
    var panels = box.querySelectorAll("[data-revel-panel]");
    var pick = function (i) {
      for (var j = 0; j < btns.length; j++) {
        btns[j].classList.toggle("is-active", j === i);
        btns[j].setAttribute("aria-selected", String(j === i));
      }
      for (var k = 0; k < panels.length; k++) panels[k].classList.toggle("rv-hide", k !== i);
    };
    for (var i = 0; i < btns.length; i++) {
      (function (n) {
        btns[n].setAttribute("role", "tab");
        btns[n].addEventListener("click", function () { pick(n); });
      })(i);
    }
    if (btns.length) pick(0);
  }
  // Carrousel et diaporama (`data-revel-slider`) : piste `data-revel-track` défilante (accroche
  // par diapositive), boutons `data-revel-prev` / `data-revel-next`, compteur `data-revel-count`,
  // points `data-revel-dots` (créés ici) ; réglages `autoplay=<s>` et `loop`.
  function slider(box) {
    var track = box.querySelector("[data-revel-track]");
    if (!track) return;
    var opts = {};
    String(box.getAttribute("data-revel-slider") || "").split(/\s+/).forEach(function (p) {
      if (!p) return;
      var i = p.indexOf("=");
      opts[i < 0 ? p : p.slice(0, i)] = i < 0 ? true : p.slice(i + 1);
    });
    var slides = function () { return Array.prototype.slice.call(track.children); };
    var reduce = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    var current = function () {
      var s = slides(), left = track.scrollLeft, best = 0, d = Infinity;
      for (var i = 0; i < s.length; i++) {
        var x = Math.abs(s[i].offsetLeft - s[0].offsetLeft - left);
        if (x < d) { d = x; best = i; }
      }
      // Fin de piste atteinte : dernière diapositive.
      if (left + track.clientWidth >= track.scrollWidth - 2) best = s.length - 1;
      return best;
    };
    var go = function (i) {
      var s = slides();
      if (!s.length) return;
      if (opts.loop !== undefined) i = (i + s.length) % s.length;
      i = Math.max(0, Math.min(s.length - 1, i));
      track.scrollTo({ left: s[i].offsetLeft - s[0].offsetLeft, behavior: reduce ? "auto" : "smooth" });
    };
    var count = box.querySelector("[data-revel-count]");
    var dots = box.querySelector("[data-revel-dots]");
    var prev = box.querySelector("[data-revel-prev]");
    var next = box.querySelector("[data-revel-next]");
    if (dots) {
      dots.innerHTML = "";
      slides().forEach(function (_, i) {
        var b = document.createElement("button");
        b.type = "button";
        b.setAttribute("aria-label", String(i + 1));
        b.addEventListener("click", function () { go(i); });
        dots.appendChild(b);
      });
    }
    var update = function () {
      var i = current(), n = slides().length;
      if (count) count.textContent = (i + 1) + " / " + n;
      if (dots) Array.prototype.forEach.call(dots.children, function (b, j) {
        b.classList.toggle("is-active", j === i);
        b.setAttribute("aria-current", String(j === i));
      });
      if (opts.loop === undefined) {
        if (prev) prev.disabled = i === 0;
        if (next) next.disabled = i === n - 1;
      }
    };
    if (prev) prev.addEventListener("click", function () { go(current() - 1); });
    if (next) next.addEventListener("click", function () { go(current() + 1); });
    track.addEventListener("scroll", update, { passive: true });
    track.addEventListener("keydown", function (e) {
      if (e.key === "ArrowRight") { e.preventDefault(); go(current() + 1); }
      if (e.key === "ArrowLeft") { e.preventDefault(); go(current() - 1); }
    });
    if (!track.hasAttribute("tabindex")) track.setAttribute("tabindex", "0");
    update();
    var secs = Number(opts.autoplay) || 0;
    if (secs > 0 && !reduce) {
      var held = false;
      box.addEventListener("pointerenter", function () { held = true; });
      box.addEventListener("pointerleave", function () { held = false; });
      box.addEventListener("focusin", function () { held = true; });
      box.addEventListener("focusout", function () { held = false; });
      setInterval(function () {
        if (held || document.hidden) return;
        var i = current(), n = slides().length;
        go(i >= n - 1 ? 0 : i + 1);
      }, secs * 1000);
    }
  }
  function all() {
    var els = document.querySelectorAll("[data-revel-on]");
    for (var i = 0; i < els.length; i++) {
      var rules = els[i].getAttribute("data-revel-on").split(";");
      for (var r = 0; r < rules.length; r++) if (rules[r].trim()) bind(els[i], rules[r].trim());
    }
    var t = document.querySelectorAll("[data-revel-tabs]");
    for (var j = 0; j < t.length; j++) tabs(t[j]);
    var sl = document.querySelectorAll("[data-revel-slider]");
    for (var k = 0; k < sl.length; k++) slider(sl[k]);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
})();
