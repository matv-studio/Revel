/* Scènes Marmoset du site (D-160) : chaque élément `data-mview="/media/x.mview"` (fichier exporté
   de Marmoset Toolbag) est lu par le lecteur officiel de Marmoset, chargé depuis
   viewer.marmoset.co au premier élément qui approche de l'écran. Réglages portés par l'élément :
   - data-mview-autostart : scène chargée sans attendre l'appui sur le bouton de lecture ;
   - data-mview-transparent : fond transparent (couleur de l'élément derrière) ;
   - data-mview-noui : commandes du lecteur masquées ;
   - data-mview-thumbnail : image d'attente à la place de celle du fichier.
   Le lecteur suit la taille de l'élément (sauf en plein écran). */
(function () {
  if (window.RevelMview) return;
  var SRC = "https://viewer.marmoset.co/main/marmoset.js";
  var waiting = [];
  function lib(cb) {
    if (window.marmoset && window.marmoset.WebViewer) { cb(); return; }
    waiting.push(cb);
    if (waiting.length > 1) return;
    var s = document.createElement("script");
    s.src = SRC;
    s.async = true;
    s.onload = function () { var w = waiting; waiting = []; w.forEach(function (f) { f(); }); };
    s.onerror = function () { var w = waiting; waiting = []; w.forEach(function (f) { f(new Error("marmoset")); }); };
    document.head.appendChild(s);
  }
  function mount(el) {
    var src = el.getAttribute("data-mview");
    if (!src) { el.setAttribute("data-mview-state", "empty"); return; }
    el.setAttribute("data-mview-state", "loading");
    var st = { key: key(el) };
    el.__rvMview = st;
    lib(function (err) {
      if (err || !el.isConnected || el.__rvMview !== st) { if (err) el.setAttribute("data-mview-state", "error"); return; }
      var M = window.marmoset;
      // Réglages globaux du lecteur, lus à la création de chaque scène.
      M.transparentBackground = el.hasAttribute("data-mview-transparent");
      M.noUserInterface = el.hasAttribute("data-mview-noui");
      var w = Math.max(1, el.clientWidth);
      var h = Math.max(1, el.clientHeight);
      var v;
      try {
        v = new M.WebViewer(w, h, src);
      } catch (e) {
        el.setAttribute("data-mview-state", "error");
        return;
      }
      var thumb = el.getAttribute("data-mview-thumbnail");
      if (thumb) v.thumbnailURL = thumb;
      var root = v.domRoot;
      root.setAttribute("data-rv-live", "");
      root.style.position = "absolute";
      root.style.inset = "0";
      el.insertBefore(root, el.firstChild);
      st.viewer = v;
      v.onLoad = function () { el.setAttribute("data-mview-state", "ready"); };
      el.setAttribute("data-mview-state", "ready");
      if (el.hasAttribute("data-mview-autostart")) v.loadScene();
      st.ro = new ResizeObserver(function () {
        if (document.fullscreenElement) return;
        var nw = el.clientWidth;
        var nh = el.clientHeight;
        if (nw && nh && (nw !== st.w || nh !== st.h)) { st.w = nw; st.h = nh; try { v.resize(nw, nh); } catch (_) { /* en cours */ } }
      });
      st.ro.observe(el);
    });
  }
  function key(el) {
    return ["data-mview", "data-mview-autostart", "data-mview-transparent", "data-mview-noui", "data-mview-thumbnail"]
      .map(function (n) { return n + "=" + el.getAttribute(n); }).join(" ");
  }
  function destroy(el) {
    var st = el.__rvMview;
    if (!st) return;
    if (st.ro) st.ro.disconnect();
    if (st.viewer) { try { st.viewer.unload(); } catch (_) { /* déjà fermé */ } }
    var ns = el.querySelectorAll(":scope > [data-rv-live]");
    for (var i = 0; i < ns.length; i++) ns[i].remove();
    el.__rvMview = null;
    el.removeAttribute("data-mview-state");
  }
  var io = "IntersectionObserver" in window ? new IntersectionObserver(function (es) {
    es.forEach(function (e) {
      if (e.isIntersecting && !e.target.__rvMview) mount(e.target);
    });
  }, { rootMargin: "200px" }) : null;
  function all() {
    var els = document.querySelectorAll("[data-mview]");
    for (var i = 0; i < els.length; i++) {
      var el = els[i];
      // Réglages changés (éditeur) : lecteur refait.
      if (el.__rvMview && el.__rvMview.key !== key(el)) destroy(el);
      if (el.__rvMviewSeen && !el.__rvMview && io) { io.unobserve(el); el.__rvMviewSeen = false; }
      if (el.__rvMviewSeen) continue;
      el.__rvMviewSeen = true;
      if (io) io.observe(el); else mount(el);
    }
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
  window.RevelMview = { refresh: all, destroy: destroy };
})();
