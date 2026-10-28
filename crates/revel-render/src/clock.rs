//! Horloge et date (D-158) : `data-revel-clock` sur un texte, réglé dans ses réglages ou ajouté
//! par le panneau Ajouter ; le texte écrit dans la page est remplacé par l'heure ou la date du
//! visiteur, dans la langue de la page, chaque seconde. Comme le composant « Time & Date » de
//! Framer, repris à l'import.
//!
//! Options (mots séparés par des espaces) : `time` (par défaut), `date` ou `both` ; `seconds`,
//! `12h`, `nominutes` pour l'heure ; `weekday`, `year`, `month=long|short|numeric|none` pour la
//! date.

const SCRIPT: &str = concat!(
    "<script>(function(w,d){function opt(e){var o=(e.getAttribute(\"data-revel-clock\")||\"\").split(/\\s+/),r={};",
    "o.forEach(function(x){var i=x.indexOf(\"=\");if(i>0)r[x.slice(0,i)]=x.slice(i+1);else if(x)r[x]=true});return r}",
    "function text(o,now,lang){var t={hour:\"numeric\",minute:o.nominutes?undefined:\"2-digit\",second:o.seconds&&!o.nominutes?\"2-digit\":undefined,hour12:!!o[\"12h\"]},",
    "m=o.month||\"long\",dt={weekday:o.weekday?\"long\":undefined,day:\"numeric\",month:m===\"none\"?undefined:m,year:o.year?\"numeric\":undefined};",
    "try{if(o.date)return new Intl.DateTimeFormat(lang,dt).format(now);if(o.both)return new Intl.DateTimeFormat(lang,Object.assign({},dt,t)).format(now);",
    "return new Intl.DateTimeFormat(lang,t).format(now)}catch(e){return now.toLocaleTimeString()}}",
    "function all(){var lang=d.documentElement.lang||undefined,now=new Date();[].forEach.call(d.querySelectorAll(\"[data-revel-clock]\"),function(e){",
    "var v=text(opt(e),now,lang);if(e.textContent!==v)e.textContent=v})}",
    "function tick(){all();setTimeout(tick,1000-Date.now()%1000+5)}",
    "if(d.readyState===\"loading\")d.addEventListener(\"DOMContentLoaded\",tick);else tick()})(window,document)</script>",
);

/// La page contient une horloge.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-clock")
}

/// Page (site, aperçu ou canevas) : script ajouté avant `</head>` si elle a une horloge.
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
    fn horloge() {
        let page = "<html><head></head><body><p data-revel-clock=\"time seconds\">14:29:00</p></body></html>".to_string();
        let out = wire(page.clone());
        assert!(
            out.contains("Intl.DateTimeFormat")
                && out.find("<script>").unwrap() < out.find("</head>").unwrap()
        );
        let none = "<html><head></head><body><p>a</p></body></html>".to_string();
        assert_eq!(wire(none.clone()), none);
    }
}
