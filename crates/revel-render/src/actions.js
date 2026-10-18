/* Interactions entre éléments (6.3) : attribut `data-revel-on` de l'élément déclencheur,
   « déclencheur action élément » séparés par des points-virgules (voir revel-edit, actions.rs) ;
   onglets (`data-revel-tabs`) : un bouton `data-revel-tab` par panneau `data-revel-panel` ;
   carrousels et diaporamas (`data-revel-slider`), lecteurs vidéo (`data-revel-video`), filtres de
   liste (`data-revel-filters`), bandeau de consentement (`data-revel-consent`), sons et musique
   (actions `sound:` et `music:`) coupés par les interrupteurs `data-revel-switch`.
   Sans JavaScript, la page reste lisible : rien n'est masqué d'avance par le script. */
(function () {
  var css = document.createElement("style");
  css.textContent = ".rv-hide{display:none!important}.rv-video{display:block;width:100%;height:100%;border:0}[data-revel-consent]:not(.rv-open){display:none!important}";
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
  // Sons et musique : `sound:<adresse>|<adresse>~<volume %>` (une adresse au hasard). Les
  // interrupteurs `data-revel-switch="music"` ou `"sounds"` les coupent (choix gardé).
  var audio = { music: store.get("rv-switch:music") !== "off", sounds: store.get("rv-switch:sounds") !== "off" };
  var song = null, songWanted = null;
  function pickAudio(spec) {
    var vol = 100, list = spec;
    var t = spec.lastIndexOf("~");
    if (t > 0 && /^\d+$/.test(spec.slice(t + 1))) { vol = parseInt(spec.slice(t + 1), 10); list = spec.slice(0, t); }
    var urls = list.split("|").filter(Boolean);
    return { url: urls[Math.floor(Math.random() * urls.length)], vol: Math.min(100, vol) / 100 };
  }
  function playSound(spec) {
    if (!audio.sounds || !spec) return;
    var a = pickAudio(spec);
    if (!a.url) return;
    var s = new Audio(a.url);
    s.volume = a.vol;
    var p = s.play();
    if (p && p.catch) p.catch(function () {});
  }
  function playMusic(spec) {
    songWanted = spec;
    if (song) { song.pause(); song = null; }
    if (!audio.music) return;
    var a = pickAudio(spec);
    if (!a.url) return;
    song = new Audio(a.url);
    song.loop = true;
    song.volume = a.vol;
    var p = song.play();
    if (p && p.catch) p.catch(function () {});
  }
  function switches() {
    var sws = document.querySelectorAll("[data-revel-switch]");
    var paint = function () {
      for (var i = 0; i < sws.length; i++) {
        var on = audio[sws[i].getAttribute("data-revel-switch")] !== false;
        sws[i].setAttribute("aria-pressed", String(on));
        sws[i].classList.toggle("is-active", on);
        var txt = sws[i].querySelectorAll("[data-revel-switch-text]");
        for (var j = 0; j < txt.length; j++) {
          var parts = (txt[j].getAttribute("data-revel-switch-text") || "On|Off").split("|");
          txt[j].textContent = on ? parts[0] : (parts[1] || "");
        }
      }
    };
    for (var i = 0; i < sws.length; i++) {
      (function (b) {
        b.addEventListener("click", function () {
          var name = b.getAttribute("data-revel-switch");
          if (!(name in audio)) return;
          audio[name] = !audio[name];
          store.set("rv-switch:" + name, audio[name] ? "on" : "off");
          if (name === "music") {
            if (!audio.music && song) { song.pause(); song = null; }
            else if (audio.music && songWanted && !song) playMusic(songWanted);
          }
          paint();
        });
      })(sws[i]);
    }
    paint();
  }
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
      // Onglets : vidéo du panneau affiché arrêtée ou relancée (vidéo de fond d'un en-tête).
      if (el.hasAttribute("data-revel-tabs")) {
        var stop = !el.classList.contains("is-paused");
        el.classList.toggle("is-paused", stop);
        var vids = el.querySelectorAll("[data-revel-panel] video");
        for (var q = 0; q < vids.length; q++) {
          if (stop) vids[q].pause();
          else if (!vids[q].closest(".rv-hide")) { var pr = vids[q].play(); if (pr && pr.catch) pr.catch(function () {}); }
        }
        return;
      }
      var v = el.tagName === "VIDEO" ? el : el.querySelector("video");
      if (v) { if (on && v.paused) v.play(); else if (!on || !v.paused) v.pause(); }
    } else if (a.indexOf("random:") === 0) {
      // Un enfant au hasard (autre que l'actuel), affiché seul après le délai.
      var kids = Array.prototype.slice.call(el.children);
      if (!kids.length) return;
      var cur = kids.findIndex(function (k) { return !k.hidden && !k.classList.contains("rv-hide"); });
      var next = cur;
      while (kids.length > 1 && next === cur) next = Math.floor(Math.random() * kids.length);
      var reveal = function () {
        kids.forEach(function (k, i) { k.hidden = i !== next; k.classList.toggle("is-active", i === next); });
      };
      var wait = still ? 0 : parseInt(a.slice(7), 10) || 0;
      if (wait) setTimeout(reveal, wait); else reveal();
    } else if (a.indexOf("spin:") === 0) {
      // Rotation ajoutée, plus un quart de tour au hasard (variable --turn et transformation).
      el.__rvTurn = (el.__rvTurn || 0) + (parseInt(a.slice(5), 10) || 0) + Math.floor(Math.random() * 4) * 90;
      el.style.setProperty("--turn", el.__rvTurn + "deg");
    } else if (a.indexOf("sound:") === 0) {
      if (on) playSound(a.slice(6));
    } else if (a.indexOf("music:") === 0) {
      if (on) playMusic(a.slice(6));
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
        if (act === "play") src.setAttribute("aria-pressed", String(state));
      });
      // Menu ouvert à l'appui : refermé par Échap, le focus revient au bouton.
      if (act === "toggle" || act.indexOf("class:") === 0) {
        document.addEventListener("keydown", function (e) {
          if (e.key !== "Escape" || !state) return;
          state = false;
          if (act === "toggle") targets(src, key).forEach(function (t) { run("hide", t, true, key); });
          else each(false);
          src.setAttribute("aria-expanded", "false");
          if (src.focus) src.focus();
        });
      }
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
  // Onglets (`data-revel-tabs`, réglage `hover` : au survol et au focus, sinon à l'appui) : un
  // bouton ou lien `data-revel-tab` par panneau `data-revel-panel` ; flèches du clavier entre les
  // boutons ; les vidéos du panneau affiché sont lancées, les autres arrêtées.
  var still = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  function tabs(box) {
    var opts = String(box.getAttribute("data-revel-tabs") || "").split(/\s+/);
    var hover = opts.indexOf("hover") >= 0;
    // `keys` : flèches et W/A/S/D n'importe où dans la page ; `sound=<adresse>` : son à chaque
    // changement.
    var keys = opts.indexOf("keys") >= 0;
    var sound = "";
    opts.forEach(function (o) { if (o.indexOf("sound=") === 0) sound = o.slice(6); });
    var btns = box.querySelectorAll("[data-revel-tab]");
    var panels = box.querySelectorAll("[data-revel-panel]");
    // Plusieurs séries de panneaux (texte, numéro…) : chacune suit les onglets dans l'ordre.
    var nb = btns.length || 1;
    var cur = -1;
    var pick = function (i) {
      if (i === cur) return;
      if (cur >= 0 && sound) playSound(sound);
      cur = i;
      for (var j = 0; j < btns.length; j++) {
        btns[j].classList.toggle("is-active", j === i);
        if (btns[j].tagName !== "A") btns[j].setAttribute("aria-selected", String(j === i));
      }
      for (var k = 0; k < panels.length; k++) {
        var off = k % nb !== i;
        panels[k].classList.toggle("rv-hide", off);
        // Panneaux masqués d'avance (`hidden`, sans attendre le script) : affichés à leur tour.
        if (panels[k].hasAttribute("hidden") || off) panels[k].hidden = off;
        var vs = panels[k].tagName === "VIDEO" ? [panels[k]] : panels[k].querySelectorAll("video");
        for (var v = 0; v < vs.length; v++) {
          if (!off && vs[v].hasAttribute("autoplay") && !still && !box.classList.contains("is-paused")) {
            var p = vs[v].play();
            if (p && p.catch) p.catch(function () {});
          } else if (off) {
            vs[v].pause();
          }
        }
      }
    };
    for (var i = 0; i < btns.length; i++) {
      (function (n) {
        var b = btns[n];
        if (b.tagName !== "A") b.setAttribute("role", "tab");
        b.addEventListener("click", function () { pick(n); });
        if (hover) {
          b.addEventListener("mouseenter", function () { pick(n); });
          b.addEventListener("focus", function () { pick(n); });
        }
        b.addEventListener("keydown", function (e) {
          var d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
          if (!d || b.tagName === "A") return;
          e.preventDefault();
          var m = (n + d + btns.length) % btns.length;
          btns[m].focus();
          pick(m);
        });
      })(i);
    }
    if (keys && btns.length) {
      document.addEventListener("keydown", function (e) {
        var t = e.target, tag = t && t.tagName;
        if (e.altKey || e.ctrlKey || e.metaKey || tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || (t && t.isContentEditable)) return;
        if (!box.offsetParent && getComputedStyle(box).position !== "fixed") return;
        var k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
        var d = k === "ArrowDown" || k === "ArrowRight" || k === "s" || k === "d" ? 1
          : k === "ArrowUp" || k === "ArrowLeft" || k === "w" || k === "a" ? -1 : 0;
        if (!d) return;
        e.preventDefault();
        pick((Math.max(cur, 0) + d + btns.length) % btns.length);
      });
    }
    var start = 0;
    for (var a = 0; a < btns.length; a++) if (btns[a].classList.contains("is-active")) { start = a; break; }
    if (btns.length) pick(start);
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
  // Lecteur vidéo (`data-revel-video="<adresse>"`) : l'appui sur `data-revel-play` (couverture)
  // met la vidéo à sa place, lancée : YouTube (sans cookies), Vimeo (sans suivi) ou fichier.
  function player(url, label) {
    // Fichier vidéo (lien direct, « progressive_redirect » de Vimeo) : lu tel quel.
    var file = /\.(mp4|webm|ogv|ogg|mov|m4v)(\?|#|$)/i.test(url) || url.indexOf("progressive_redirect") >= 0;
    var yt = file ? null : url.match(/(?:youtu\.be\/|youtube(?:-nocookie)?\.com\/(?:watch\?(?:.*&)?v=|embed\/|shorts\/|live\/))([\w-]{11})/);
    var vm = file ? null : url.match(/vimeo\.com\/(?:.*\/)?(\d+)/);
    var el;
    if (yt || vm) {
      el = document.createElement("iframe");
      el.src = yt
        ? "https://www.youtube-nocookie.com/embed/" + yt[1] + "?autoplay=1&rel=0"
        : "https://player.vimeo.com/video/" + vm[1] + "?autoplay=1&dnt=1";
      el.setAttribute("allow", "autoplay; fullscreen; picture-in-picture");
      el.setAttribute("allowfullscreen", "");
      el.title = label;
    } else {
      el = document.createElement("video");
      el.src = url;
      el.controls = true;
      el.autoplay = true;
      el.playsInline = true;
    }
    el.className = "rv-video";
    return el;
  }
  function video(box) {
    var btn = box.querySelector("[data-revel-play]");
    var url = (box.getAttribute("data-revel-video") || "").trim();
    if (!btn || !url) return;
    btn.addEventListener("click", function () {
      var el = player(url, btn.getAttribute("aria-label") || "Video");
      btn.parentNode.replaceChild(el, btn);
      box.classList.add("is-playing");
      if (el.focus) el.focus();
    });
  }
  // Filtres de liste (`data-revel-filters="<paramètre>"`) : boutons `data-revel-filter="<valeur>"`
  // (vide : tout) ; éléments filtrés `data-revel-tags="a b"` de la page ; message
  // `data-revel-filter-empty` sans résultat ; valeur gardée dans l'adresse (`?<paramètre>=`).
  function filters(group) {
    var param = (group.getAttribute("data-revel-filters") || "").trim();
    var buttons = group.querySelectorAll("[data-revel-filter]");
    var apply = function (value, push) {
      var items = document.querySelectorAll("[data-revel-tags]");
      var visible = 0;
      for (var i = 0; i < items.length; i++) {
        var ok = !value || (" " + items[i].getAttribute("data-revel-tags") + " ").indexOf(" " + value + " ") >= 0;
        items[i].classList.toggle("rv-hide", !ok);
        if (ok) visible++;
      }
      for (var b = 0; b < buttons.length; b++) {
        var on = (buttons[b].getAttribute("data-revel-filter") || "") === value;
        buttons[b].classList.toggle("is-active", on);
        buttons[b].setAttribute("aria-pressed", String(on));
      }
      var empty = document.querySelectorAll("[data-revel-filter-empty]");
      for (var e = 0; e < empty.length; e++) {
        empty[e].classList.toggle("rv-hide", visible > 0);
        empty[e].hidden = visible > 0;
      }
      if (push && param && window.URL && history.replaceState) {
        var url = new URL(location.href);
        if (value) url.searchParams.set(param, value); else url.searchParams.delete(param);
        history.replaceState(null, "", url);
      }
    };
    for (var i = 0; i < buttons.length; i++) {
      (function (btn) {
        btn.addEventListener("click", function () { apply(btn.getAttribute("data-revel-filter") || "", true); });
      })(buttons[i]);
    }
    var empty0 = document.querySelectorAll("[data-revel-filter-empty]");
    for (var k = 0; k < empty0.length; k++) empty0[k].classList.add("rv-hide");
    var initial = param && window.URL ? new URL(location.href).searchParams.get(param) : null;
    if (initial) apply(initial, false);
  }
  // Consentement (`data-revel-consent="G-…"`, identifiant Google Analytics 4 facultatif) : le
  // bandeau s'affiche tant que le visiteur n'a pas choisi ; `data-revel-consent-choice="accept"`
  // ou `"refuse"` ; `data-revel-consent-open` le rouvre. Mesure chargée après l'accord seulement.
  var KEY = "revel-consent";
  function analytics(id) {
    if (!id || window.__rvGa) return;
    window.__rvGa = true;
    var sc = document.createElement("script");
    sc.async = true;
    sc.src = "https://www.googletagmanager.com/gtag/js?id=" + encodeURIComponent(id);
    document.head.appendChild(sc);
    window.dataLayer = window.dataLayer || [];
    window.gtag = function () { window.dataLayer.push(arguments); };
    window.gtag("js", new Date());
    window.gtag("config", id, { anonymize_ip: true });
  }
  function consent(box) {
    var id = (box.getAttribute("data-revel-consent") || "").trim();
    var choice = store.get(KEY);
    if (choice === "accept") analytics(id);
    box.classList.toggle("rv-open", !choice);
    var btns = box.querySelectorAll("[data-revel-consent-choice]");
    for (var i = 0; i < btns.length; i++) {
      (function (b) {
        b.addEventListener("click", function () {
          var v = b.getAttribute("data-revel-consent-choice");
          store.set(KEY, v);
          box.classList.remove("rv-open");
          if (v === "accept") { analytics(id); return; }
          // Refus : cookies de mesure retirés (accord précédent).
          document.cookie.split(";").map(function (c) { return c.trim().split("=")[0]; })
            .filter(function (n) { return /^_ga/.test(n); })
            .forEach(function (n) {
              var host = location.hostname.replace(/^www\./, "");
              document.cookie = n + "=; Max-Age=0; path=/; domain=." + host;
              document.cookie = n + "=; Max-Age=0; path=/";
            });
          if (window.__rvGa) location.reload();
        });
      })(btns[i]);
    }
    var opens = document.querySelectorAll("[data-revel-consent-open]");
    for (var j = 0; j < opens.length; j++) {
      opens[j].addEventListener("click", function (e) {
        e.preventDefault();
        box.classList.add("rv-open");
        var f = box.querySelector("button, a");
        if (f) f.focus();
      });
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
    var cs = document.querySelectorAll("[data-revel-consent]");
    for (var c = 0; c < cs.length; c++) consent(cs[c]);
    var fs = document.querySelectorAll("[data-revel-filters]");
    for (var f = 0; f < fs.length; f++) filters(fs[f]);
    var vs = document.querySelectorAll("[data-revel-video]");
    for (var v = 0; v < vs.length; v++) video(vs[v]);
    switches();
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
})();
