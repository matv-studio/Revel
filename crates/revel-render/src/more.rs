//! Affichage progressif des listes (D-114) : un élément `data-revel-more="6"` montre ses 6
//! premiers éléments ; quand le bas de la liste arrive à l'écran (dans la page ou dans la colonne
//! qui défile), un indicateur de chargement tourne un instant puis les 6 suivants apparaissent,
//! et ainsi de suite. Sans script, tout est montré ; le canevas de l'éditeur montre tout.
//! Liste filtrée (`data-revel-filters`) : seuls les éléments gardés par le filtre comptent, et la
//! liste repart de ses premiers éléments à chaque changement de filtre. Bouton « Afficher plus »
//! (D-168) : un élément dont l'action est `more` sur la liste la fait avancer à l'appui (plus au
//! défilement) et disparaît quand tout est montré.

const SCRIPT: &str = concat!(
    "<style>.rv-more-wait{display:none!important}",
    ".rv-more-loading{display:flex;justify-content:center;padding:16px 0;grid-column:1/-1}",
    ".rv-more-loading::before{content:\"\";width:20px;height:20px;border-radius:50%;border:2px solid currentColor;",
    "border-right-color:transparent;opacity:.45;animation:rv-more-spin .7s linear infinite}",
    "@keyframes rv-more-spin{to{transform:rotate(1turn)}}",
    "@media (prefers-reduced-motion: reduce){.rv-more-loading::before{animation:none}}</style>",
    "<script>(function(w,d){function sc(el){for(var e=el.parentElement;e&&e!==d.body;e=e.parentElement){var o=getComputedStyle(e).overflowY;",
    "if((o===\"auto\"||o===\"scroll\")&&e.scrollHeight>e.clientHeight+1)return e}return null}",
    "function init(){[].forEach.call(d.querySelectorAll(\"[data-revel-more]\"),function(box){",
    "var n=parseInt(box.getAttribute(\"data-revel-more\"),10);if(!(n>0))return;",
    "var items=[].filter.call(box.children,function(c){return c.nodeType===1&&!/^(SCRIPT|TEMPLATE|STYLE)$/.test(c.tagName)});",
    "var shown=n,busy=false,q=false;",
    // Éléments gardés par les filtres de la liste (`rv-hide` : écartés) : les `shown` premiers
    // sont montrés ; le reste à venir est annoncé au bouton « Afficher la suite » (action `more`).
    "function live(){return items.filter(function(c){return !c.classList.contains(\"rv-hide\")})}",
    "function paint(){var l=live();items.forEach(function(c){c.classList.toggle(\"rv-more-wait\",l.indexOf(c)>=shown)});",
    "box.__rvMoreRest=Math.max(0,l.length-shown);box.dispatchEvent(new CustomEvent(\"rv-more-rest\"))}",
    "paint();",
    // Filtre changé : la liste repart de ses premiers éléments.
    "if(w.MutationObserver)new MutationObserver(function(ms){for(var i=0;i<ms.length;i++){var t=ms[i].target;",
    "if(t.parentNode===box&&((\" \"+(ms[i].oldValue||\"\")+\" \").indexOf(\" rv-hide \")>=0)!==t.classList.contains(\"rv-hide\")){shown=n;paint();ask();return}}})",
    ".observe(box,{subtree:true,attributes:true,attributeFilter:[\"class\"],attributeOldValue:true});",
    "box.addEventListener(\"rv-more\",function(){shown+=n;paint()});",
    "var slow=!(w.matchMedia&&matchMedia(\"(prefers-reduced-motion: reduce)\").matches);",
    // Bas de la liste à l'écran (dans la page ou dans la colonne qui défile) : la suite, sauf
    // liste reliée à un bouton (`data-rv-more-manual`, posé par l'action `more`).
    "function check(){q=false;if(busy||box.hasAttribute(\"data-rv-more-manual\")||shown>=live().length)return;var c=sc(box),lim=w.innerHeight;",
    "if(c)lim=Math.min(lim,c.getBoundingClientRect().bottom);if(box.getBoundingClientRect().bottom>lim+60)return;busy=true;",
    "var spin=null;if(slow){spin=d.createElement(\"div\");spin.className=\"rv-more-loading\";spin.setAttribute(\"aria-hidden\",\"true\");",
    "box.parentNode.insertBefore(spin,box.nextSibling)}",
    "setTimeout(function(){if(spin)spin.remove();shown+=n;paint();busy=false;ask()},slow?450:0)}",
    "function ask(){if(!q){q=true;requestAnimationFrame(check)}}",
    "d.addEventListener(\"scroll\",ask,{passive:true,capture:true});w.addEventListener(\"resize\",ask);ask()})}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",init);else init()})(window,document)</script>",
);

/// La page contient une liste à affichage progressif.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-more=")
}

/// Page du site : script ajouté avant `</head>` (pas dans le canevas : `scripts` faux).
pub fn wire(html: String, scripts: bool) -> String {
    if scripts && uses(&html) {
        crate::insert_before(&html, "</head>", SCRIPT, false)
    } else {
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affichage_progressif() {
        let page =
            "<html><head></head><body><div data-revel-more=\"6\"><p>a</p></div></body></html>"
                .to_string();
        let out = wire(page.clone(), true);
        assert!(out.contains("rv-more-wait") && out.contains("rv-more-loading"));
        assert!(out.find("<script>").unwrap() < out.find("</head>").unwrap());
        assert_eq!(wire(page.clone(), false), page);
        let plain = "<html><head></head><body></body></html>".to_string();
        assert_eq!(wire(plain.clone(), true), plain);
    }
}
