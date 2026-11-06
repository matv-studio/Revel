/* Modèles 3D du site (D-160) : chaque élément `data-model="/media/x.glb"` (GLB ou glTF) est
   affiché par three.js (licence MIT, hébergé avec le site, aucun service extérieur), chargé quand
   l'élément approche de l'écran. Réglages portés par l'élément :
   - data-model-poster : image montrée jusqu'au premier rendu (et si WebGL manque) ;
   - data-model-yaw, data-model-pitch : angle horizontal et hauteur de la caméra (degrés) ;
   - data-model-zoom : 100 = modèle cadré entier, plus grand = plus près ;
   - data-model-target : point visé, « x y z » en % du rayon du modèle depuis son centre ;
   - data-model-fov : champ de vision vertical (degrés, 30 par défaut) ;
   - data-model-camera : caméra du fichier (nom) à la place de la caméra réglée ;
   - data-model-controls="none" : aucune interaction ; data-model-pan : déplacement permis ;
   - data-model-zoom-limits « min max » (%), data-model-pitch-limits « min max » (degrés) ;
   - data-model-rotate : rotation automatique (degrés par seconde, négatif : autre sens) ;
   - data-model-env : studio (défaut), none, ou fichier .hdr ; data-model-exposure ;
   - data-model-backdrop : environnement visible derrière le modèle (flou de 0 à 1) ;
   - data-model-shadow : ombre au sol (0 à 1) ;
   - data-model-animation : animation du fichier (nom) ; data-model-loop ; data-model-speed ;
   - data-model-view : render, albedo, normals, roughness, metalness, wireframe, ou layers (calques :
     toutes les vues côte à côte, en bandes obliques nommées, comme le lecteur de Marmoset) ;
   - data-model-stats : nombre de triangles et de sommets affiché en bas à gauche.
   Boutons contenus (`data-model-do`) : fullscreen, reset, rotate, play, view:<vue>.
   Cadrage : sphère englobante, le plus étroit des deux angles de vue décide (jamais coupé, même en
   portrait) ; recalculé à chaque changement de taille. Gestes : glissement vertical d'un doigt
   laissé au défilement, molette active seulement après un appui dans le modèle (ou avec Ctrl).
   Calcul seulement quand l'image change ; pause hors de l'écran ; finesse plafonnée à 2. */
(function () {
  if (window.RevelModel) return;
  var me = document.currentScript && document.currentScript.src;
  var base = me ? me.replace(/revel-model\.js(\?.*)?$/, "") : "/assets/";
  var LIB = base + "revel-3d/";
  var reduce = !!(window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  var RT = null;
  var waiting = [];
  var shared = {};
  var live = [];
  var MAX_LIVE = 8;
  function engine(cb) {
    if (window.RevelThree) { RT = window.RevelThree; cb(); return; }
    waiting.push(cb);
    if (waiting.length > 1) return;
    var s = document.createElement("script");
    s.src = LIB + "three.js";
    s.async = true;
    s.onload = function () {
      RT = window.RevelThree;
      var w = waiting; waiting = [];
      w.forEach(function (f) { try { f(); } catch (e) { console.error(e); } });
    };
    s.onerror = function () {
      var w = waiting; waiting = [];
      w.forEach(function (f) { try { f(new Error("three")); } catch (_) { /* rien */ } });
    };
    document.head.appendChild(s);
  }
  function num(el, name, dflt) {
    var v = parseFloat(el.getAttribute(name));
    return isFinite(v) ? v : dflt;
  }
  function pair(el, name, a, b) {
    var p = (el.getAttribute(name) || "").trim().split(/[\s,]+/).map(parseFloat);
    return [isFinite(p[0]) ? p[0] : a, isFinite(p[1]) ? p[1] : b];
  }
  function triple(el, name) {
    var p = (el.getAttribute(name) || "").trim().split(/[\s,]+/).map(parseFloat);
    return [p[0] || 0, p[1] || 0, p[2] || 0];
  }
  function editor() { return !!window.__revelCanvas; }
  function loaders(renderer) {
    var T = RT;
    if (!shared.draco) {
      shared.draco = new T.DRACOLoader();
      shared.draco.setDecoderPath(LIB);
    }
    if (!shared.ktx2) {
      shared.ktx2 = new T.KTX2Loader();
      shared.ktx2.setTranscoderPath(LIB);
      shared.ktx2.detectSupport(renderer);
    }
    var g = new T.GLTFLoader();
    g.setDRACOLoader(shared.draco);
    g.setKTX2Loader(shared.ktx2);
    g.setMeshoptDecoder(T.MeshoptDecoder);
    return g;
  }
  // Nœuds ajoutés par le lecteur (gardés par le canevas de l'éditeur).
  function liveNode(el, tag) {
    var n = document.createElement(tag);
    n.setAttribute("data-rv-live", "");
    el.insertBefore(n, el.firstChild);
    return n;
  }

  // Noms des vues et du compteur, dans la langue de la page (textes de l'interface de Revel).
  var WORDS = /*WORDS*/{}/*/WORDS*/;
  function word(el, key) {
    var l = ((el.closest && el.closest("[lang]") || document.documentElement).getAttribute("lang") || "en").toLowerCase().slice(0, 2);
    var t = WORDS[l] || WORDS.en || {};
    return t[key] || (WORDS.en && WORDS.en[key]) || key;
  }

  function Viewer(el) {
    this.el = el;
    this.raf = 0;
    this.visible = true;
    this.active = false;
    this.fit = 1;
    this.src = null;
    this.envKey = null;
    this.view = "render";
    this.alt = new Map();
    this.idle = 0;
    this.last = 0;
    this.offs = [];
    this.setup();
  }
  var V = Viewer.prototype;
  V.setup = function () {
    var el = this.el;
    this.posterSync();
    el.setAttribute("data-model-state", el.getAttribute("data-model") ? "loading" : "empty");
    this.boot();
  };
  // Moteur chargé puis lecteur démarré, dès qu'un fichier est donné.
  V.boot = function () {
    var el = this.el;
    var self = this;
    if (this.booting || this.renderer || !el.getAttribute("data-model")) return;
    this.booting = true;
    el.setAttribute("data-model-state", "loading");
    engine(function (err) {
      self.booting = false;
      if (err || self.dead) { el.setAttribute("data-model-state", "error"); return; }
      try { self.start(); } catch (e) { console.error(e); el.setAttribute("data-model-state", "error"); }
    });
  };
  // Image d'attente : suit `data-model-poster`.
  V.posterSync = function () {
    var el = this.el;
    var poster = el.getAttribute("data-model-poster");
    if (!poster) {
      if (this.poster) { this.poster.remove(); this.poster = null; }
      return;
    }
    if (!this.poster) {
      this.poster = liveNode(el, "img");
      this.poster.alt = "";
      this.poster.setAttribute("aria-hidden", "true");
      this.poster.style.cssText = "position:absolute;inset:0;width:100%;height:100%;object-fit:contain;pointer-events:none;transition:opacity .4s";
    }
    if (this.poster.getAttribute("src") !== poster) this.poster.setAttribute("src", poster);
    this.poster.style.opacity = this.model ? "0" : "";
  };
  V.start = function () {
    var T = RT.THREE;
    var el = this.el;
    var self = this;
    var canvas = liveNode(el, "canvas");
    canvas.style.cssText = "position:absolute;inset:0;width:100%;height:100%;display:block;outline:none";
    this.canvas = canvas;
    var renderer;
    try {
      renderer = new T.WebGLRenderer({ canvas: canvas, antialias: true, alpha: true, stencil: true, powerPreference: "high-performance" });
    } catch (e) {
      canvas.remove();
      el.setAttribute("data-model-state", "error");
      return;
    }
    this.renderer = renderer;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.outputColorSpace = T.SRGBColorSpace;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = T.VSMShadowMap;
    renderer.setClearColor(0x000000, 0);
    this.scene = new T.Scene();
    this.camera = new T.PerspectiveCamera(30, 1, 0.01, 1000);
    this.pmrem = new T.PMREMGenerator(renderer);
    var c = new RT.OrbitControls(this.camera, canvas);
    this.controls = c;
    c.enableDamping = true;
    c.dampingFactor = 0.1;
    c.screenSpacePanning = true;
    c.touches = { ONE: T.TOUCH.ROTATE, TWO: T.TOUCH.DOLLY_PAN };
    // Un doigt vers le haut ou le bas fait défiler la page (le navigateur garde ce geste).
    canvas.style.touchAction = "pan-y";
    c.addEventListener("change", function () { self.draw(); });
    c.addEventListener("start", function () { self.dragging = true; self.touched = true; });
    c.addEventListener("end", function () { self.dragging = false; self.idle = performance.now(); self.draw(); });
    var on = function (target, type, fn, opts) {
      target.addEventListener(type, fn, opts);
      self.offs.push(function () { target.removeEventListener(type, fn, opts); });
    };
    // Molette : sans effet tant qu'on n'a pas appuyé dans le modèle (ou avec Ctrl).
    on(el, "wheel", function (e) {
      c.enableZoom = !!(self.zoomable && (self.active || e.ctrlKey || self.orbiting));
    }, { capture: true, passive: true });
    on(el, "pointerdown", function () {
      self.active = true;
      c.enableZoom = !!self.zoomable;
    }, true);
    on(document, "pointerdown", function (e) { if (!el.contains(e.target)) self.active = false; }, true);
    on(el, "click", function (e) {
      var b = e.target.closest && e.target.closest("[data-model-do]");
      if (b && el.contains(b) && !editor()) { e.preventDefault(); self.command(b.getAttribute("data-model-do")); }
    });
    on(document, "fullscreenchange", function () { self.pressed(); setTimeout(function () { self.resize(); }, 50); });
    canvas.addEventListener("webglcontextlost", function (e) { e.preventDefault(); });
    canvas.addEventListener("webglcontextrestored", function () { self.restart(); });
    this.ro = new ResizeObserver(function () { self.resize(); });
    this.ro.observe(el);
    this.key = new T.DirectionalLight(0xffffff, 0);
    this.key.castShadow = true;
    // Ombre douce juste sous le modèle (lumière presque au-dessus, flou large).
    this.key.shadow.mapSize.set(512, 512);
    this.key.shadow.radius = 12;
    this.key.shadow.blurSamples = 16;
    this.key.shadow.bias = -0.0005;
    this.scene.add(this.key);
    this.scene.add(this.key.target);
    this.hemi = new T.HemisphereLight(0xffffff, 0x444444, 0);
    this.scene.add(this.hemi);
    this.ground = new T.Mesh(new T.PlaneGeometry(1, 1), new T.ShadowMaterial({ opacity: 0 }));
    this.ground.rotation.x = -Math.PI / 2;
    this.ground.receiveShadow = true;
    this.ground.visible = false;
    this.scene.add(this.ground);
    // Plein écran impossible (iPhone, cadre sans permission) : bouton masqué ; montré dans l'éditeur.
    if (!shared.fs && !document.fullscreenEnabled && !editor()) shared.fs = "no";
    this.update();
  };
  V.restart = function () {
    this.teardown();
    this.dead = false;
    this.booting = false;
    this.setup();
  };
  // Réglages relus (première fois, puis à chaque changement dans l'éditeur).
  V.update = function () {
    this.posterSync();
    if (!this.renderer) { this.boot(); return; }
    var el = this.el;
    var T = RT.THREE;
    var src = el.getAttribute("data-model");
    if (src !== this.src) { this.load(src); return; }
    var c = this.controls;
    this.renderer.toneMapping = T.NeutralToneMapping;
    this.renderer.toneMappingExposure = num(el, "data-model-exposure", 1);
    this.environment();
    var none = el.getAttribute("data-model-controls") === "none";
    c.enabled = !none && (!editor() || this.orbiting);
    c.enableRotate = true;
    c.enablePan = el.hasAttribute("data-model-pan");
    this.zoomable = !none;
    c.enableZoom = !!(this.zoomable && (this.active || editor()));
    var pl = pair(el, "data-model-pitch-limits", -90, 90);
    c.minPolarAngle = (90 - Math.min(90, Math.max(pl[0], pl[1]))) * Math.PI / 180;
    c.maxPolarAngle = (90 - Math.max(-90, Math.min(pl[0], pl[1]))) * Math.PI / 180;
    var speed = num(el, "data-model-rotate", 0);
    this.rotate = reduce ? 0 : speed;
    c.autoRotate = !!this.rotate;
    c.autoRotateSpeed = this.rotate / 6;
    var sh = Math.max(0, Math.min(1, num(el, "data-model-shadow", 0)));
    this.ground.material.opacity = sh;
    this.ground.visible = sh > 0;
    this.key.castShadow = sh > 0;
    this.setView(el.getAttribute("data-model-view") || "render");
    this.stats();
    this.animation();
    this.fullButtons();
    // Caméra refaite seulement si ses réglages changent (la vue du visiteur, ou celle réglée au
    // glisser dans l'éditeur, est gardée sinon).
    var cam = ["data-model-yaw", "data-model-pitch", "data-model-zoom", "data-model-target", "data-model-fov",
      "data-model-camera", "data-model-zoom-limits"].map(function (n) { return el.getAttribute(n); }).join("|");
    if (this.model && cam !== this.camKey) { this.camKey = cam; this.frame(true); }
    this.draw();
  };
  V.environment = function () {
    var T = RT.THREE;
    var el = this.el;
    var self = this;
    var env = el.getAttribute("data-model-env") || "studio";
    var bd = el.getAttribute("data-model-backdrop");
    var key = env + "|" + (bd === null ? "" : bd);
    if (key === this.envKey) return;
    this.envKey = key;
    var apply = function (tex) {
      if (self.envTex && self.envTex !== tex) self.envTex.dispose();
      self.envTex = tex;
      self.scene.environment = tex;
      self.scene.background = tex && bd !== null ? tex : null;
      self.scene.backgroundBlurriness = Math.max(0, Math.min(1, parseFloat(bd) || 0));
      self.hemi.intensity = tex ? 0 : 1.2;
      self.key.intensity = tex ? 0 : 2;
      self.draw();
    };
    if (env === "none") { apply(null); return; }
    if (/\.hdr(\?|#|$)/i.test(env)) {
      new RT.HDRLoader().load(env, function (t) {
        t.mapping = T.EquirectangularReflectionMapping;
        var p = self.pmrem.fromEquirectangular(t).texture;
        t.dispose();
        if (key === self.envKey) apply(p); else p.dispose();
      }, undefined, function () { if (key === self.envKey) apply(self.pmrem.fromScene(new RT.RoomEnvironment(), 0.04).texture); });
      return;
    }
    apply(this.pmrem.fromScene(new RT.RoomEnvironment(), 0.04).texture);
  };
  V.load = function (src) {
    var el = this.el;
    var self = this;
    this.src = src;
    this.clear();
    if (!src) { el.setAttribute("data-model-state", "empty"); this.draw(); return; }
    el.setAttribute("data-model-state", "loading");
    // Barre de chargement (couleur du texte), retirée au premier rendu.
    if (!this.bar) {
      this.bar = liveNode(el, "div");
      this.bar.style.cssText = "position:absolute;left:0;bottom:0;height:2px;width:0;background:currentColor;opacity:.6;transition:width .2s;pointer-events:none";
    }
    var gl = loaders(this.renderer);
    gl.load(src, function (g) {
      if (self.dead || src !== self.src) return;
      self.gltf = g;
      self.model = g.scene || (g.scenes && g.scenes[0]);
      if (!self.model) { el.setAttribute("data-model-state", "error"); return; }
      self.model.traverse(function (o) {
        if (o.isMesh) { o.castShadow = true; o.receiveShadow = false; }
      });
      self.scene.add(self.model);
      self.measure();
      self.mixer = g.animations && g.animations.length ? new RT.THREE.AnimationMixer(self.model) : null;
      self.anim = null;
      self.alt = new Map();
      self.view = "render";
      el.setAttribute("data-model-state", "ready");
      if (self.bar) { self.bar.remove(); self.bar = null; }
      self.update();
      self.frame(true);
      self.report();
    }, function (p) {
      if (self.bar && p && p.lengthComputable && p.total) self.bar.style.width = (100 * p.loaded / p.total) + "%";
    }, function (e) {
      console.error(e);
      if (src === self.src) el.setAttribute("data-model-state", "error");
      if (self.bar) { self.bar.remove(); self.bar = null; }
    });
  };
  // Sphère englobante du modèle à sa pose de départ.
  V.measure = function () {
    var T = RT.THREE;
    this.model.updateMatrixWorld(true);
    var box = new T.Box3().setFromObject(this.model);
    if (box.isEmpty()) box.set(new T.Vector3(-1, -1, -1), new T.Vector3(1, 1, 1));
    this.box = box;
    this.sphere = box.getBoundingSphere(new T.Sphere());
    if (!(this.sphere.radius > 0)) this.sphere.radius = 1;
    var r = this.sphere.radius;
    this.ground.scale.set(r * 8, r * 8, 1);
    this.ground.position.set(this.sphere.center.x, box.min.y, this.sphere.center.z);
    this.key.position.set(this.sphere.center.x + r * 0.15, box.min.y + r * 4, this.sphere.center.z + r * 0.25);
    this.key.target.position.copy(this.sphere.center);
    var sc = this.key.shadow.camera;
    sc.left = sc.bottom = -r * 2;
    sc.right = sc.top = r * 2;
    sc.near = r * 0.5;
    sc.far = r * 8;
    sc.updateProjectionMatrix();
  };
  // Distance qui cadre la sphère : le plus étroit des deux angles de vue décide.
  V.fitDistance = function () {
    var cam = this.camera;
    var v = cam.fov * Math.PI / 180;
    var h = 2 * Math.atan(Math.tan(v / 2) * cam.aspect);
    return this.sphere.radius / Math.sin(Math.min(v, h) / 2);
  };
  // Caméra réglée (ou caméra du fichier) ; `keep` : angle et distance relative de la vue actuelle
  // gardés (changement de taille après une interaction).
  V.frame = function (reset, keep) {
    if (!this.model) return;
    var T = RT.THREE;
    var el = this.el;
    var cam = this.camera;
    var c = this.controls;
    var w = el.clientWidth || 1;
    var hgt = el.clientHeight || 1;
    cam.aspect = w / hgt;
    var named = el.getAttribute("data-model-camera");
    var fileCam = null;
    if (named && this.gltf && this.gltf.cameras) {
      for (var i = 0; i < this.gltf.cameras.length; i++) {
        if (this.gltf.cameras[i].name === named) { fileCam = this.gltf.cameras[i]; break; }
      }
    }
    var r = this.sphere.radius;
    if (!keep) {
      // Élan d'un glisser précédent (amorti) vidé : la vue réglée est exacte.
      c.enableDamping = false;
      c.update();
      c.enableDamping = true;
    }
    if (fileCam && fileCam.isPerspectiveCamera) {
      // Caméra du fichier : sa position, son angle ; en portrait, champ élargi pour garder
      // l'angle horizontal prévu.
      fileCam.updateMatrixWorld(true);
      var fov = fileCam.fov;
      var fa = fileCam.aspect || 0;
      if (fa && cam.aspect < fa) {
        fov = 2 * Math.atan(Math.tan(fov * Math.PI / 360) * fa / cam.aspect) * 180 / Math.PI;
      }
      cam.fov = Math.min(fov, 120);
      if (!keep) {
        fileCam.getWorldPosition(cam.position);
        var dir = new T.Vector3();
        fileCam.getWorldDirection(dir);
        var d = Math.max(r * 0.01, cam.position.distanceTo(this.sphere.center));
        c.target.copy(cam.position).addScaledVector(dir, d);
        this.fit = d;
      }
    } else {
      var prev = this.fit;
      cam.fov = Math.max(5, Math.min(120, num(el, "data-model-fov", 30)));
      cam.updateProjectionMatrix();
      this.fit = this.fitDistance();
      if (keep && prev) {
        var off = cam.position.clone().sub(c.target).multiplyScalar(this.fit / prev);
        cam.position.copy(c.target).add(off);
      } else {
        var t = triple(el, "data-model-target");
        c.target.copy(this.sphere.center).add(new T.Vector3(t[0], t[1], t[2]).multiplyScalar(r / 100));
        var yaw = num(el, "data-model-yaw", 0) * Math.PI / 180;
        var pitch = Math.max(-89.9, Math.min(89.9, num(el, "data-model-pitch", 15))) * Math.PI / 180;
        var dist = this.fit * 100 / Math.max(1, num(el, "data-model-zoom", 100));
        cam.position.set(
          c.target.x + dist * Math.cos(pitch) * Math.sin(yaw),
          c.target.y + dist * Math.sin(pitch),
          c.target.z + dist * Math.cos(pitch) * Math.cos(yaw)
        );
      }
    }
    var zl = pair(el, "data-model-zoom-limits", 50, 400);
    c.minDistance = this.fit * 100 / Math.max(zl[0], zl[1], 1);
    c.maxDistance = this.fit * 100 / Math.max(1, Math.min(zl[0], zl[1]));
    cam.near = Math.max(r / 1000, (this.fit - r * 2) / 10, 1e-4);
    cam.far = this.fit * 4 + r * 20;
    cam.updateProjectionMatrix();
    c.update();
    if (reset) this.touched = false;
    this.draw();
  };
  V.resize = function () {
    if (!this.renderer) return;
    var w = this.el.clientWidth;
    var h = this.el.clientHeight;
    if (!w || !h) return;
    if (w === this.w && h === this.h) return;
    this.w = w;
    this.h = h;
    this.renderer.setSize(w, h, false);
    this.bandKey = null;
    if (this.model) this.frame(false, this.touched);
    else { this.camera.aspect = w / h; this.camera.updateProjectionMatrix(); }
    this.draw();
  };
  V.draw = function () {
    if (this.raf || !this.renderer || this.dead) return;
    var self = this;
    this.raf = requestAnimationFrame(function (t) { self.raf = 0; self.tick(t); });
  };
  V.tick = function (t) {
    var dt = this.last ? Math.min(0.1, (t - this.last) / 1000) : 0;
    this.last = t;
    var c = this.controls;
    // Rotation automatique reprise trois secondes après une interaction.
    c.autoRotate = !!this.rotate && this.spin !== false && !this.dragging && (!this.idle || t - this.idle > 3000);
    var moving = c.update(dt);
    if (this.mixer && this.playing) { this.mixer.update(dt); moving = true; }
    if (c.autoRotate) moving = true;
    if (this.layers && this.model) this.renderLayers();
    else this.renderer.render(this.scene, this.camera);
    if (this.poster && this.model && this.poster.style.opacity !== "0") this.poster.style.opacity = "0";
    if ((moving || (this.idle && this.rotate && t - this.idle <= 3000)) && this.visible) this.draw();
    else this.last = 0;
  };
  V.command = function (cmd) {
    var el = this.el;
    if (cmd === "fullscreen") {
      if (document.fullscreenElement === el) document.exitFullscreen();
      else if (el.requestFullscreen) el.requestFullscreen().catch(function () { /* refusé */ });
    } else if (cmd === "reset") {
      this.frame(true);
    } else if (cmd === "rotate") {
      this.spin = this.spin === false;
      this.draw();
    } else if (cmd === "play") {
      if (this.anim) { this.playing = !this.playing; this.anim.paused = !this.playing; this.draw(); }
    } else if (cmd.indexOf("view:") === 0) {
      this.setView(cmd.slice(5));
    }
    this.pressed();
  };
  // Boutons enfoncés (`aria-pressed`) : vue choisie, rotation et lecture en cours.
  V.pressed = function () {
    var self = this;
    var bs = this.el.querySelectorAll("[data-model-do]");
    for (var i = 0; i < bs.length; i++) {
      var d = bs[i].getAttribute("data-model-do");
      var on = d === "view:" + this.view || (d === "rotate" && !!this.rotate && this.spin !== false) ||
        (d === "play" && !!this.playing) || (d === "fullscreen" && document.fullscreenElement === self.el);
      if (d.indexOf("view:") === 0 || d === "rotate" || d === "play" || d === "fullscreen") {
        bs[i].setAttribute("aria-pressed", on ? "true" : "false");
        bs[i].classList.toggle("is-active", on);
      }
    }
  };
  V.fullButtons = function () {
    var bs = this.el.querySelectorAll('[data-model-do="fullscreen"]');
    for (var i = 0; i < bs.length; i++) bs[i].style.display = shared.fs === "no" ? "none" : "";
  };
  // Vues de contrôle : matériaux de remplacement, faits une fois par matériau.
  V.setView = function (view) {
    if (!this.model) { this.view = view; return; }
    var T = RT.THREE;
    var self = this;
    var ok = ["render", "albedo", "normals", "roughness", "metalness", "wireframe", "layers"];
    if (ok.indexOf(view) < 0) view = "render";
    var wire = new T.Color(getComputedStyle(this.el).color || "#888");
    this.layers = view === "layers";
    this.bandKey = null;
    if (!this.layers) this.layerMarks();
    if (this.layers) view = "render";
    this.model.traverse(function (o) {
      if (!o.isMesh) return;
      if (!o.userData.rvMat) o.userData.rvMat = o.material;
      var orig = o.userData.rvMat;
      if (view === "render") { o.material = orig; return; }
      var one = function (m) { return self.altMaterial(m, view, wire); };
      o.material = Array.isArray(orig) ? orig.map(one) : one(orig);
    });
    this.view = this.layers ? "layers" : view;
    this.pressed();
    this.draw();
  };
  // Calques : une bande oblique par vue (rendu, normales, couleur, métal, rugosité, filaire), chaque
  // vue dessinée seulement dans sa bande (tampon de pochoir), traits et noms par-dessus.
  var LAYERS = ["render", "normals", "albedo", "metalness", "roughness", "wireframe"];
  var SLOPE = 0.42;
  V.layerX = function (k, y) {
    // Trait `k` (0 à 6) : abscisse à la hauteur `y` (pixels, depuis le haut).
    var sp = this.span;
    return sp.x0 + (sp.x1 - sp.x0) * k / LAYERS.length + SLOPE * (y - sp.cy);
  };
  // Largeur partagée entre les bandes : celle du modèle à l'écran (entre 40 % et 90 % de la
  // largeur), pour que chaque vue en montre une part.
  V.layerSpan = function () {
    var w = this.w, h = this.h;
    var cx = w / 2, cy = h / 2, half = w * 0.45;
    if (this.sphere && this.camera.isPerspectiveCamera) {
      var v = this.sphere.center.clone().project(this.camera);
      cx = (v.x + 1) / 2 * w;
      cy = (1 - v.y) / 2 * h;
      var d = this.camera.position.distanceTo(this.sphere.center);
      var r = this.sphere.radius / (d * Math.tan(this.camera.fov * Math.PI / 360)) * h / 2;
      half = Math.min(Math.max(r * 0.7, w * 0.2), w * 0.45);
    }
    return { x0: cx - half, x1: cx + half, cy: cy };
  };
  V.layerMarks = function () {
    if (this.marks) { this.marks.remove(); this.marks = null; }
    if (!this.layers || !this.span) return;
    var m = liveNode(this.el, "div");
    m.setAttribute("aria-hidden", "true");
    m.setAttribute("data-rv-layers", "");
    m.style.cssText = "position:absolute;inset:0;z-index:1;overflow:hidden;pointer-events:none;color:inherit;font:12px/1 system-ui,sans-serif";
    var deg = Math.atan(SLOPE) * 180 / Math.PI;
    var top = 0;
    var len = Math.sqrt(this.h * this.h * (1 + SLOPE * SLOPE));
    var w = this.w, n = LAYERS.length;
    // Noms sous la barre des vues si elle est en haut, sinon près du bord.
    var bar = this.el.querySelector(":scope > [data-revel-part=views]");
    var ty = bar && bar.offsetTop < this.h / 2 ? bar.offsetTop + bar.offsetHeight + 12 : 14;
    for (var k = 0; k < n; k++) {
      if (k > 0) {
        var line = document.createElement("div");
        line.style.cssText = "position:absolute;top:0;left:" + this.layerX(k, top) + "px;width:1px;height:" + len + "px;background:currentColor;opacity:.55;transform-origin:0 0;transform:rotate(" + (-deg) + "deg)";
        m.appendChild(line);
      }
      // Nom au milieu de la bande, en haut, le long des traits (bandes du bord : partie visible).
      var a = Math.max(this.layerX(k, ty), 0), b = Math.min(this.layerX(k + 1, ty), w);
      var lab = document.createElement("div");
      lab.textContent = word(this.el, LAYERS[k]);
      lab.style.cssText = "position:absolute;top:" + ty + "px;left:" + Math.max((a + b) / 2 + 6, 20) + "px;opacity:.8;transform-origin:0 0;transform:rotate(" + (90 - deg) + "deg);white-space:nowrap;letter-spacing:.04em";
      m.appendChild(lab);
    }
    this.marks = m;
  };
  // Bandes écrites dans le pochoir (valeur k + 1 pour la vue k), refaites si la taille change.
  V.layerBands = function () {
    var T = RT.THREE;
    var sp = this.layerSpan();
    var key = this.w + "x" + this.h + ":" + Math.round(sp.x0) + "," + Math.round(sp.x1) + "," + Math.round(sp.cy);
    if (this.bandKey === key && this.bands) return;
    this.bandKey = key;
    this.span = sp;
    this.layerMarks();
    if (!this.bands) {
      this.bands = new T.Scene();
      this.bandCam = new T.OrthographicCamera(0, 1, 1, 0, -1, 1);
    }
    var sc = this.bands;
    while (sc.children.length) {
      var c = sc.children.pop();
      c.geometry.dispose();
      c.material.dispose();
    }
    var w = this.w, h = this.h, far = w + h * SLOPE + 10;
    this.bandCam.right = w;
    this.bandCam.top = h;
    this.bandCam.updateProjectionMatrix();
    for (var k = 0; k < LAYERS.length; k++) {
      var l0 = k === 0 ? [-far, -far] : [this.layerX(k, 0), this.layerX(k, h)];
      var l1 = k === LAYERS.length - 1 ? [far + w, far + w] : [this.layerX(k + 1, 0), this.layerX(k + 1, h)];
      // Coins (y vers le haut pour la caméra) : haut gauche, haut droit, bas droit, bas gauche.
      var pts = [l0[0], h, l1[0], h, l1[1], 0, l0[1], 0];
      var g = new T.BufferGeometry();
      g.setAttribute("position", new T.Float32BufferAttribute([
        pts[0], pts[1], 0, pts[2], pts[3], 0, pts[4], pts[5], 0,
        pts[0], pts[1], 0, pts[4], pts[5], 0, pts[6], pts[7], 0
      ], 3));
      var mat = new T.MeshBasicMaterial({
        colorWrite: false, depthWrite: false, depthTest: false, side: T.DoubleSide,
        stencilWrite: true, stencilRef: k + 1, stencilFunc: T.AlwaysStencilFunc, stencilZPass: T.ReplaceStencilOp
      });
      sc.add(new T.Mesh(g, mat));
    }
  };
  V.renderLayers = function () {
    var T = RT.THREE;
    var r = this.renderer;
    var self = this;
    this.layerBands();
    var bg = this.scene.background;
    this.scene.background = null;
    r.autoClear = false;
    r.clear(true, true, true);
    r.render(this.bands, this.bandCam);
    var wire = new T.Color(0x202226);
    var used = [];
    var stencil = function (m, ref) {
      m.stencilWrite = true;
      m.stencilRef = ref;
      m.stencilFunc = T.EqualStencilFunc;
      m.stencilFail = m.stencilZFail = m.stencilZPass = T.KeepStencilOp;
      used.push(m);
    };
    for (var k = 0; k < LAYERS.length; k++) {
      var view = LAYERS[k];
      var passes = view === "wireframe" ? ["clay", "wireframe"] : [view];
      for (var p = 0; p < passes.length; p++) {
        var v = passes[p];
        r.clearDepth();
        this.model.traverse(function (o) {
          if (!o.isMesh) return;
          var orig = o.userData.rvMat || o.material;
          var one = function (m) {
            var x = v === "render" ? m : self.altMaterial(m, v, wire);
            stencil(x, k + 1);
            return x;
          };
          o.material = Array.isArray(orig) ? orig.map(one) : one(orig);
        });
        this.ground.visible = view === "render" && this.ground.material.opacity > 0;
        if (this.ground.visible) stencil(this.ground.material, k + 1);
        r.render(this.scene, this.camera);
      }
    }
    // Matériaux remis comme avant (rendu, sans pochoir).
    used.forEach(function (m) { m.stencilWrite = false; m.stencilFunc = T.AlwaysStencilFunc; });
    this.model.traverse(function (o) { if (o.isMesh && o.userData.rvMat) o.material = o.userData.rvMat; });
    this.ground.visible = this.ground.material.opacity > 0;
    this.scene.background = bg;
    r.autoClear = true;
  };
  // Nombre de triangles et de sommets du modèle (`data-model-stats`).
  V.stats = function () {
    var on = this.el.hasAttribute("data-model-stats") && this.model;
    if (!on) { if (this.statsBox) { this.statsBox.remove(); this.statsBox = null; } return; }
    var tri = 0, vert = 0;
    this.model.traverse(function (o) {
      if (!o.isMesh || !o.geometry) return;
      var g = o.geometry, pos = g.getAttribute("position");
      if (!pos) return;
      vert += pos.count;
      tri += (g.index ? g.index.count : pos.count) / 3;
    });
    if (!this.statsBox) {
      this.statsBox = liveNode(this.el, "div");
      this.statsBox.setAttribute("data-rv-stats", "");
      // Place et aspect : styles de l'outil (`[data-rv-stats]`, en bas à gauche), réglables par le site.
    }
    var lang = document.documentElement.lang || undefined;
    var fmt = function (n) { try { return Math.round(n).toLocaleString(lang); } catch (_) { return String(Math.round(n)); } };
    this.statsBox.textContent = "";
    var a = document.createElement("div");
    a.textContent = word(this.el, "triangles") + " : " + fmt(tri);
    var b = document.createElement("div");
    b.textContent = word(this.el, "vertices") + " : " + fmt(vert);
    this.statsBox.appendChild(a);
    this.statsBox.appendChild(b);
  };
  V.altMaterial = function (m, view, wire) {
    var T = RT.THREE;
    var k = view;
    var byMat = this.alt.get(m);
    if (!byMat) { byMat = {}; this.alt.set(m, byMat); }
    if (byMat[k]) {
      if (view === "wireframe") byMat[k].color.copy(wire);
      return byMat[k];
    }
    var common = { side: m.side, transparent: m.transparent, opacity: m.opacity, alphaTest: m.alphaTest };
    var out;
    if (view === "albedo") {
      out = new T.MeshBasicMaterial(Object.assign({ map: m.map || null, color: m.color ? m.color.clone() : 0xffffff, vertexColors: m.vertexColors, toneMapped: false }, common));
    } else if (view === "normals") {
      out = new T.MeshNormalMaterial({ side: m.side, normalMap: m.normalMap || null, normalScale: m.normalScale || new T.Vector2(1, 1) });
    } else if (view === "wireframe") {
      out = new T.MeshBasicMaterial({ color: wire, wireframe: true, toneMapped: false });
    } else if (view === "clay") {
      // Fond clair sous le filaire des calques (faces visibles, traits par-dessus).
      out = new T.MeshBasicMaterial({ color: 0xd9dadc, toneMapped: false, side: m.side, polygonOffset: true, polygonOffsetFactor: 1, polygonOffsetUnits: 1 });
    } else {
      // Rugosité (canal vert) ou métal (canal bleu) de la texture glTF, en gris brut.
      var rough = view === "roughness";
      var map = (rough ? m.roughnessMap : m.metalnessMap) || null;
      var val = rough ? (m.roughness !== undefined ? m.roughness : 1) : (m.metalness !== undefined ? m.metalness : 0);
      out = new T.MeshBasicMaterial({ map: map, color: new T.Color(1, 1, 1), toneMapped: false, side: m.side });
      var ch = rough ? "g" : "b";
      out.onBeforeCompile = function (s) {
        s.uniforms.rvValue = { value: val };
        s.fragmentShader = "uniform float rvValue;\n" + s.fragmentShader.replace(
          "#include <map_fragment>",
          "float rvV = rvValue;\n#ifdef USE_MAP\nrvV *= texture2D( map, vMapUv )." + ch + ";\n#endif\n" +
          "diffuseColor.rgb = vec3( pow( rvV, 2.2 ) );"
        );
      };
      out.customProgramCacheKey = function () { return "rv-" + ch; };
    }
    byMat[k] = out;
    return out;
  };
  V.animation = function () {
    if (!this.mixer) return;
    var el = this.el;
    var name = el.getAttribute("data-model-animation") || "";
    var loop = el.hasAttribute("data-model-loop");
    var speed = num(el, "data-model-speed", 1);
    var key = name + "|" + loop + "|" + speed;
    if (key === this.animKey) return;
    this.animKey = key;
    this.mixer.stopAllAction();
    this.anim = null;
    this.playing = false;
    var clip = null;
    var list = this.gltf.animations;
    for (var i = 0; i < list.length; i++) if (list[i].name === name) { clip = list[i]; break; }
    if (!clip && name === "*") clip = list[0];
    if (!clip) { this.mixer.setTime(0); this.draw(); return; }
    var a = this.mixer.clipAction(clip);
    a.setLoop(loop ? RT.THREE.LoopRepeat : RT.THREE.LoopOnce, Infinity);
    a.clampWhenFinished = true;
    a.timeScale = speed;
    a.play();
    this.anim = a;
    this.playing = !reduce;
    a.paused = reduce;
    if (reduce) this.mixer.update(0);
    this.draw();
  };
  // Noms des animations et caméras du fichier, pour les réglages de l'éditeur.
  V.report = function () {
    var g = this.gltf;
    var info = {
      animations: (g.animations || []).map(function (a) { return a.name; }),
      cameras: (g.cameras || []).filter(function (c) { return c.isPerspectiveCamera && c.name; }).map(function (c) { return c.name; })
    };
    this.info = info;
    try { this.el.dispatchEvent(new CustomEvent("revel-model-ready", { bubbles: true, detail: info })); } catch (_) { /* ancien navigateur */ }
  };
  // Vue actuelle en réglages de l'élément (bouton « Prendre la vue actuelle » de l'éditeur).
  V.current = function () {
    if (!this.model) return null;
    var c = this.controls;
    var r = this.sphere.radius;
    var off = this.camera.position.clone().sub(c.target);
    var dist = off.length();
    var yaw = Math.atan2(off.x, off.z) * 180 / Math.PI;
    var pitch = Math.asin(Math.max(-1, Math.min(1, off.y / dist))) * 180 / Math.PI;
    var t = c.target.clone().sub(this.sphere.center).multiplyScalar(100 / r);
    var round = function (v, n) { var p = Math.pow(10, n || 0); return Math.round(v * p) / p; };
    return {
      yaw: round(yaw),
      pitch: round(pitch),
      zoom: round(this.fit * 100 / dist),
      target: Math.abs(t.x) + Math.abs(t.y) + Math.abs(t.z) < 0.5 ? "" : [round(t.x), round(t.y), round(t.z)].join(" ")
    };
  };
  V.orbit = function (on) {
    this.orbiting = !!on;
    if (this.controls) {
      this.controls.enabled = on || !editor();
      this.controls.enableZoom = !!this.zoomable;
    }
  };
  V.clear = function () {
    if (this.model) {
      this.scene.remove(this.model);
      this.model.traverse(function (o) {
        if (o.geometry) o.geometry.dispose();
        var ms = o.userData && o.userData.rvMat ? o.userData.rvMat : o.material;
        (Array.isArray(ms) ? ms : ms ? [ms] : []).forEach(function (m) {
          for (var k in m) if (m[k] && m[k].isTexture) m[k].dispose();
          m.dispose();
        });
      });
      this.alt.forEach(function (o) { for (var k in o) o[k].dispose(); });
      this.alt = new Map();
    }
    if (this.statsBox) { this.statsBox.remove(); this.statsBox = null; }
    this.model = null;
    this.camKey = null;
    this.gltf = null;
    this.mixer = null;
    this.anim = null;
    this.animKey = null;
  };
  V.teardown = function () {
    this.dead = true;
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    if (this.ro) this.ro.disconnect();
    this.offs.forEach(function (f) { f(); });
    this.offs = [];
    if (this.renderer) {
      this.clear();
      if (this.envTex) this.envTex.dispose();
      this.controls.dispose();
      this.pmrem.dispose();
      this.renderer.dispose();
      try { this.renderer.forceContextLoss(); } catch (_) { /* déjà perdu */ }
    }
    var ns = this.el.querySelectorAll(":scope > [data-rv-live]");
    for (var i = 0; i < ns.length; i++) ns[i].remove();
    this.renderer = null;
    this.marks = null;
    this.statsBox = null;
    this.bands = null;
    this.bandKey = null;
    this.bar = null;
    this.poster = null;
    this.envKey = null;
    this.src = null;
    this.w = this.h = 0;
  };
  V.destroy = function () {
    this.teardown();
    this.el.removeAttribute("data-model-state");
    this.el.__rvModel = null;
    var i = live.indexOf(this);
    if (i >= 0) live.splice(i, 1);
  };

  // Chargement à l'approche de l'écran, pause hors de l'écran ; au-delà de huit modèles ouverts,
  // ceux qui sont loin de l'écran depuis 20 s sont fermés (le navigateur limite les contextes WebGL).
  var io = "IntersectionObserver" in window ? new IntersectionObserver(function (es) {
    es.forEach(function (e) {
      var el = e.target;
      var v = el.__rvModel;
      if (e.isIntersecting) {
        el.__rvAway = 0;
        if (!v) { v = el.__rvModel = new Viewer(el); live.push(v); }
        v.visible = true;
        v.draw();
      } else if (v) {
        v.visible = false;
        el.__rvAway = performance.now();
      }
    });
    if (live.length > MAX_LIVE) {
      var now = performance.now();
      live.slice().forEach(function (v) {
        if (live.length > MAX_LIVE && v.el.__rvAway && now - v.el.__rvAway > 20000) v.destroy();
      });
    }
  }, { rootMargin: "200px" }) : null;
  function setup(el) {
    if (el.__rvModelSeen) return;
    el.__rvModelSeen = true;
    if (io) io.observe(el);
    else { el.__rvModel = new Viewer(el); live.push(el.__rvModel); }
  }
  function all() {
    var els = document.querySelectorAll("[data-model]");
    for (var i = 0; i < els.length; i++) setup(els[i]);
    // Éléments retirés de la page : lecteur fermé.
    live.slice().forEach(function (v) { if (!v.el.isConnected) { if (io) io.unobserve(v.el); v.destroy(); } });
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
  window.RevelModel = {
    refresh: all,
    update: function (el) { if (el.__rvModel) el.__rvModel.update(); },
    view: function (el) { return el.__rvModel ? el.__rvModel.current() : null; },
    info: function (el) { return el.__rvModel ? el.__rvModel.info || null : null; },
    orbit: function (el, on) { if (el.__rvModel) el.__rvModel.orbit(on); },
    destroy: function (el) {
      if (el.__rvModel) el.__rvModel.destroy();
      if (io) io.unobserve(el);
      el.__rvModelSeen = false;
    }
  };
})();
