//! Interrupteur clair / sombre du site (D-149) : un élément `data-revel-switch="theme"` bascule le
//! site entre ses couleurs claires et sombres (réglées dans Paramètres › Couleurs, écrites dans
//! `@media (prefers-color-scheme: dark)`), choix gardé par le navigateur ; sans choix, le site
//! suit le thème de l'appareil. Le script, placé à la fin de `<head>` (après les feuilles de
//! style), réécrit les requêtes `prefers-color-scheme` des feuilles avant le premier affichage :
//! pas de flash du mauvais thème. `html` reçoit `data-revel-scheme="dark|light"` ; l'interrupteur
//! `aria-pressed="true"` et la classe `is-active` en mode sombre, son texte
//! `data-revel-switch-text="Sombre|Clair"`.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){var K=\"rv-theme\",m=w.matchMedia?w.matchMedia(\"(prefers-color-scheme: dark)\"):null,v=null;",
    "try{v=w.localStorage.getItem(K)}catch(e){}if(v!==\"dark\"&&v!==\"light\")v=null;",
    "var T=\"(min-width: 0px)\",F=\"(max-width: 0px)\",R=/\\(\\s*prefers-color-scheme\\s*:\\s*(dark|light)\\s*\\)/g;",
    "function each(l,f){for(var i=0;i<l.length;i++){var r=l[i];if(r.media&&r.cssRules)f(r);if(r.cssRules)each(r.cssRules,f)}}",
    "function apply(){var dk=v?v===\"dark\":!!(m&&m.matches),h=d.documentElement;",
    "h.setAttribute(\"data-revel-scheme\",dk?\"dark\":\"light\");h.style.colorScheme=dk?\"dark\":\"light\";",
    "for(var s=0;s<d.styleSheets.length;s++){var l;try{l=d.styleSheets[s].cssRules}catch(e){continue}",
    "each(l,function(r){var t=r.__rv;if(t===undefined){t=r.media.mediaText;if(!/prefers-color-scheme/.test(t))return;r.__rv=t}",
    "var n=v?t.replace(R,function(_,x){return x===v?T:F}):t;if(r.media.mediaText!==n)r.media.mediaText=n})}",
    "var b=d.querySelectorAll(\"[data-revel-switch=\\\"theme\\\"]\");for(var i=0;i<b.length;i++){",
    "b[i].setAttribute(\"aria-pressed\",String(dk));b[i].classList.toggle(\"is-active\",dk);",
    "var x=b[i].querySelectorAll(\"[data-revel-switch-text]\");for(var j=0;j<x.length;j++){",
    "var p=(x[j].getAttribute(\"data-revel-switch-text\")||\"On|Off\").split(\"|\");x[j].textContent=dk?p[0]:(p[1]||\"\")}}}",
    "w.RevelTheme={set:function(x){v=x===\"dark\"||x===\"light\"?x:null;try{if(v)w.localStorage.setItem(K,v);",
    "else w.localStorage.removeItem(K)}catch(e){}apply()},get:function(){return d.documentElement.getAttribute(\"data-revel-scheme\")}};",
    "d.addEventListener(\"click\",function(e){var b=e.target.closest&&e.target.closest(\"[data-revel-switch=\\\"theme\\\"]\");",
    "if(!b)return;e.preventDefault();w.RevelTheme.set(w.RevelTheme.get()===\"dark\"?\"light\":\"dark\")});",
    "if(m){var c=function(){if(!v)apply()};if(m.addEventListener)m.addEventListener(\"change\",c);else if(m.addListener)m.addListener(c)}",
    "apply();d.addEventListener(\"DOMContentLoaded\",apply);w.addEventListener(\"load\",apply)})(window,document)</script>",
);

/// La page contient un interrupteur clair / sombre.
pub fn uses(html: &str) -> bool {
    html.contains("data-revel-switch=\"theme\"")
}

/// Page du site : script ajouté juste avant `</head>` (pas dans le canevas : `scripts` faux).
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
    fn interrupteur_clair_sombre() {
        let page = "<html><head><link rel=\"stylesheet\" href=\"/a.css\"></head><body><button data-revel-switch=\"theme\">x</button></body></html>".to_string();
        let out = wire(page.clone(), true);
        let s = out.find("<script>").unwrap();
        assert!(out.find("a.css").unwrap() < s && s < out.find("</head>").unwrap());
        assert!(out.contains("RevelTheme") && out.contains("prefers-color-scheme"));
        assert_eq!(wire(page, false).matches("<script>").count(), 0);
        let plain = "<html><head></head><body></body></html>".to_string();
        assert_eq!(wire(plain.clone(), true), plain);
    }
}
