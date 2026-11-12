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
     les vues en bandes obliques égales sur tout l'écran ; un appui sur une bande la montre seule,
     un nouvel appui revient aux bandes, comme le lecteur de Marmoset) ;
   - data-model-stats : nombre de triangles et de vertex (icônes) affiché en bas à gauche.
   Variable CSS `--model-layers-top` : hauteur des noms des bandes (sous les boutons du site).
   Boutons contenus (`data-model-do`) : fullscreen, reset, rotate, play, layers (ouvre ou ferme les
   calques), view:<vue>. Vue montrée : `data-model-shown` (layers, ou le nom de la vue).
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
    this.strip = null;
    this.hot = -1;
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
    // Calques : un appui bref (sans glisser) choisit la bande visée, ou revient aux bandes.
    var down = null;
    var at = function (e) {
      var r = canvas.getBoundingClientRect();
      return [e.clientX - r.left, e.clientY - r.top];
    };
    on(canvas, "pointerdown", function (e) {
      down = e.button === 0 ? { x: e.clientX, y: e.clientY, t: performance.now() } : null;
    });
    on(canvas, "pointerup", function (e) {
      var d = down;
      down = null;
      if (!d || editor() || self.strip == null) return;
      if (Math.abs(e.clientX - d.x) > 6 || Math.abs(e.clientY - d.y) > 6 || performance.now() - d.t > 600) return;
      var p = at(e);
      self.tap(p[0], p[1]);
    });
    on(canvas, "pointermove", function (e) {
      if (self.strip !== "menu" || editor() || e.buttons) { self.hover(-1); return; }
      var p = at(e);
      self.hover(self.bandAt(p[0], p[1]));
    });
    on(canvas, "pointerleave", function () { self.hover(-1); });
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
    if (this.cuts) this.cuts = this.cutTargets();
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
    if (this.stripStep(dt)) moving = true;
    if (this.stripLive && this.model && this.cuts) { this.renderLayers(); this.layerPlace(); }
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
    } else if (cmd === "layers" || cmd === "view:layers") {
      // Calques ouverts ou fermés (une vue choisie dans les calques : retour aux bandes).
      if (this.model) this.strips(this.strip === "menu" ? null : "menu");
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
      var on = d === "view:" + this.view || (d === "layers" && this.strip != null) ||
        (d === "rotate" && !!this.rotate && this.spin !== false) ||
        (d === "play" && !!this.playing) || (d === "fullscreen" && document.fullscreenElement === self.el);
      if (d.indexOf("view:") === 0 || d === "layers" || d === "rotate" || d === "play" || d === "fullscreen") {
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
  V.applyView = function (view) {
    var T = RT.THREE;
    var self = this;
    var wire = new T.Color(getComputedStyle(this.el).color || "#888");
    this.model.traverse(function (o) {
      if (!o.isMesh) return;
      if (!o.userData.rvMat) o.userData.rvMat = o.material;
      var orig = o.userData.rvMat;
      if (view === "render") { o.material = orig; return; }
      var one = function (m) { return self.altMaterial(m, view, wire); };
      o.material = Array.isArray(orig) ? orig.map(one) : one(orig);
    });
    this.single = view;
  };
  V.setView = function (view) {
    if (!this.model) { this.view = view; return; }
    var ok = ["render", "albedo", "normals", "roughness", "metalness", "wireframe", "layers"];
    if (ok.indexOf(view) < 0) view = "render";
    if (view === "layers") {
      // Déjà ouverts : le choix du visiteur (bandes ou vue seule) est gardé.
      if (this.strip == null) this.strips("menu");
      return;
    }
    this.strips(null, true);
    this.applyView(view);
    this.view = view;
    this.shown();
    this.pressed();
    this.draw();
  };
  // Calques, à la manière du lecteur de Marmoset : les vues de contrôle en bandes obliques égales
  // sur tout l'écran, le rendu à droite. Un appui sur une bande la montre seule ; un nouvel appui
  // revient aux bandes ; la bande du rendu ferme les calques. Les traits glissent d'un état à
  // l'autre (demi-vie de 70 ms). Chaque vue n'est dessinée que dans sa bande (tampon de pochoir).
  var LAYERS = ["normals", "albedo", "metalness", "roughness", "wireframe", "render"];
  var SLANT = 0.25;
  var HALF = 0.07;
  // Pente des traits, en pixels par pixel (un quart de la largeur pour toute la hauteur).
  V.slope = function () { return SLANT * this.w / this.h; };
  // Place visée de chaque trait (abscisse à mi-hauteur) : bandes égales, ou hors de l'écran de part
  // et d'autre de la vue montrée seule (calques fermés : tous à gauche, le rendu partout).
  V.cutTargets = function () {
    var n = LAYERS.length, w = this.w, off = this.slope() * this.h / 2 + 2;
    var sel = this.strip == null || this.strip === "menu" ? -1 : LAYERS.indexOf(this.strip);
    var out = [];
    for (var k = 1; k < n; k++) {
      if (this.strip === "menu") out.push(w * k / n);
      else out.push(sel >= 0 && k > sel ? w + off : -off);
    }
    return out;
  };
  // État des calques : `null` (fermés), "menu" (bandes) ou le nom de la vue montrée seule.
  V.strips = function (state, instant) {
    if (state == null && this.strip == null && !this.stripLive) return;
    if (state === this.strip && !instant) return;
    if (!this.w) this.resize();
    if (!this.w) return;
    var opening = this.strip == null && state != null;
    // Départ : calques fermés (traits hors de l'écran, à gauche).
    if (!this.cuts) { this.strip = null; this.cuts = this.cutTargets(); }
    this.strip = state;
    if (opening) this.applyView("render");
    if (instant || reduce) this.cuts = this.cutTargets();
    if (state != null) this.stripLive = true;
    else if (instant || reduce) this.stripLive = false;
    this.view = state != null ? "layers" : (this.single || "render");
    this.layerMarks();
    this.hover(-1);
    this.shown();
    this.pressed();
    this.draw();
  };
  // Un pas du glissement des traits ; vrai tant qu'ils bougent.
  V.stripStep = function (dt) {
    if (!this.stripLive || !this.cuts) return false;
    var to = this.cutTargets(), moving = false;
    var f = Math.exp(-0.6931 / HALF * dt);
    for (var i = 0; i < to.length; i++) {
      var d = to[i] - this.cuts[i];
      if (Math.abs(d) < 0.5) this.cuts[i] = to[i];
      else { this.cuts[i] = to[i] - d * f; moving = true; }
    }
    if (!moving && this.strip == null) { this.stripLive = false; this.layerMarks(); }
    return moving;
  };
  // Trait `i` (0 à 4) : abscisse à la hauteur `y` (pixels, depuis le haut).
  V.cutX = function (i, y) { return this.cuts[i] + this.slope() * (y - this.h / 2); };
  // Bande (0 à 5, dans l'ordre de LAYERS) sous le point (x, y).
  V.bandAt = function (x, y) {
    if (!this.cuts) return -1;
    for (var i = 0; i < LAYERS.length - 1; i++) if (x < this.cutX(i, y)) return i;
    return LAYERS.length - 1;
  };
  V.pick = function (view) { this.strips(view === "render" ? null : view); };
  V.tap = function (x, y) {
    if (this.strip == null) return;
    if (this.strip === "menu") this.pick(LAYERS[this.bandAt(x, y)]);
    else this.strips("menu");
  };
  // Bande survolée : son nom en évidence, pointeur en main sur les bandes.
  V.hover = function (band) {
    if (band === this.hot) return;
    this.hot = band;
    if (this.canvas) this.canvas.style.cursor = band >= 0 ? "pointer" : "";
    var E = this.markEls;
    if (E) for (var i = 0; i < E.labels.length; i++) E.labels[i].classList.toggle("is-hover", i === band);
  };
  // Vue montrée, lue par les styles du site et les essais.
  V.shown = function () {
    this.el.setAttribute("data-model-shown", this.strip === "menu" ? "layers" : (this.strip || this.single || "render"));
  };
  // Traits et noms par-dessus le modèle (allure : styles de l'outil). Chaque nom est un bouton,
  // suspendu au trait qui ferme sa bande à droite.
  V.layerMarks = function () {
    if (!this.stripLive) {
      if (this.marks) { this.marks.remove(); this.marks = null; this.markEls = null; }
      return;
    }
    var self = this;
    if (!this.marks) {
      var m = liveNode(this.el, "div");
      m.setAttribute("data-rv-layers", "");
      var lines = [], labels = [];
      LAYERS.slice(0, -1).forEach(function (v) {
        var line = document.createElement("div");
        line.setAttribute("data-rv-layer-line", "");
        line.setAttribute("aria-hidden", "true");
        var lab = document.createElement("button");
        lab.type = "button";
        lab.setAttribute("data-rv-layer", v);
        lab.textContent = word(self.el, v);
        lab.addEventListener("click", function (e) {
          e.stopPropagation();
          if (!editor() && self.strip === "menu") self.pick(v);
        });
        m.appendChild(line);
        m.appendChild(lab);
        lines.push(line);
        labels.push(lab);
      });
      var name = document.createElement("div");
      name.setAttribute("data-rv-layer-name", "");
      m.appendChild(name);
      this.marks = m;
      this.markEls = { lines: lines, labels: labels, name: name };
    }
    var E = this.markEls;
    var one = this.strip != null && this.strip !== "menu";
    if (one) E.name.textContent = word(this.el, this.strip);
    E.name.classList.toggle("is-on", one);
    for (var i = 0; i < E.labels.length; i++) {
      // Noms atteignables au clavier seulement quand les bandes sont montrées.
      E.labels[i].tabIndex = this.strip === "menu" ? 0 : -1;
    }
    this.layerPlace();
  };
  V.layerPlace = function () {
    var E = this.markEls;
    if (!E || !this.cuts) return;
    var s = this.slope(), h = this.h;
    var deg = Math.atan(s) * 180 / Math.PI;
    var len = Math.sqrt(1 + s * s) * h;
    // Haut des noms : `--model-layers-top` (réglé par le site, sous ses boutons), sinon sous la barre
    // des vues si elle est en haut, sinon 60 px.
    var bar = this.el.querySelector(":scope > [data-revel-part=views]");
    var y0 = parseFloat(getComputedStyle(this.el).getPropertyValue("--model-layers-top"));
    if (!isFinite(y0)) y0 = Math.max(60, bar && bar.offsetTop < h / 2 ? bar.offsetTop + bar.offsetHeight + 16 : 0);
    for (var i = 0; i < E.lines.length; i++) {
      var xt = this.cuts[i] - s * h / 2;
      E.lines[i].style.height = len + "px";
      E.lines[i].style.transform = "translateX(" + xt.toFixed(2) + "px) rotate(" + (-deg) + "deg)";
      E.labels[i].style.transform = "translate(" + (xt + s * y0).toFixed(2) + "px," + y0 + "px) rotate(" + (90 - deg) + "deg)";
    }
  };
  // Bandes écrites dans le pochoir (valeur k + 1 pour la vue k) ; rend les bandes visibles.
  V.layerBands = function () {
    var T = RT.THREE;
    var n = LAYERS.length, w = this.w, h = this.h, k;
    if (!this.bands) {
      this.bands = new T.Scene();
      this.bandCam = new T.OrthographicCamera(0, 1, 1, 0, -1, 1);
      for (k = 0; k < n; k++) {
        var g = new T.BufferGeometry();
        g.setAttribute("position", new T.BufferAttribute(new Float32Array(18), 3));
        var mesh = new T.Mesh(g, new T.MeshBasicMaterial({
          colorWrite: false, depthWrite: false, depthTest: false, side: T.DoubleSide,
          stencilWrite: true, stencilRef: k + 1, stencilFunc: T.AlwaysStencilFunc, stencilZPass: T.ReplaceStencilOp
        }));
        mesh.frustumCulled = false;
        this.bands.add(mesh);
      }
    }
    this.bandCam.right = w;
    this.bandCam.top = h;
    this.bandCam.updateProjectionMatrix();
    var far = w + h + 10;
    var seen = [];
    for (k = 0; k < n; k++) {
      // Bords gauche et droit de la bande, en haut et en bas de l'écran.
      var a0 = k === 0 ? -far : this.cutX(k - 1, 0), a1 = k === 0 ? -far : this.cutX(k - 1, h);
      var b0 = k === n - 1 ? w + far : this.cutX(k, 0), b1 = k === n - 1 ? w + far : this.cutX(k, h);
      var p = this.bands.children[k].geometry.attributes.position;
      // La caméra des bandes compte les hauteurs depuis le bas.
      p.setXYZ(0, a0, h, 0); p.setXYZ(1, b0, h, 0); p.setXYZ(2, b1, 0, 0);
      p.setXYZ(3, a0, h, 0); p.setXYZ(4, b1, 0, 0); p.setXYZ(5, a1, 0, 0);
      p.needsUpdate = true;
      seen.push(b0 - a0 > 0.25 && Math.max(b0, b1) > 0 && Math.min(a0, a1) < w);
    }
    return seen;
  };
  V.renderLayers = function () {
    var T = RT.THREE;
    var r = this.renderer;
    var self = this;
    var seen = this.layerBands();
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
      if (!seen[k]) continue;
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
  // Nombre de triangles et de vertex du modèle (`data-model-stats`) : une icône et un nombre
  // chacun, le nom en info-bulle.
  var STAT_ICONS = {
    triangles: '<path d="M8 2.6 14 13H2z"/>',
    vertices: '<path d="M2.6 13 8 4.2 13.4 13"/><circle cx="8" cy="4.2" r="1.9" fill="currentColor" stroke="none"/>'
  };
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
      // Place et aspect : styles de l'outil (`[data-rv-stats]`, en bas à gauche).
    }
    var self = this;
    var lang = document.documentElement.lang || undefined;
    var fmt = function (n) { try { return Math.round(n).toLocaleString(lang); } catch (_) { return String(Math.round(n)); } };
    this.statsBox.textContent = "";
    [["triangles", tri], ["vertices", vert]].forEach(function (it) {
      var name = word(self.el, it[0]);
      var one = document.createElement("span");
      one.setAttribute("data-rv-stat", it[0]);
      one.title = name;
      one.setAttribute("role", "img");
      one.setAttribute("aria-label", name + " : " + fmt(it[1]));
      one.innerHTML = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">' + STAT_ICONS[it[0]] + "</svg>";
      var n = document.createElement("span");
      n.textContent = fmt(it[1]);
      one.appendChild(n);
      self.statsBox.appendChild(one);
    });
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
    this.strip = null;
    this.stripLive = false;
    this.cuts = null;
    this.single = null;
    this.layerMarks();
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
    this.markEls = null;
    this.statsBox = null;
    this.bands = null;
    this.cuts = null;
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
