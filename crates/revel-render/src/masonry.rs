//! Maçonnerie (D-110) : grille dont chaque élément se place sous le plus court de sa colonne,
//! comme le Masonry de Framer.
//!
//! Une grille `data-revel-masonry` garde ses colonnes (réglées par point de rupture dans le
//! panneau Disposition) ; le script lui donne des lignes de 1 px et à chaque élément autant de
//! lignes que sa hauteur, écart compris : le placement automatique de la grille met alors chaque
//! élément dans la colonne la moins haute, dans l'ordre de la page (lecteurs d'écran, clavier).
//! Refait quand un élément change de taille (image chargée, largeur de la fenêtre). Sans script,
//! ou quand l'élément n'est plus une grille (autre point de rupture), grille ordinaire.
//! Écrit aussi dans le canevas : c'est une mise en page, pas une animation.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){var Q=new Set(),q=false;",
    "function lay(g){var s=getComputedStyle(g),kids=[].filter.call(g.children,function(c){return c.nodeType===1});",
    "if(s.display.indexOf(\"grid\")<0){if(g.__rvm!==undefined){g.style.gridAutoRows=g.style.rowGap=\"\";",
    "kids.forEach(function(c){c.style.gridRowEnd=c.style.alignSelf=\"\"});delete g.__rvm}return}",
    "if(g.__rvm===undefined){g.__rvm=parseFloat(s.rowGap)||0;g.style.gridAutoRows=\"1px\";g.style.rowGap=\"0px\"}",
    "kids.forEach(function(c){c.style.alignSelf=\"start\"});",
    "kids.forEach(function(c){c.style.gridRowEnd=\"span \"+Math.max(1,Math.round(c.getBoundingClientRect().height+g.__rvm))})}",
    "function run(){q=false;Q.forEach(lay);Q.clear()}function ask(g){Q.add(g);if(!q){q=true;requestAnimationFrame(run)}}",
    "function init(){[].forEach.call(d.querySelectorAll(\"[data-revel-masonry]\"),function(g){lay(g);",
    "if(w.ResizeObserver){var o=new ResizeObserver(function(){ask(g)});o.observe(g);",
    "[].forEach.call(g.children,function(c){o.observe(c)})}else w.addEventListener(\"resize\",function(){ask(g)})})}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",init);else init()})(window,document)</script>",
);

/// La page contient une grille en maçonnerie.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-masonry")
}

/// Page (site, aperçu ou canevas) : script ajouté avant `</head>` si elle a une maçonnerie.
pub fn wire(html: String) -> String {
    if uses(&html) {
        crate::insert_before(&html, "</head>", SCRIPT, false)
    } else {
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maconnerie() {
        let page = "<html><head></head><body><div data-revel-masonry><p>a</p></div></body></html>"
            .to_string();
        let out = wire(page);
        assert!(out.contains("gridAutoRows") && out.contains("ResizeObserver"));
        assert!(out.find("<script>").unwrap() < out.find("</head>").unwrap());
        let plain =
            "<html><head></head><body><div class=\"grille\"></div></body></html>".to_string();
        assert_eq!(wire(plain.clone()), plain);
    }
}
