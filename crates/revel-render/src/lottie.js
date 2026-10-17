/* Animations Lottie du site (6.2) : chaque élément `data-lottie="/media/x.json"` reçoit son
   animation, lue par le lecteur lottie-web (licence MIT, hébergé avec le site, aucun service
   extérieur). Réglages portés par l'élément :
   - data-lottie-play : auto (à l'écran, en pause hors de l'écran), hover (survol : avance,
     revient en arrière en sortant), click (rejouée à chaque appui), scroll (suit le défilement) ;
   - data-lottie-loop : en boucle ;
   - data-lottie-speed : vitesse (1 par défaut).
   Mouvement réduit demandé par le visiteur : première image, sans lecture automatique. */
(function () {
  var L = window.lottie;
  if (!L) return;
  var reduce = !!(window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  function setup(el) {
    if (el.__rvLottie) return;
    var src = el.getAttribute("data-lottie");
    if (!src) return;
    var play = el.getAttribute("data-lottie-play") || "auto";
    var loop = el.hasAttribute("data-lottie-loop");
    var speed = parseFloat(el.getAttribute("data-lottie-speed")) || 1;
    var anim = L.loadAnimation({
      container: el,
      renderer: "svg",
      loop: loop && play !== "scroll",
      autoplay: false,
      path: src,
      rendererSettings: { preserveAspectRatio: "xMidYMid meet" }
    });
    el.__rvLottie = anim;
    anim.setSpeed(speed);
    anim.addEventListener("DOMLoaded", function () {
      // Sans hauteur donnée par le style : proportions de l'animation.
      var d = anim.animationData;
      if (d && d.w && d.h && el.clientHeight < 2) el.style.aspectRatio = d.w + " / " + d.h;
      if (play === "scroll") onScroll();
    });
    if (play === "hover") {
      el.addEventListener("mouseenter", function () { anim.setDirection(1); anim.play(); });
      el.addEventListener("mouseleave", function () {
        if (loop) anim.pause();
        else { anim.setDirection(-1); anim.play(); }
      });
    } else if (play === "click") {
      el.style.cursor = "pointer";
      el.addEventListener("click", function () { anim.setDirection(1); anim.goToAndPlay(0, true); });
    } else if (play === "scroll") {
      var onScroll = function () {
        if (!anim.totalFrames) return;
        var r = el.getBoundingClientRect();
        var h = window.innerHeight || 1;
        var p = (h - r.top) / (h + r.height);
        p = p < 0 ? 0 : p > 1 ? 1 : p;
        anim.goToAndStop(p * (anim.totalFrames - 1), true);
      };
      window.addEventListener("scroll", onScroll, { passive: true });
      window.addEventListener("resize", onScroll);
    } else if (!reduce) {
      // À l'écran : lecture ; hors de l'écran : pause.
      if ("IntersectionObserver" in window) {
        new IntersectionObserver(function (es) {
          es.forEach(function (e) { if (e.isIntersecting) anim.play(); else anim.pause(); });
        }, { threshold: 0.1 }).observe(el);
      } else anim.play();
    }
  }
  function all() {
    var els = document.querySelectorAll("[data-lottie]");
    for (var i = 0; i < els.length; i++) setup(els[i]);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", all);
  else all();
  window.RevelLottie = { refresh: all };
})();
