//! Styles propres aux outils de Revel (D-151) : ce qu'un outil exige pour fonctionner (piste d'un
//! carrousel qui défile de côté), écrit par Revel dans la page qui l'emploie, sur le site comme dans
//! le canevas, avec une priorité nulle (`:where`) : les styles du site, réglés dans le panneau,
//! passent toujours devant. L'utilisateur n'a jamais à les écrire.

/// (attribut qui signale l'outil, règles).
const TOOLS: &[(&str, &str)] = &[
    // Élément montré dans un seul thème (D-168) ; l'interrupteur de thème réécrit ces requêtes.
    (
        " data-revel-scheme-only",
        "@media (prefers-color-scheme: dark){[data-revel-scheme-only=light]{display:none!important}}\
@media not (prefers-color-scheme: dark){[data-revel-scheme-only=dark]{display:none!important}}",
    ),
    // Horloge : chiffres de même largeur (l'heure ne bouge pas à chaque seconde), sur une ligne.
    (
        " data-revel-clock",
        ":where([data-revel-clock]){font-variant-numeric:tabular-nums;white-space:nowrap}",
    ),
    (
        " data-revel-track",
        ":where([data-revel-track]){overflow-x:auto;scroll-snap-type:x mandatory;scrollbar-width:none;overscroll-behavior-x:contain}\
:where([data-revel-track])::-webkit-scrollbar{display:none}\
:where([data-revel-track]>*,[data-revel-track]>picture>img){scroll-snap-align:start}",
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
    // Sommaire (D-172) : liens les uns sous les autres, filet à gauche, partie lue soulignée.
    (
        "toc",
        "B ol{list-style:none;margin:0;padding:0;display:flex;flex-direction:column}\
B a{display:block;padding:8px 0 8px 14px;border-left:2px solid color-mix(in srgb,currentColor 20%,transparent);\
color:inherit;text-decoration:none;opacity:.7}B a.is-active{border-left-color:currentColor;opacity:1;font-weight:600}",
    ),
    // Modèle 3D (D-160) : forme réglée dans Styles (proportions par point de rupture) ; boutons
    // (parties) en bas à droite ; compteur (`data-model-stats`, icônes) en bas à gauche ; calques
    // (traits, noms des bandes, nom de la vue montrée seule) ; plein écran : tout l'écran.
    (
        "model",
        "B{position:relative;display:block;width:100%;aspect-ratio:4/3;overflow:hidden}\
B:fullscreen{width:100%;height:100%;aspect-ratio:auto}\
B [data-revel-part=controls]{position:absolute;right:12px;bottom:12px;z-index:1;display:flex;gap:6px}\
B [data-revel-part=button]{display:grid;place-items:center;width:36px;height:36px;padding:0;border:0;border-radius:8px;\
background-color:color-mix(in srgb,currentColor 12%,transparent);color:inherit;cursor:pointer}\
B [data-revel-part=button].is-active{background-color:color-mix(in srgb,currentColor 30%,transparent)}\
B [data-revel-part=button] svg{width:18px;height:18px}\
B [data-revel-part=views]{position:absolute;top:12px;left:12px;z-index:1;display:flex;flex-wrap:wrap;gap:4px}\
B [data-revel-part=views] [data-revel-part=button]{width:auto;height:28px;padding:0 10px;font:inherit;font-size:12px}\
B [data-rv-stats]{position:absolute;left:12px;bottom:12px;z-index:1;display:flex;align-items:center;gap:14px;height:36px;\
font-size:12px;line-height:1;font-variant-numeric:tabular-nums;opacity:.75}\
B [data-rv-stat]{display:flex;align-items:center;gap:6px}\
B [data-rv-layers]{position:absolute;inset:0;z-index:1;overflow:hidden;pointer-events:none;font-size:12px;line-height:1;letter-spacing:.02em}\
B [data-rv-layer-line]{position:absolute;top:0;left:0;width:1px;background-color:currentColor;opacity:.35;transform-origin:0 0}\
B [data-rv-layer]{position:absolute;top:0;left:0;margin:0;padding:8px 14px 8px 0;border:0;background:none;color:inherit;font:inherit;\
letter-spacing:inherit;white-space:nowrap;cursor:pointer;pointer-events:auto;opacity:.65;transform-origin:0 0;transition:opacity .15s}\
B [data-rv-layer]:hover,B [data-rv-layer].is-hover,B [data-rv-layer]:focus-visible{opacity:1}\
B [data-rv-layer-name]{position:absolute;top:14px;left:50%;transform:translateX(-50%);display:grid;place-items:center;height:32px;\
padding:0 12px;border-radius:8px;background-color:color-mix(in srgb,currentColor 12%,transparent);white-space:nowrap;opacity:0;\
transition:opacity .2s}\
B [data-rv-layer-name].is-on{opacity:1}",
    ),
    (
        "mview",
        "B{position:relative;display:block;width:100%;aspect-ratio:4/3;overflow:hidden}",
    ),
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
        // `data-revel-own` (D-178) : outil repris d'un site importé, qui garde l'allure donnée
        // par ses classes ; l'allure de Revel ne s'ajoute pas.
        "B:not([data-revel-own]) [data-revel-part=tablist]{display:flex;flex-wrap:wrap;gap:4px;\
border-bottom:1px solid color-mix(in srgb,currentColor 30%,transparent)}\
B [data-revel-part=tab]{cursor:pointer}\
B:not([data-revel-own]) [data-revel-part=tab]{padding:10px 16px;border:0;border-bottom:2px solid transparent;margin-bottom:-1px;\
background:none;color:inherit;font:inherit}B:not([data-revel-own]) [data-revel-part=tab].is-active{border-bottom-color:currentColor}\
B:not([data-revel-own])>[data-revel-panel]{padding:16px 0}",
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
        "B{display:flex;align-items:center;gap:48px;overflow:hidden}B>*{flex:none;white-space:nowrap}\
B>img,B>picture>img{width:auto;height:48px;min-width:96px;object-fit:contain}",
    ),
    (
        "carousel",
        // D-174 : largeur des diapositives selon le nombre montré (`--rv-show`), écart, proportions
        // des images ; commandes dessous, dessus ou sur les côtés ; mosaïque ; compteur écrit par
        // un compteur CSS tant que le script ne l'a pas rempli (canevas).
        "B{position:relative;display:flex;flex-direction:column;gap:12px;counter-reset:rv-slide;--rv-gap:12px;--rv-show:1}\
B [data-revel-part=track]{display:flex;gap:var(--rv-gap)}\
B [data-revel-part=track]>*{flex:none;min-width:0;counter-increment:rv-slide}\
B [data-revel-part=track]>*,B [data-revel-part=track]>picture>img{width:calc((100% - var(--rv-gap) * (var(--rv-show) - 1)) / var(--rv-show))}\
B [data-revel-part=track]>img,B [data-revel-part=track]>picture>img{height:auto;aspect-ratio:var(--rv-ratio,4 / 3);object-fit:cover}\
B [data-revel-part=track]>*,B [data-revel-part=track]>picture>img,B .rv-more-tile{border-radius:var(--rv-radius,0)}\
B:not([data-revel-own]) [data-revel-part=track]>div{min-height:240px}\
B [data-revel-part=controls]{display:flex;align-items:center;justify-content:flex-end;gap:12px}\
B [data-revel-part=title]{order:-2;margin:0 auto 0 0}\
B [data-revel-part=count]{order:-1;margin:0;font-variant-numeric:tabular-nums}\
B [data-revel-part=count]:empty::before{content:\"1 / \" counter(rv-slide)}\
B [data-revel-part=arrow]{width:44px;height:44px;flex:none;display:grid;place-items:center;padding:0;border:1px solid currentColor;border-radius:50%;background:none;\
color:inherit;font:inherit;cursor:pointer}B [data-revel-part=arrow]:disabled{opacity:.35;cursor:default}\
B [data-revel-part=arrow] svg{width:18px;height:18px}\
B [data-revel-part=dots]{display:flex;gap:8px}B [data-revel-part=dots] button{width:8px;height:8px;padding:0;border:0;\
border-radius:50%;background:currentColor;opacity:.3;cursor:pointer}B [data-revel-part=dots] button.is-active{opacity:1}\
B[data-revel-controls=top] [data-revel-part=controls]{order:-1}\
B[data-revel-controls=sides] [data-revel-part=controls]{position:absolute;inset:0;justify-content:space-between;padding:0 12px;pointer-events:none}\
B[data-revel-controls=sides] [data-revel-part=arrow]{pointer-events:auto;background-color:color-mix(in srgb,Canvas 70%,transparent)}\
B[data-revel-controls=sides] [data-revel-part=title]{display:none}\
B[data-revel-controls=sides] [data-revel-part=count],B[data-revel-controls=sides] [data-revel-part=dots]{position:absolute;left:50%;bottom:12px;transform:translateX(-50%)}\
B[data-revel-layout=mosaic] [data-revel-part=track]{display:grid;grid-template-columns:repeat(var(--rv-cols,3),minmax(0,1fr))}\
B[data-revel-layout=mosaic] [data-revel-part=track]>*,B[data-revel-layout=mosaic] [data-revel-part=track]>picture>img{width:100%;aspect-ratio:var(--rv-ratio,1)}\
B[data-revel-layout=mosaic] [data-revel-part=controls]{display:none}\
B .rv-more-tile{aspect-ratio:var(--rv-ratio,1);display:grid;place-items:center;padding:0;border:1px solid color-mix(in srgb,currentColor 20%,transparent);\
background:color-mix(in srgb,currentColor 8%,transparent);color:inherit;font:inherit;font-weight:600;font-size:1.25em;cursor:pointer}\
B [hidden]{display:none}",
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
        "accordion-group",
        // D-176 : volets `details`, titre avec icône d'ouverture (à droite, ou à gauche avec
        // `data-revel-icon="start"`), qui tourne de `--rv-turn` quand le volet est ouvert.
        "B:not([data-revel-own])>details{border-bottom:1px solid color-mix(in srgb,currentColor 30%,transparent)}\
B [data-revel-part=summary]{display:flex;align-items:center;justify-content:space-between;gap:16px;\
cursor:pointer;list-style:none}B:not([data-revel-own]) [data-revel-part=summary]{padding:14px 0;font-weight:600}\
B [data-revel-part=summary]::-webkit-details-marker{display:none}\
B [data-revel-part=summary] svg{width:18px;height:18px;flex:none;transition:transform .2s}\
B details[open]>[data-revel-part=summary] svg{transform:rotate(var(--rv-turn,180deg))}\
B[data-revel-icon=start] [data-revel-part=summary]{flex-direction:row-reverse;justify-content:flex-end}\
B:not([data-revel-own]) details>:not([data-revel-part]){padding-bottom:14px}B details>:not([data-revel-part]):empty{min-height:24px}\
B [hidden]{display:none}",
    ),
    (
        "navbar",
        // Menu replié derrière son bouton sur téléphone (767 px) ; `data-revel-collapse` :
        // `tablet` (991 px), `small` (479 px), `always`, `never` (D-176).
        "B{position:relative;display:flex;align-items:center;justify-content:space-between;gap:24px}\
B:not([data-revel-own]){padding:16px 24px}B:not([data-revel-own])>a{font-weight:700}\
B>a{color:inherit;text-decoration:none}\
B [data-revel-part=menu]{display:flex;align-items:center;gap:24px}B [data-revel-part=menu] a{color:inherit;text-decoration:none}\
B [data-revel-part=toggle]{display:none;padding:8px 12px;border:1px solid currentColor;border-radius:6px;background:none;\
color:inherit;font:inherit;cursor:pointer}\
B [data-revel-part=toggle]:has(svg){padding:6px;border:0}B [data-revel-part=toggle] svg{display:block;width:24px;height:24px}\
B>a img{display:block;height:32px;width:auto}\
@media (max-width:991px){B[data-revel-collapse=tablet] [data-revel-part=toggle]{display:block}B[data-revel-collapse=tablet] [data-revel-part=menu]{display:none;position:absolute;top:100%;left:0;right:0;flex-direction:column;align-items:flex-start;gap:16px;padding:16px 24px 24px;background-color:inherit}B[data-revel-collapse=tablet].is-open [data-revel-part=menu]{display:flex}}\
@media (max-width:767px){B:not([data-revel-collapse]) [data-revel-part=toggle]{display:block}B:not([data-revel-collapse]) [data-revel-part=menu]{display:none;position:absolute;top:100%;left:0;right:0;flex-direction:column;align-items:flex-start;gap:16px;padding:16px 24px 24px;background-color:inherit}B:not([data-revel-collapse]).is-open [data-revel-part=menu]{display:flex}}\
@media (max-width:479px){B[data-revel-collapse=small] [data-revel-part=toggle]{display:block}B[data-revel-collapse=small] [data-revel-part=menu]{display:none;position:absolute;top:100%;left:0;right:0;flex-direction:column;align-items:flex-start;gap:16px;padding:16px 24px 24px;background-color:inherit}B[data-revel-collapse=small].is-open [data-revel-part=menu]{display:flex}}\
B[data-revel-collapse=always] [data-revel-part=toggle]{display:block}B[data-revel-collapse=always] [data-revel-part=menu]{display:none;position:absolute;top:100%;left:0;right:0;flex-direction:column;align-items:flex-start;gap:16px;padding:16px 24px 24px;background-color:inherit}B[data-revel-collapse=always].is-open [data-revel-part=menu]{display:flex}",
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
            // Partie ajoutée (`::before`, `::after`, `::-webkit-…`) : hors de `:where()`, qui ne
            // l'accepte pas.
            let (sel, pseudo) = match sel.rfind("::") {
                Some(i) if !sel[i..].contains(',') => (sel[..i].to_string(), sel[i..].to_string()),
                _ => (sel, String::new()),
            };
            out.push_str(&format!(":where({sel}){pseudo}{}", &rule[open..]));
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
        assert!(css.contains("@media (max-width:767px){:where([data-revel-base=\"navbar\"]:not([data-revel-collapse]) [data-revel-part=toggle]){display:block}"));
        assert!(css.contains(
            ":where([data-revel-base=\"navbar\"]:not([data-revel-collapse]).is-open [data-revel-part=menu]){display:flex}}"
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
