/* Interactions entre éléments (6.3) : attribut `data-revel-on` de l'élément déclencheur,
   « déclencheur action élément » séparés par des points-virgules (voir revel-edit, actions.rs) ;
   onglets (`data-revel-tabs`) : un bouton `data-revel-tab` par panneau `data-revel-panel`.
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
  function all() {
    var els = document.querySelectorAll("[data-revel-on]");
    for (var i = 0; i < els.length; i++) {
      var rules = els[i].getAttribute("data-revel-on").split(";");
      for (var r = 0; r < rules.length; r++) if (rules[r].trim()) bind(els[i], rules[r].trim());
    }
    var t = document.querySelectorAll("[data-revel-tabs]");
    for (var j = 0; j < t.length; j++) tabs(t[j]);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
})();
