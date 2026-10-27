/* Interactions entre éléments (6.3) : attribut `data-revel-on` de l'élément déclencheur,
   « déclencheur action élément » séparés par des points-virgules (voir revel-edit, actions.rs) ;
   onglets (`data-revel-tabs`) : un bouton `data-revel-tab` par panneau `data-revel-panel` ;
   carrousels et diaporamas (`data-revel-slider`), lecteurs vidéo (`data-revel-video`), filtres de
   liste (`data-revel-filters`), bandeau de consentement (`data-revel-consent`), sons et musique
   (actions `sound:` et `music:`) coupés par les interrupteurs `data-revel-switch` ; média au
   survol (`data-revel-hover-media`) : vidéo ou image qui montre le média de l'élément survolé.
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
    // `closest:.carte` : l'élément de la classe qui contient le déclencheur.
    if (t.indexOf("closest:") === 0) { var c = null; try { c = el.closest(t.slice(8)); } catch (_) { /* sélecteur invalide */ } return c ? [c] : []; }
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
        // Interrupteur clair / sombre : tenu par le script du thème.
        if (sws[i].getAttribute("data-revel-switch") === "theme") continue;
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
      // Média au survol (vidéo de fond pilotée par une liste) : arrêté ou relancé ; un autre
      // élément survolé pendant l'arrêt reste arrêté.
      var hms = el.hasAttribute("data-revel-hover-media") ? [el] : el.querySelectorAll("[data-revel-hover-media]");
      if (hms.length) {
        var halt = !el.classList.contains("is-paused");
        el.classList.toggle("is-paused", halt);
        for (var h = 0; h < hms.length; h++) {
          hms[h].__rvPaused = halt;
          if (hms[h].tagName !== "VIDEO") continue;
          if (halt) hms[h].pause();
          else if (hms[h].__rvLoad) hms[h].__rvLoad();
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
      // « Lire ou mettre en pause » un média au survol, avec la préférence « réduire les
      // animations » : en pause dès le chargement (le bouton propose de lire).
      if (act === "play" && still) {
        targets(src, key).forEach(function (t) {
          var hm = t.hasAttribute("data-revel-hover-media") ? [t] : t.querySelectorAll("[data-revel-hover-media]");
          if (!hm.length) return;
          t.classList.add("is-paused");
          for (var h = 0; h < hm.length; h++) hm[h].__rvPaused = true;
          state = true;
          src.setAttribute("aria-pressed", "true");
        });
      }
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
  // Média au survol (`data-revel-hover-media="<nom>"`, sur une vidéo ou une image) : montre le
  // média de l'élément survolé ou atteint au clavier parmi ceux `data-revel-media-for="<nom>"`
  // (adresse `data-revel-media`, image d'attente `data-revel-media-poster`), le premier au
  // chargement ; classe `is-active` sur l'élément montré, `is-ready` sur le média une fois lancé
  // (vidéo) ou chargé (image). Vidéo lancée seulement si elle a la lecture automatique, sans
  // préférence « réduire les animations » ; rien n'est chargé d'avance avec cette préférence.
  function hoverMedia(disp) {
    var name = disp.getAttribute("data-revel-hover-media");
    if (!name) return;
    var items = Array.prototype.filter.call(document.querySelectorAll("[data-revel-media-for]"), function (x) {
      return x.getAttribute("data-revel-media-for") === name;
    });
    if (!items.length) return;
    var isVideo = disp.tagName === "VIDEO";
    // Lecture automatique gérée ici (une vidéo arrêtée ne repart pas à chaque changement).
    var auto = isVideo && disp.hasAttribute("autoplay");
    if (auto) { disp.autoplay = false; disp.removeAttribute("autoplay"); }
    disp.addEventListener(isVideo ? "playing" : "load", function () { disp.classList.add("is-ready"); });
    var cur = -1;
    var run = function () {
      if (!auto || still || disp.__rvPaused) return;
      var p = disp.play();
      if (p && p.catch) p.catch(function () {});
    };
    var load = function (start) {
      var it = items[cur];
      var src = it ? (it.getAttribute("data-revel-media") || "").trim() : "";
      if (!src) return;
      if (start && isVideo && still) return;
      if (disp.getAttribute("src") === src) { if (isVideo && disp.paused) run(); return; }
      var poster = (it.getAttribute("data-revel-media-poster") || "").trim();
      disp.classList.remove("is-ready");
      if (isVideo) {
        if (poster) disp.setAttribute("poster", poster);
        disp.setAttribute("src", src);
        run();
      } else {
        disp.removeAttribute("srcset");
        disp.setAttribute("src", src);
      }
    };
    // Relancée par le bouton « Lire ou mettre en pause », même avec « réduire les animations ».
    disp.__rvLoad = function () {
      load(false);
      if (isVideo && disp.getAttribute("src")) { var p = disp.play(); if (p && p.catch) p.catch(function () {}); }
    };
    var show = function (i, start) {
      if (i === cur) return;
      cur = i;
      for (var j = 0; j < items.length; j++) items[j].classList.toggle("is-active", j === i);
      load(start);
    };
    items.forEach(function (it, i) {
      it.addEventListener("mouseenter", function () { show(i, false); });
      it.addEventListener("focusin", function () { show(i, false); });
    });
    var first = 0;
    for (var a = 0; a < items.length; a++) if (items[a].classList.contains("is-active")) { first = a; break; }
    show(first, true);
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
  // Image révélée au survol (`data-revel-reveal="spot radius=140 soft=40"`, D-125) : la dernière
  // image de l'élément se montre au survol, en entier (`fade`), jusqu'au pointeur (`wipe`, un volet
  // qui le suit) ou autour de lui (`spot`, un halo de `radius` px aux bords adoucis de `soft` px).
  // Au toucher, le doigt la déplace de même.
  function reveal(box) {
    var imgs = box.querySelectorAll("img");
    var top = imgs[imgs.length - 1];
    if (!top || imgs.length < 2) return;
    var parts = (box.getAttribute("data-revel-reveal") || "fade").split(/\s+/);
    var mode = parts[0] || "fade", radius = 140, soft = 40;
    parts.forEach(function (p) {
      if (p.indexOf("radius=") === 0) radius = parseFloat(p.slice(7)) || radius;
      if (p.indexOf("soft=") === 0) soft = Math.max(0, parseFloat(p.slice(5)) || 0);
    });
    var on = false;
    var set = function (x, y) {
      if (mode === "fade") {
        top.style.opacity = on ? "1" : "0";
      } else if (mode === "wipe") {
        top.style.transition = "none";
        top.style.opacity = "1";
        top.style.clipPath = "inset(0 " + (on ? 100 - x : 100) + "% 0 0)";
      } else {
        top.style.transition = "none";
        top.style.opacity = "1";
        var r = on ? radius : 0;
        var m = "radial-gradient(circle at " + x + "% " + y + "%, #000 " + Math.max(0, r - soft) + "px, transparent " + r + "px)";
        top.style.webkitMaskImage = m;
        top.style.maskImage = m;
      }
    };
    var at = function (e) {
      var b = box.getBoundingClientRect();
      set(Math.min(100, Math.max(0, (e.clientX - b.left) / b.width * 100)), Math.min(100, Math.max(0, (e.clientY - b.top) / b.height * 100)));
    };
    set(0, 0);
    box.addEventListener("pointerenter", function (e) { on = true; at(e); });
    box.addEventListener("pointermove", at);
    box.addEventListener("pointerleave", function () { on = false; set(0, 0); });
  }
  // Panier (D-129) : commande envoyée par un formulaire du site, sans service de paiement.
  // Boutons `data-revel-cart-add` (produit `data-revel-product`, prix `data-revel-price`),
  // compteurs `data-revel-cart-count`, panier `data-revel-cart="EUR"` (devise) : ligne modèle
  // `data-revel-cart-item` (nom `-name`, quantité `-qty`, prix `-price`, boutons `-plus`,
  // `-minus`, `-remove`), total `data-revel-cart-total`, message `data-revel-cart-empty`, champ
  // `data-revel-cart-field` (texte de la commande envoyé avec le formulaire). Gardé dans le
  // navigateur du visiteur ; vidé quand le formulaire est envoyé.
  var CART = "revel-cart";
  function cartLoad() {
    try { return JSON.parse(localStorage.getItem(CART)) || []; } catch (e) { return []; }
  }
  function cartSave(list) {
    try { localStorage.setItem(CART, JSON.stringify(list)); } catch (e) {}
    cartRender();
  }
  function cartPrice(v) {
    var n = parseFloat(String(v || "").replace(/\s/g, "").replace(/[^0-9,.-]/g, "").replace(",", "."));
    return isNaN(n) ? 0 : n;
  }
  function cartMoney(n, cur) {
    try {
      return new Intl.NumberFormat(document.documentElement.lang || undefined, { style: "currency", currency: cur || "EUR" }).format(n);
    } catch (e) { return n.toFixed(2) + " " + (cur || ""); }
  }
  function cartRender() {
    var list = cartLoad(), count = 0, total = 0;
    list.forEach(function (l) { count += l.qty; total += l.qty * l.price; });
    [].forEach.call(document.querySelectorAll("[data-revel-cart-count]"), function (e) {
      e.textContent = String(count);
      e.classList.toggle("is-empty", count === 0);
    });
    [].forEach.call(document.querySelectorAll("[data-revel-cart]"), function (box) {
      var cur = box.getAttribute("data-revel-cart") || "EUR";
      if (!box.__row) {
        var model = box.querySelector("[data-revel-cart-item]");
        if (!model) return;
        box.__row = model;
        box.__list = model.parentNode;
        model.parentNode.removeChild(model);
      }
      [].forEach.call(box.querySelectorAll("[data-revel-cart-line]"), function (e) { e.parentNode.removeChild(e); });
      list.forEach(function (l, i) {
        var row = box.__row.cloneNode(true);
        row.removeAttribute("data-revel-cart-item");
        row.setAttribute("data-revel-cart-line", String(i));
        var put = function (sel, t) { var e = row.querySelector(sel); if (e) e.textContent = t; };
        put("[data-revel-cart-name]", l.name);
        put("[data-revel-cart-qty]", String(l.qty));
        put("[data-revel-cart-price]", cartMoney(l.qty * l.price, cur));
        var on = function (sel, f) {
          var e = row.querySelector(sel);
          if (e) e.addEventListener("click", function () { var d = cartLoad(); if (d[i]) { f(d); cartSave(d); } });
        };
        on("[data-revel-cart-plus]", function (d) { d[i].qty++; });
        on("[data-revel-cart-minus]", function (d) { d[i].qty--; if (d[i].qty <= 0) d.splice(i, 1); });
        on("[data-revel-cart-remove]", function (d) { d.splice(i, 1); });
        box.__list.appendChild(row);
      });
      [].forEach.call(box.querySelectorAll("[data-revel-cart-total]"), function (e) { e.textContent = cartMoney(total, cur); });
      [].forEach.call(box.querySelectorAll("[data-revel-cart-empty]"), function (e) { e.hidden = list.length > 0; });
      box.classList.toggle("is-empty", list.length === 0);
      var text = list.map(function (l) { return l.qty + " × " + l.name + " — " + cartMoney(l.qty * l.price, cur); }).join("\n")
        + (/^fr/.test(document.documentElement.lang) ? "\nTotal : " : "\nTotal: ") + cartMoney(total, cur);
      [].forEach.call(box.querySelectorAll("[data-revel-cart-field]"), function (e) {
        e.value = list.length ? text : "";
        var form = e.form;
        if (form && !form.__cart) {
          form.__cart = true;
          // Envoi réussi (classe `is-sent` posée par le script des formulaires) : panier vidé.
          new MutationObserver(function () {
            if (form.classList.contains("is-sent")) { form.classList.remove("is-sent"); cartSave([]); }
          }).observe(form, { attributes: true, attributeFilter: ["class"] });
        }
      });
    });
  }
  function cart() {
    document.addEventListener("click", function (e) {
      var b = e.target.closest && e.target.closest("[data-revel-cart-add]");
      if (!b) return;
      e.preventDefault();
      var name = (b.getAttribute("data-revel-product") || b.textContent || "").trim();
      var list = cartLoad();
      var line = list.filter(function (l) { return l.name === name; })[0];
      if (line) line.qty++;
      else list.push({ name: name, price: cartPrice(b.getAttribute("data-revel-price")), qty: 1 });
      cartSave(list);
      b.classList.add("is-added");
      setTimeout(function () { b.classList.remove("is-added"); }, 1400);
    });
    cartRender();
    // Autre onglet du site : panier relu.
    window.addEventListener("storage", function (e) { if (e.key === CART) cartRender(); });
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
    var hm = document.querySelectorAll("[data-revel-hover-media]");
    for (var m = 0; m < hm.length; m++) hoverMedia(hm[m]);
    var rv = document.querySelectorAll("[data-revel-reveal]");
    for (var q = 0; q < rv.length; q++) reveal(rv[q]);
    switches();
    if (document.querySelector("[data-revel-cart],[data-revel-cart-add],[data-revel-cart-count]")) cart();
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
})();
