//! Défilement (D-110) : défilement doux du site et barres de progression.
//!
//! - **Défilement doux** (`settings.smooth_scroll`, comme Lenis) : la molette fixe une cible ;
//!   la page, ou le conteneur qui défile sous le pointeur (colonnes à défilement propre), glisse
//!   vers elle image après image. Clavier, barre de défilement et toucher gardent leur
//!   comportement ; un élément `data-revel-no-smooth` (carte, liste à défilement propre) est
//!   laissé au navigateur ; coupé pour les visiteurs qui demandent moins de mouvement.
//! - **Barre de progression** (`data-revel-progress`) : la variable CSS `--revel-progress` de
//!   l'élément suit, de 0 à 1, le défilement de la page (`page`) ou du premier conteneur qui
//!   défile autour de lui (`parent`, par défaut ; la page s'il n'y en a pas). Le remplissage
//!   s'en sert dans son style (`width: calc(var(--revel-progress, 0) * 100%)`).
//!
//! - **Section affichée** (`data-revel-spy`, D-111) : dans l'élément (navigation), le lien vers
//!   la section de la page en cours de lecture reçoit la classe `is-active` (état « Actif » des
//!   styles), et l'élément l'attribut `data-revel-section="<id de la section>"`. La section en
//!   cours est la dernière dont le haut a passé les 40 % de la fenêtre (la première sinon) ;
//!   suit aussi les colonnes à défilement propre.
//!
//! Scripts courts, écrits dans la page ; pas dans le canevas de l'éditeur (barre à 0).

/// Script du défilement doux.
const SMOOTH: &str = concat!(
    "<style>html.rv-smooth,html.rv-smooth *{scroll-behavior:auto!important}</style>",
    "<script>(function(w,d){if(w.matchMedia&&matchMedia(\"(prefers-reduced-motion: reduce)\").matches)return;",
    "d.documentElement.classList.add(\"rv-smooth\");var M=new Map(),run=false;",
    "function can(e,y){var o=getComputedStyle(e).overflowY;return(o===\"auto\"||o===\"scroll\")&&e.scrollHeight>e.clientHeight+1&&(y>0?e.scrollTop+e.clientHeight<e.scrollHeight-1:e.scrollTop>0)}",
    "function find(t,y){for(var e=t&&t.nodeType===1?t:t&&t.parentElement;e&&e!==d.body&&e!==d.documentElement;e=e.parentElement){",
    "if(e.hasAttribute(\"data-revel-no-smooth\")||/^(TEXTAREA|SELECT)$/.test(e.tagName))return null;if(can(e,y))return e}",
    "return d.scrollingElement||d.documentElement}",
    "function step(){M.forEach(function(s,e){var c=e.scrollTop;if(Math.abs(c-s.last)>1){M.delete(e);return}",
    "var n=c+(s.t-c)*.12;if(Math.abs(s.t-n)<.5)n=s.t;e.scrollTop=n;s.last=e.scrollTop;",
    "if(Math.abs(s.t-s.last)<.5||s.last===c)M.delete(e)});",
    "if(M.size)requestAnimationFrame(step);else run=false}",
    "w.addEventListener(\"wheel\",function(v){if(v.ctrlKey||v.defaultPrevented||Math.abs(v.deltaX)>Math.abs(v.deltaY))return;",
    "var y=v.deltaY*(v.deltaMode===1?16:v.deltaMode===2?innerHeight:1),e=find(v.target,y);if(!e)return;",
    "var s=M.get(e);if(!s){s={t:e.scrollTop,last:e.scrollTop};M.set(e,s)}",
    "s.t=Math.max(0,Math.min(e.scrollHeight-e.clientHeight,s.t+y));v.preventDefault();",
    "if(!run){run=true;requestAnimationFrame(step)}},{passive:false})})(window,document)</script>",
);

/// Script des barres de progression.
const PROGRESS: &str = concat!(
    "<script>(function(w,d){function box(el){if(el.getAttribute(\"data-revel-progress\")!==\"page\")",
    "for(var e=el.parentElement;e&&e!==d.body;e=e.parentElement){var o=getComputedStyle(e).overflowY;",
    "if(o===\"auto\"||o===\"scroll\")return e}return null}",
    "function init(){[].forEach.call(d.querySelectorAll(\"[data-revel-progress]\"),function(el){",
    "var c=box(el),p=-1,q=false;function u(){q=false;var r=d.documentElement,",
    "top=c?c.scrollTop:w.scrollY,max=c?c.scrollHeight-c.clientHeight:r.scrollHeight-w.innerHeight,",
    "v=max>0?Math.min(1,Math.max(0,top/max)):0;if(v!==p){p=v;el.style.setProperty(\"--revel-progress\",v.toFixed(4))}}",
    "function o(){if(!q){q=true;requestAnimationFrame(u)}}(c||w).addEventListener(\"scroll\",o,{passive:true});",
    "w.addEventListener(\"resize\",o);u()})}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",init);else init()})(window,document)</script>",
);

/// Script des navigations qui suivent la section affichée.
const SPY: &str = concat!(
    "<script>(function(w,d){function init(){[].forEach.call(d.querySelectorAll(\"[data-revel-spy]\"),function(el){",
    "var L=[].filter.call(el.querySelectorAll(\"a[href*='#']\"),function(a){var u=new URL(a.href,location.href);",
    "return u.pathname===location.pathname&&u.hash.length>1&&d.getElementById(decodeURIComponent(u.hash.slice(1)))});",
    "if(!L.length)return;var S=L.map(function(a){return d.getElementById(decodeURIComponent(new URL(a.href,location.href).hash.slice(1)))}),",
    "cur=null,q=false;function u(){q=false;var i=0,lim=w.innerHeight*.4;S.forEach(function(s,k){var r=s.getBoundingClientRect();",
    "if(r.height&&r.top<=lim)i=k});if(i===cur)return;cur=i;L.forEach(function(a,k){a.classList.toggle(\"is-active\",S[k]===S[i])});",
    "el.setAttribute(\"data-revel-section\",S[i].id)}function o(){if(!q){q=true;requestAnimationFrame(u)}}",
    "d.addEventListener(\"scroll\",o,{passive:true,capture:true});w.addEventListener(\"resize\",o);u()})}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",init);else init()})(window,document)</script>",
);

/// La page contient une navigation qui suit la section affichée.
pub fn uses_spy(html: &str) -> bool {
    html.contains(" data-revel-spy")
}

/// La page contient une barre de progression.
pub fn uses_progress(html: &str) -> bool {
    html.contains(" data-revel-progress")
}

/// Page du site : scripts du défilement doux (`smooth`) et des barres de progression ajoutés
/// avant `</head>` (pas dans le canevas : `scripts` faux).
pub fn wire(html: String, smooth: bool, scripts: bool) -> String {
    if !scripts {
        return html;
    }
    let mut code = String::new();
    if smooth {
        code.push_str(SMOOTH);
    }
    if uses_progress(&html) {
        code.push_str(PROGRESS);
    }
    if uses_spy(&html) {
        code.push_str(SPY);
    }
    if code.is_empty() {
        html
    } else {
        crate::insert_before(&html, "</head>", &code, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defilement_doux_et_progression() {
        let page = "<html><head></head><body><p>a</p></body></html>".to_string();
        assert_eq!(wire(page.clone(), false, true), page);
        let out = wire(page.clone(), true, true);
        assert!(out.contains("prefers-reduced-motion") && out.contains("\"wheel\""));
        assert!(!out.contains("--revel-progress"));
        assert_eq!(wire(page.clone(), true, false), page);
        let bar = "<html><head></head><body><div data-revel-progress=\"parent\"><div></div></div></body></html>"
            .to_string();
        let out = wire(bar.clone(), false, true);
        assert!(out.contains("--revel-progress") && !out.contains("\"wheel\""));
        assert!(out.find("<script>").unwrap() < out.find("</head>").unwrap());
        assert_eq!(wire(bar.clone(), true, false), bar);
        let nav = "<html><head></head><body><nav data-revel-spy=\"section\"><a href=\"#a\">A</a></nav><section id=\"a\"></section></body></html>"
            .to_string();
        let out = wire(nav.clone(), false, true);
        assert!(
            out.contains("data-revel-section")
                && out.contains("is-active")
                && !out.contains("--revel-progress")
        );
        assert_eq!(wire(nav.clone(), false, false), nav);
    }
}
