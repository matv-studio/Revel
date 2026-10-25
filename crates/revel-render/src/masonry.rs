//! Maçonnerie (D-110) : grille dont chaque élément se place sous la colonne la moins haute
//! (`data-revel-masonry`), ou dans les colonnes à tour de rôle (`data-revel-masonry="order"` :
//! colonnes de Framer, élément 1 à gauche, 2 à droite, 3 à gauche…).
//!
//! Une grille `data-revel-masonry` garde ses colonnes (réglées par point de rupture dans le
//! panneau Disposition) ; le script lui donne des lignes de 1 px et à chaque élément autant de
//! lignes que sa hauteur, écart compris, et sa place (colonne, première ligne) ; l'ordre de la
//! page reste celui des lecteurs d'écran et du clavier.
//! Refait quand un élément change de taille (image chargée, largeur de la fenêtre). Sans script,
//! ou quand l'élément n'est plus une grille (autre point de rupture), grille ordinaire.
//! Écrit aussi dans le canevas : c'est une mise en page, pas une animation.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){var Q=new Set(),q=false,G=new Set(),R=w.ResizeObserver?new ResizeObserver(function(es){es.forEach(function(e){var g=e.target.hasAttribute(\"data-revel-masonry\")?e.target:e.target.parentElement;if(g)ask(g)})}):null;",
    "function reset(g,kids){g.style.gridAutoRows=g.style.rowGap=\"\";kids.forEach(function(c){c.style.gridRow=c.style.gridColumn=c.style.alignSelf=\"\"});delete g.__rvm}",
    "function lay(g){var s=getComputedStyle(g),kids=[].filter.call(g.children,function(c){return c.nodeType===1&&getComputedStyle(c).display!==\"none\"});",
    "if(!g.hasAttribute(\"data-revel-masonry\")||s.display.indexOf(\"grid\")<0){if(g.__rvm!==undefined)reset(g,kids);return}",
    "if(g.__rvm===undefined){g.__rvm=parseFloat(s.rowGap)||0;g.style.gridAutoRows=\"1px\";g.style.rowGap=\"0px\"}",
    "var n=Math.max(1,s.gridTemplateColumns.split(\" \").filter(Boolean).length),H=[],i,o=g.getAttribute(\"data-revel-masonry\")===\"order\";",
    "var L=[];for(i=0;i<n;i++)H.push(0);",
    "kids.forEach(function(c){c.style.alignSelf=\"start\"});",
    "kids.forEach(function(c,k){var col=0;if(o)col=k%n;else for(i=1;i<n;i++)if(H[i]<H[col])col=i;",
    "var h=c.getBoundingClientRect().height,sp=Math.max(1,Math.round(h+g.__rvm));",
    "c.style.gridColumn=String(col+1);c.style.gridRow=(H[col]+1)+\" / span \"+sp;L[col]=[c,H[col],h];H[col]+=sp});",
    // Dernier élément de chaque colonne : sans l'écart qui le suit (hauteur d'une grille ordinaire).
    "L.forEach(function(x){if(x)x[0].style.gridRow=(x[1]+1)+\" / span \"+Math.max(1,Math.round(x[2]))})}",
    "function run(){q=false;Q.forEach(lay);Q.clear()}function ask(g){Q.add(g);if(!q){q=true;requestAnimationFrame(run)}}",
    // Grilles ajoutées, retirées ou réglées après le chargement (canevas de l'éditeur) : suivies.
    "function scan(){G.forEach(function(g){if(!g.isConnected||!g.hasAttribute(\"data-revel-masonry\")){G.delete(g);if(g.isConnected)lay(g)}});",
    "[].forEach.call(d.querySelectorAll(\"[data-revel-masonry]\"),function(g){G.add(g);",
    "if(R){R.observe(g);[].forEach.call(g.children,function(c){R.observe(c)})}ask(g)})}",
    "function init(){scan();if(!R)w.addEventListener(\"resize\",function(){G.forEach(ask)});",
    "if(w.MutationObserver)new MutationObserver(scan).observe(d.documentElement,{subtree:true,childList:true,attributes:true,attributeFilter:[\"data-revel-masonry\"]})}",
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
        assert!(
            out.contains("gridAutoRows")
                && out.contains("ResizeObserver")
                && out.contains("MutationObserver")
        );
        assert!(out.find("<script>").unwrap() < out.find("</head>").unwrap());
        let plain =
            "<html><head></head><body><div class=\"grille\"></div></body></html>".to_string();
        assert_eq!(wire(plain.clone()), plain);
    }
}
