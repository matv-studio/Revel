//! Texte ajusté à l'écran (D-153) : un élément `data-revel-fit="h=55 min=28"` garde sa taille de
//! texte (réglée dans Styles) tant qu'il tient dans `h` % de la hauteur de l'écran ; plus long ou
//! sur un écran plus bas, sa taille diminue jusqu'à ce qu'il tienne, sans descendre sous `min`
//! pixels. Recalculé quand la fenêtre, les polices ou le texte changent. Le script, court, est
//! placé à la fin de `<head>`, sur le site comme dans le canevas de l'éditeur (le réglage se voit
//! en le faisant) ; il s'exécute avant les animations de texte, qui découpent les lignes ajustées.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){var S=\"[data-revel-fit]\";",
    "function opt(e,k,v){var m=(e.getAttribute(\"data-revel-fit\")||\"\").match(new RegExp(\"(?:^|\\\\s)\"+k+\"=([0-9.]+)\"));return m?parseFloat(m[1]):v}",
    "function fit(e){var h=opt(e,\"h\",60),mn=opt(e,\"min\",16);e.style.fontSize=\"\";",
    "var max=parseFloat(getComputedStyle(e).fontSize)||16,lim=w.innerHeight*h/100;",
    "if(e.offsetHeight<=lim||max<=mn)return;var lo=mn,hi=max;",
    "for(var i=0;i<9;i++){var mid=(lo+hi)/2;e.style.fontSize=mid+\"px\";if(e.offsetHeight<=lim)lo=mid;else hi=mid}",
    "e.style.fontSize=Math.max(mn,lo).toFixed(1)+\"px\"}",
    "function all(){var l=d.querySelectorAll(S);for(var i=0;i<l.length;i++)fit(l[i])}",
    "var q=0;function soon(){if(q)return;q=1;(w.requestAnimationFrame||setTimeout)(function(){q=0;all()})}",
    "w.RevelFit=all;d.addEventListener(\"DOMContentLoaded\",all);w.addEventListener(\"load\",all);w.addEventListener(\"resize\",soon);",
    "if(d.fonts&&d.fonts.ready)d.fonts.ready.then(soon);",
    "if(w.MutationObserver)new MutationObserver(function(m){for(var i=0;i<m.length;i++){var t=m[i].target;",
    "if(t&&t.nodeType!==1)t=t.parentNode;if(t&&t.closest&&t.closest(S)&&!(m[i].type===\"attributes\"&&m[i].attributeName===\"style\")){soon();return}",
    "if(m[i].type===\"childList\"){soon();return}}}).observe(d.documentElement,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:[\"data-revel-fit\",\"class\"]})",
    "})(window,document)</script>",
);

/// La page contient un texte ajusté à l'écran.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-fit=")
}

/// Script ajouté juste avant `</head>` (site et canevas : le texte s'y ajuste aussi).
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
    fn texte_ajuste_a_l_ecran() {
        let page =
            "<html><head></head><body><h2 data-revel-fit=\"h=50 min=24\">Titre</h2></body></html>"
                .to_string();
        let out = wire(page);
        assert!(
            out.contains("RevelFit")
                && out.find("<script>").unwrap() < out.find("</head>").unwrap()
        );
        let plain = "<html><head></head><body></body></html>".to_string();
        assert_eq!(wire(plain.clone()), plain);
    }
}
