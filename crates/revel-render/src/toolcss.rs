//! Styles propres aux outils de Revel (D-151) : ce qu'un outil exige pour fonctionner (piste d'un
//! carrousel qui défile de côté), écrit par Revel dans la page qui l'emploie, sur le site comme dans
//! le canevas, avec une priorité nulle (`:where`) : les styles du site, réglés dans le panneau,
//! passent toujours devant. L'utilisateur n'a jamais à les écrire.

/// (attribut qui signale l'outil, règles).
const TOOLS: &[(&str, &str)] = &[
    // Horloge : chiffres de même largeur (l'heure ne bouge pas à chaque seconde), sur une ligne.
    (
        " data-revel-clock",
        ":where([data-revel-clock]){font-variant-numeric:tabular-nums;white-space:nowrap}",
    ),
    (
        " data-revel-track",
        ":where([data-revel-track]){overflow-x:auto;scroll-snap-type:x mandatory;scrollbar-width:none;overscroll-behavior-x:contain}\
:where([data-revel-track])::-webkit-scrollbar{display:none}\
:where([data-revel-track]>*){scroll-snap-align:start}",
    ),
];

/// Allure propre des éléments ajoutés (D-159), par nature (`data-revel-base`) : ce que l'élément
/// montre tant que l'utilisateur ne lui a pas donné ses classes. Couleurs tirées du texte
/// (`currentColor`) : l'élément suit les couleurs du site, clair ou sombre.
const BASES: &[(&str, &str)] = &[
    (
        "grid",
        "B{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px}",
    ),
    (
        "columns",
        "B{display:flex;flex-wrap:wrap;gap:24px}B>*{flex:1 1 240px;min-width:0}",
    ),
    (
        "button",
        "B{display:inline-flex;align-items:center;justify-content:center;gap:8px;padding:12px 22px;\
border:1px solid currentColor;border-radius:999px;background:none;color:inherit;font:inherit;font-weight:600;\
text-decoration:none;cursor:pointer}",
    ),
    ("video", "B{display:block;width:100%;height:auto}"),
    (
        "pixels",
        "B{position:relative;width:100%;min-height:320px;overflow:hidden}",
    ),
    (
        "cart-add",
        "B{display:inline-flex;align-items:center;padding:12px 22px;border:1px solid currentColor;\
border-radius:999px;background:none;color:inherit;font:inherit;font-weight:600;cursor:pointer}B.is-added{opacity:.7}",
    ),
    (
        "hover-media",
        "B{display:block;width:100%;aspect-ratio:16/9;object-fit:cover}",
    ),
    ("lottie", "B{display:block;width:100%;max-width:480px}"),
    (
        "collection",
        "B{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:24px}",
    ),
    (
        "reveal",
        "B{position:relative;display:block;overflow:hidden}B>img{display:block;width:100%;height:auto}\
B>img:last-child:not(:first-child){position:absolute;inset:0;height:100%;object-fit:cover;opacity:0;transition:opacity .35s ease}\
B:hover>img:last-child:not(:first-child){opacity:1}",
    ),
    (
        "progress",
        "B{position:relative;width:100%;height:4px;overflow:hidden;border-radius:2px;\
background-color:color-mix(in srgb,currentColor 20%,transparent)}\
B [data-revel-part=fill]{width:0%;height:100%;border-radius:inherit;background-color:currentColor;\
transition:width .45s cubic-bezier(.33,1,.68,1)}",
    ),
    (
        "tabs",
        "B [data-revel-part=tablist]{display:flex;flex-wrap:wrap;gap:4px;\
border-bottom:1px solid color-mix(in srgb,currentColor 30%,transparent)}\
B [data-revel-part=tab]{padding:10px 16px;border:0;border-bottom:2px solid transparent;margin-bottom:-1px;\
background:none;color:inherit;font:inherit;cursor:pointer}B [data-revel-part=tab].is-active{border-bottom-color:currentColor}\
B>[data-revel-panel]{padding:16px 0}",
    ),
    (
        "accordion",
        "B{border-bottom:1px solid color-mix(in srgb,currentColor 30%,transparent)}\
B>[data-revel-part=summary]{padding:14px 0;font-weight:600;cursor:pointer}B>:not([data-revel-part]){padding-bottom:14px}",
    ),
    (
        "switch",
        "B{display:inline-flex;align-items:center;gap:10px;padding:6px 6px 6px 14px;border:1px solid currentColor;\
border-radius:999px;background:none;color:inherit;font:inherit;cursor:pointer}\
B [data-revel-part=state]{min-width:48px;padding:2px 10px;border-radius:999px;text-align:center;\
background-color:color-mix(in srgb,currentColor 18%,transparent)}B[aria-pressed=false] [data-revel-part=state]{opacity:.55}",
    ),
    (
        "consent",
        "B{position:fixed;right:16px;bottom:16px;left:16px;z-index:60;max-width:480px;margin-left:auto;padding:20px;\
border-radius:12px;background-color:#212328;color:#fff}B p{margin:0}\
B [data-revel-part=actions]{display:flex;gap:8px;margin-top:12px}\
B [data-revel-consent-choice]{padding:8px 16px;border:1px solid currentColor;border-radius:999px;background:none;\
color:inherit;font:inherit;cursor:pointer}B [data-revel-consent-choice=accept]{border-color:#fff;background-color:#fff;color:#111}",
    ),
    (
        "filters",
        "B{display:flex;flex-wrap:wrap;gap:8px}\
B [data-revel-part=filter]{padding:8px 16px;border:1px solid currentColor;border-radius:999px;background:none;\
color:inherit;font:inherit;cursor:pointer}B [data-revel-part=filter].is-active{background-color:color-mix(in srgb,currentColor 15%,transparent)}",
    ),
    (
        "video-player",
        "B{position:relative;margin:0;aspect-ratio:16/9;overflow:hidden;background-color:#000}\
B [data-revel-part=cover]{display:block;position:relative;width:100%;height:100%;padding:0;border:0;background:none;\
color:#fff;cursor:pointer}B [data-revel-part=cover] img{display:block;width:100%;height:100%;object-fit:cover}\
B [data-revel-part=play]{position:absolute;top:50%;left:50%;display:grid;place-items:center;width:72px;height:72px;\
border-radius:50%;background-color:rgba(0,0,0,.6);transform:translate(-50%,-50%)}B [data-revel-part=play] svg{width:32px;height:32px}",
    ),
    (
        "ticker",
        "B{display:flex;align-items:center;gap:48px;overflow:hidden}B>*{flex:none;white-space:nowrap}",
    ),
    (
        "carousel",
        "B{position:relative}B [data-revel-part=track]{display:flex;gap:16px}\
B [data-revel-part=track]>*{flex:0 0 min(80%,420px);min-height:240px;padding:24px;\
background-color:color-mix(in srgb,currentColor 8%,transparent)}\
B [data-revel-part=controls]{display:flex;align-items:center;gap:12px;margin-top:12px}\
B [data-revel-part=arrow]{width:40px;height:40px;border:1px solid currentColor;border-radius:50%;background:none;\
color:inherit;font:inherit;cursor:pointer}B [data-revel-part=arrow]:disabled{opacity:.35;cursor:default}",
    ),
    (
        "slideshow",
        "B{position:relative}B [data-revel-part=track]>*{flex:0 0 100%;min-height:240px;padding:24px;\
background-color:color-mix(in srgb,currentColor 8%,transparent)}\
B [data-revel-part=controls]{display:flex;align-items:center;justify-content:center;gap:12px;margin-top:12px}\
B [data-revel-part=arrow]{width:40px;height:40px;border:1px solid currentColor;border-radius:50%;background:none;\
color:inherit;font:inherit;cursor:pointer}B [data-revel-part=arrow]:disabled{opacity:.35;cursor:default}\
B [data-revel-part=dots]{display:flex;gap:8px}B [data-revel-part=dots] button{width:8px;height:8px;padding:0;border:0;\
border-radius:50%;background-color:currentColor;opacity:.3;cursor:pointer}B [data-revel-part=dots] button.is-active{opacity:1}",
    ),
    (
        "navbar",
        "B{position:relative;display:flex;align-items:center;justify-content:space-between;gap:24px;padding:16px 24px}\
B>a{font-weight:700;color:inherit;text-decoration:none}\
B [data-revel-part=menu]{display:flex;align-items:center;gap:24px}B [data-revel-part=menu] a{color:inherit;text-decoration:none}\
B [data-revel-part=toggle]{display:none;padding:8px 12px;border:1px solid currentColor;border-radius:6px;background:none;\
color:inherit;font:inherit;cursor:pointer}\
@media (max-width:767px){B [data-revel-part=toggle]{display:block}B [data-revel-part=menu]{display:none;position:absolute;\
top:100%;left:0;right:0;flex-direction:column;align-items:flex-start;gap:16px;padding:16px 24px 24px;background-color:inherit}\
B.is-open [data-revel-part=menu]{display:flex}}",
    ),
    (
        "locales",
        "B{display:flex;align-items:center;gap:12px}B [data-revel-part=link]{color:inherit;text-decoration:none;opacity:.6}\
B [data-revel-part=link][aria-current=page]{opacity:1;font-weight:600}",
    ),
];

/// Règles d'une nature d'élément : chaque sélecteur (séparé par `}`) mis à priorité nulle.
fn base_css(kind: &str, rules: &str) -> String {
    let root = format!("[data-revel-base=\"{kind}\"]");
    let mut out = String::new();
    // `@media (…){…}` : sélecteurs intérieurs traités de même.
    let mut rest = rules;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("@media") {
            let open = r.find('{').unwrap_or(0);
            let cond = &r[..open];
            // Fin du bloc : l'accolade qui ferme `@media` (deux accolades de suite).
            let body_end = r.find("}}").map(|i| i + 1).unwrap_or(r.len());
            let inner = &r[open + 1..body_end];
            out.push_str(&format!("@media{cond}{{{}}}", base_css(kind, inner)));
            rest = r.get(body_end + 1..).unwrap_or("");
            continue;
        }
        let Some(end) = rest.find('}') else { break };
        let rule = &rest[..=end];
        if let Some(open) = rule.find('{') {
            let sel = rule[..open].replace('B', &root);
            out.push_str(&format!(":where({sel}){}", &rule[open..]));
        }
        rest = &rest[end + 1..];
    }
    out
}

/// Règles des outils employés par la page.
fn css(html: &str) -> String {
    let mut out: String = TOOLS
        .iter()
        .filter(|(attr, _)| html.contains(attr))
        .map(|(_, css)| *css)
        .collect();
    for (kind, rules) in BASES {
        if html.contains(&format!("data-revel-base=\"{kind}\"")) {
            out.push_str(&base_css(kind, rules));
        }
    }
    out
}

/// Feuille ajoutée au début de `<head>` (avant celles du site).
pub fn wire(html: String) -> String {
    let rules = css(&html);
    put(html, rules)
}

fn put(html: String, rules: String) -> String {
    if rules.is_empty() {
        return html;
    }
    let tag = format!("<style data-revel-tools>{rules}</style>");
    // `<head>` peut porter des attributs (repères du canevas).
    let open = html
        .find("<head>")
        .or_else(|| html.find("<head "))
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1));
    match open {
        Some(i) => format!("{}{tag}{}", &html[..i], &html[i..]),
        None => html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_des_outils() {
        let page = "<html><head><link rel=\"stylesheet\" href=\"/s.css\"></head><body><div data-revel-track>x</div></body></html>".to_string();
        let out = wire(page);
        let s = out.find("data-revel-tools").unwrap();
        assert!(s < out.find("/s.css").unwrap() && out.contains(":where([data-revel-track])"));
        let plain = "<html><head></head><body></body></html>".to_string();
        assert_eq!(wire(plain.clone()), plain);
        let canvas = wire(
            "<html><head data-rv=\"h\"></head><body><div data-revel-track></div></body></html>"
                .into(),
        );
        assert!(canvas.starts_with("<html><head data-rv=\"h\"><style data-revel-tools>"));
    }

    #[test]
    fn allure_propre_des_elements_ajoutes() {
        let css = base_css(
            "navbar",
            BASES.iter().find(|(k, _)| *k == "navbar").unwrap().1,
        );
        assert!(css.starts_with(":where([data-revel-base=\"navbar\"]){position:relative"));
        assert!(css.contains("@media (max-width:767px){:where([data-revel-base=\"navbar\"] [data-revel-part=toggle]){display:block}"));
        assert!(css.contains(
            ":where([data-revel-base=\"navbar\"].is-open [data-revel-part=menu]){display:flex}}"
        ));
        // Chaque nature : des règles bien formées, rien hors de `:where`.
        for (kind, rules) in BASES {
            let c = base_css(kind, rules);
            assert_eq!(c.matches('{').count(), c.matches('}').count(), "{kind}");
            assert!(
                !c.contains("B{") && !c.contains("B ") && !c.contains("B>"),
                "{kind}"
            );
        }
        let page = "<html><head></head><body><div data-revel-base=\"grid\">x</div></body></html>"
            .to_string();
        assert!(wire(page).contains(":where([data-revel-base=\"grid\"]){display:grid"));
    }
}
