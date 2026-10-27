//! Taille du texte ajustée à l'écran (D-153) : unité « écran » de Styles › Texte › Taille. La
//! règle porte `--revel-fit: 42` (part de la hauteur de l'écran, en %, que le texte occupe) ; la
//! taille écrite à côté (`font-size`) reste celle d'un navigateur sans script. Le script calcule la
//! taille qui donne au texte cette hauteur, sans déborder en largeur, entre 12 et 600 px ; il suit
//! les points de rupture (valeur lue dans les styles calculés : `0` l'arrête), la fenêtre, les
//! polices et le texte. Il lit les règles dans les feuilles de la page : dans le canevas, une valeur
//! réglée dans le panneau s'applique aussitôt.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){var P=\"--revel-fit\",F=[];",
    "function sels(){var o=[];function walk(l){for(var i=0;i<l.length;i++){var r=l[i];",
    "if(r.style&&r.selectorText&&r.style.getPropertyValue(P))o.push(r.selectorText);if(r.cssRules)walk(r.cssRules)}}",
    "for(var s=0;s<d.styleSheets.length;s++){try{walk(d.styleSheets[s].cssRules)}catch(e){}}return o}",
    // Hauteur de l'écran : celle du canevas quand il est allongé (`--rv-vh`), sinon la fenêtre.
    "function screenH(){var v=parseFloat(getComputedStyle(d.documentElement).getPropertyValue(\"--rv-vh\"));return v>0?v*100:w.innerHeight}",
    "function fit(e,H){e.style.fontSize=\"\";e.__rvFit=\"\";var h=parseFloat(getComputedStyle(e).getPropertyValue(P));if(!(h>0))return false;",
    // Transition de la taille coupée pendant la mesure (sinon la hauteur lue serait l'ancienne).
    "var lim=H*h/100,lo=12,hi=600,t=e.style.transition;e.style.transition=\"none\";for(var i=0;i<14;i++){var m=(lo+hi)/2;e.style.fontSize=m+\"px\";",
    "if(e.offsetHeight<=lim&&e.scrollWidth<=e.clientWidth+1)lo=m;else hi=m}e.style.fontSize=e.__rvFit=Math.floor(lo*10)/10+\"px\";e.offsetHeight;e.style.transition=t;return true}",
    // Éléments ajustés auparavant et qui ne le sont plus (valeur retirée) : taille écrite rendue.
    "function all(){var H=screenH(),done=[];sels().forEach(function(s){var l;try{l=d.querySelectorAll(s)}catch(e){return}",
    "for(var i=0;i<l.length;i++)if(done.indexOf(l[i])<0&&fit(l[i],H))done.push(l[i])});",
    "F.forEach(function(e){if(done.indexOf(e)<0&&e.__rvFit){if(e.style.fontSize===e.__rvFit)e.style.fontSize=\"\";e.__rvFit=\"\";if(RO)RO.unobserve(e)}});",
    "F=done;if(RO)done.forEach(function(e){RO.observe(e)})}",
    // Taille de l'élément changée sans mutation (feuille du canevas, police, largeur) : nouveau calcul.
    "var q=0,RO=w.ResizeObserver?new ResizeObserver(function(){soon()}):null;function soon(){if(q)return;q=1;(w.requestAnimationFrame||setTimeout)(function(){q=0;all()})}",
    "w.RevelFit=all;d.addEventListener(\"DOMContentLoaded\",all);w.addEventListener(\"load\",all);w.addEventListener(\"resize\",soon);",
    "if(d.fonts&&d.fonts.ready)d.fonts.ready.then(soon);",
    // Attribut `style` : ignoré quand c'est la taille écrite ici ; remis à zéro ailleurs (mise à
    // jour du canevas) : nouveau calcul.
    "if(w.MutationObserver)new MutationObserver(function(m){for(var i=0;i<m.length;i++){var r=m[i];",
    "if(r.type===\"attributes\"&&r.attributeName===\"style\"&&(!r.target.__rvFit||r.target.style.fontSize===r.target.__rvFit))continue;soon();return}})",
    ".observe(d.documentElement,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:[\"class\",\"style\"]})",
    "})(window,document)</script>",
);

/// Propriété de Revel portée par la règle.
pub const PROP: &str = "--revel-fit";

/// Le site règle une taille ajustée à l'écran.
pub fn site_uses(site: &revel_model::Site) -> bool {
    site.style_rules()
        .any(|r| r.decls.iter().any(|d| d.prop == PROP))
}

/// Script ajouté juste avant `</head>` aux pages du site qui en ont besoin.
pub fn wire(html: String, on: bool) -> String {
    if on {
        crate::insert_before(&html, "</head>", SCRIPT, false)
    } else {
        html
    }
}

/// Canevas : le script toujours présent (une valeur réglée dans le panneau s'y voit sans
/// recharger) ; marqué quand le site ne l'utilise pas encore, pour que la page sans les repères de
/// l'éditeur reste celle du site.
pub fn wire_editor(html: String, site_uses: bool) -> String {
    if site_uses {
        return wire(html, true);
    }
    let marked = SCRIPT.replacen("<script>", "<script data-rv-tool>", 1);
    crate::insert_before(&html, "</head>", &marked, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taille_ajustee_a_l_ecran() {
        let page = "<html><head></head><body><h2 class=\"t\">Titre</h2></body></html>".to_string();
        let out = wire(page.clone(), true);
        assert!(
            out.contains("RevelFit")
                && out.find("<script>").unwrap() < out.find("</head>").unwrap()
        );
        assert_eq!(wire(page.clone(), false), page);
        let mut site = revel_model::Site::new("x", "fr");
        assert!(!site_uses(&site));
        site.stylesheet = Some(revel_model::Stylesheet {
            url: "/s.css".into(),
            source: None,
            items: vec![revel_model::StyleItem::Rule(revel_model::StyleRule {
                id: None,
                selector: ".t".into(),
                media: None,
                decls: vec![revel_model::Decl {
                    prop: PROP.into(),
                    value: "42".into(),
                    important: false,
                }],
            })],
        });
        assert!(site_uses(&site));
    }
}
