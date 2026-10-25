//! Affichage progressif des listes (D-114) : un élément `data-revel-more="6"` montre ses 6
//! premiers éléments ; quand le bas de la liste arrive à l'écran (dans la page ou dans la colonne
//! qui défile), un indicateur de chargement tourne un instant puis les 6 suivants apparaissent,
//! et ainsi de suite. Sans script, tout est montré ; le canevas de l'éditeur montre tout.

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
    "if(items.length<=n)return;var shown=n,busy=false,q=false;",
    "items.forEach(function(c,i){if(i>=n)c.classList.add(\"rv-more-wait\")});",
    "var slow=!(w.matchMedia&&matchMedia(\"(prefers-reduced-motion: reduce)\").matches);",
    // Bas de la liste à l'écran (dans la page ou dans la colonne qui défile) : la suite.
    "function check(){q=false;if(busy||shown>=items.length)return;var c=sc(box),lim=w.innerHeight;",
    "if(c)lim=Math.min(lim,c.getBoundingClientRect().bottom);if(box.getBoundingClientRect().bottom>lim+60)return;busy=true;",
    "var spin=null;if(slow){spin=d.createElement(\"div\");spin.className=\"rv-more-loading\";spin.setAttribute(\"aria-hidden\",\"true\");",
    "box.parentNode.insertBefore(spin,box.nextSibling)}",
    "setTimeout(function(){if(spin)spin.remove();items.slice(shown,shown+n).forEach(function(c){c.classList.remove(\"rv-more-wait\")});",
    "shown+=n;busy=false;ask()},slow?450:0)}",
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
